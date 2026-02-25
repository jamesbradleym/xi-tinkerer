/// FFXI Event DAT parser and serializer.
///
/// Event DATs contain per-zone cutscene/event bytecode executed by the client's
/// internal virtual machine. Each DAT is structured as:
///
///   eventheader_t  – block count + block sizes
///   eventblock_t[] – one per NPC/entity, containing:
///       ActorNumber   – entity server ID (0x7FFFFFFF = player/zone events)
///       TagOffset[]   – byte offsets where each event's data begins within EventData
///       EventExecNum[] – event IDs corresponding to each TagOffset
///       ImmedData[]   – reference table of 32-bit values (item IDs, dialog IDs, etc.)
///       EventData     – bytecode blob, 4-byte aligned
///
/// Within EventData, each event series (delimited by TagOffset boundaries) contains
/// VM opcodes (0x00–0xD9) followed by an optional **inline data section**. The data
/// section holds reference tables and string data that opcodes access at runtime via
/// `getworkofs_` / `getworkstrofs_` (reading 2-byte little-endian values where the
/// high byte 0x80 indicates an ImmedData/References lookup). The VM never executes
/// the data section — it is only read by parameter-fetching helpers.
///
/// Because the TagOffset boundaries encompass both code and data, the parser cannot
/// assume the entire range is opcodes. Instead, it parses opcodes greedily and stores
/// any trailing unparseable bytes as a `data_section` (see `ParsedData::OpcodesWithData`).
///
/// Opcode definitions are based on atom0s's clean-room reverse engineering of the
/// FFXI client binary (FFXiMain.dll). Each opcode's size and sub-opcode dispatch
/// is documented in the XiEvents repository.
///
/// References:
///   - DAT structure: https://github.com/atom0s/XiEvents/blob/main/Event%20DAT%20Structures.md
///   - VM functions:  https://github.com/atom0s/XiEvents/blob/main/Event%20VM%20Functions.md
///   - Opcodes:       https://github.com/atom0s/XiEvents/tree/main/OpCodes

use std::collections::HashMap;
use std::fmt;

use anyhow::{anyhow, Result};
use serde::de::{Deserializer, Error as DeError, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use common::{byte_walker::ByteWalker, writing_byte_walker::WritingByteWalker};
use crate::dat_format::DatFormat;
use crate::formats::opcode_descriptions;
use crate::formats::opcode_descriptions::DESCRIPTIONS;

/// 0xFF is used by the client as padding/alignment; it is never a valid opcode.
const INVALID_OPCODE: u8 = 0xFF;

/// The documented opcode range from atom0s's reverse engineering of the client.
const _VALID_OPCODE_RANGE: std::ops::RangeInclusive<u8> = 0x00..=0xD9;

/// Represents the header of the event file.
/// Contains metadata about the blocks, including the number of blocks and their sizes.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventHeader {
    pub block_count: u32,        // Number of event blocks in the file.
    pub block_sizes: Vec<u32>,   // Sizes of each block in bytes.
}

/// Represents the entire event file.
/// Combines the header and all blocks into one structure.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Events {
    pub header: EventHeader,       // The header containing block metadata.
    pub blocks: Vec<EventBlock>,   // A list of all event blocks in the file.
}

/// A single event block containing all event data for one entity.
///
/// Maps to `eventblock_t` in the DAT. Each block belongs to one NPC/entity
/// (identified by `actor_number`) and contains one or more event series
/// (cutscenes, menus, dialog sequences) encoded as VM bytecode.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventBlock {
    /// Resolved entity name from zone's entity_names DAT (export-only, not stored in binary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_name: Option<String>,
    /// Entity server ID. `0x7FFFFFFF` represents player/zone-wide events.
    pub actor_number: u32,
    pub tag_count: u32,
    /// Byte offsets into EventData where each event series begins.
    pub tag_offsets: Vec<u16>,
    /// Event IDs corresponding to each TagOffset entry.
    pub event_exec_nums: Vec<u16>,
    pub immed_count: u32,
    /// Immediate data / References table. Contains 32-bit values (item IDs,
    /// dialog IDs, quantities, etc.) that opcodes look up at runtime via
    /// `getworkofs_` when the encoded parameter has bit 15 set (0x80XX pattern).
    pub immed_data: Vec<ImmedValue>,
    /// Parsed event series from the EventData bytecode blob.
    pub event_series: Vec<EventSeries>,
}

/// A single immediate data value. Serializes as scalar when unannotated,
/// or as `{ value, comment }` when the value matches a known item ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImmedValue {
    Simple(u32),
    Annotated { value: u32, comment: String },
}

impl ImmedValue {
    pub fn to_u32(&self) -> u32 {
        match self {
            ImmedValue::Simple(v) => *v,
            ImmedValue::Annotated { value, .. } => *value,
        }
    }
}

impl Serialize for ImmedValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            ImmedValue::Simple(v) => serializer.serialize_u32(*v),
            ImmedValue::Annotated { value, comment } => {
                use serde::ser::SerializeStruct;
                let mut s = serializer.serialize_struct("ImmedValue", 2)?;
                s.serialize_field("value", value)?;
                s.serialize_field("comment", comment)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for ImmedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ImmedValueDe {
            Simple(u32),
            Annotated { value: u32, comment: String },
        }
        match ImmedValueDe::deserialize(deserializer)? {
            ImmedValueDe::Simple(v) => Ok(ImmedValue::Simple(v)),
            ImmedValueDe::Annotated { value, comment } => Ok(ImmedValue::Annotated { value, comment }),
        }
    }
}

/// One event within a block, identified by its event ID and containing
/// parsed bytecode (and optionally a trailing data section).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventSeries {
    /// Event ID from EventExecNum table (e.g., dialog/cutscene number).
    pub id: u16,
    pub parsed_data: ParsedData,
}

/// Parsed content of an event series within EventData.
///
/// Each event series (bounded by consecutive TagOffset entries) can contain:
///
/// 1. **Pure opcodes** (`Opcodes`) – the entire range is valid VM bytecode.
///
/// 2. **Opcodes + data section** (`OpcodesWithData`) – VM bytecode followed by
///    an inline data region. The data region typically contains:
///    - Reference tables: sequential `XX 80` byte pairs encoding
///      `getworkofs_` lookups into the ImmedData/References array
///      (value `0x80XX` has bit 15 set → `References[4 * (val & 0x7FFF)]`).
///    - String data: null-terminated, fixed-width entity/NPC name strings
///      read by `getworkstrofs_`.
///    The VM accesses this data at absolute byte offsets within the series;
///    it is never executed as opcodes.
///
/// 3. **Raw bytes** (`RawBytes`) – fallback when no opcodes could be parsed
///    at all (e.g., empty series or entirely unrecognized data).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ParsedData {
    Opcodes(Vec<EventOpcode>),
    OpcodesWithData {
        opcodes: Vec<EventOpcode>,
        #[serde(
            serialize_with = "serialize_vec_u8_as_hex",
            deserialize_with = "deserialize_vec_u8_from_hex"
        )]
        data_section: Vec<u8>,
    },
    #[serde(
        serialize_with = "serialize_vec_u8_as_hex",
        deserialize_with = "deserialize_vec_u8_from_hex"
    )]
    RawBytes(Vec<u8>),
}

/// Represents an individual opcode within an event series.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventOpcode {
    #[serde(
        serialize_with = "serialize_u8_as_hex",
        deserialize_with = "deserialize_u8_from_hex"
    )]
    pub opcode: u8,
    #[serde(
        serialize_with = "serialize_vec_u8_as_hex",
        deserialize_with = "deserialize_vec_u8_from_hex"
    )]
    pub params: Vec<u8>,
    pub description: Option<String>,
    pub url: Option<String>,
}

/// Represents metadata for an opcode.
#[derive(Debug)]
pub struct OpcodeMetadata {
    pub description: &'static str, // Static string for descriptions
    pub url: String,               // URL for documentation
    pub sizes: Vec<usize>,         // Sizes for opcode parameters
    pub callback: Option<opcode_descriptions::OpcodeSizeCallback>,
}

#[derive(Debug, Clone)]
pub struct ParseFailure {
    pub offset: usize,
    pub opcode_byte: u8,
    pub sub_opcode: Option<u8>,
    pub reason: ParseFailureReason,
    pub preceding_opcodes: Vec<u8>,
    pub preceding_details: Vec<(u8, usize, usize)>,
}

#[derive(Debug, Clone)]
pub enum ParseFailureReason {
    InvalidOpcode,
    NoMetadata,
    CallbackReturnedNone,
    MultiSizesNoCallback,
    ParamsExceededBounds,
}

impl EventHeader {
    fn get_header_values<T: ByteWalker>(walker: &mut T) -> Result<(u32, Vec<u32>)> {
        let block_count = walker.step::<u32>()?;

        if block_count == 0 {
            return Err(anyhow!("Event file contains no blocks."));
        }

        let mut block_sizes = Vec::new();
        for _ in 0..block_count {
            block_sizes.push(walker.step::<u32>()?);
        }

        Ok((block_count, block_sizes))
    }

    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let block_count = walker.step::<u32>()?;
        if block_count == 0 || block_count > 1024 {
            return Err(anyhow!("Invalid BlockCount: {}", block_count));
        }

        let block_sizes: Vec<u32> = (0..block_count)
            .map(|_| walker.step::<u32>())
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            block_count,
            block_sizes,
        })
    }
}

impl EventBlock {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let actor_number = walker.step::<u32>()?;
        let tag_count = walker.step::<u32>()?;
        let tag_offsets: Vec<u16> = (0..tag_count)
            .map(|_| walker.step::<u16>())
            .collect::<Result<Vec<_>>>()?;
        let event_exec_nums: Vec<u16> = (0..tag_count)
            .map(|_| walker.step::<u16>())
            .collect::<Result<Vec<_>>>()?;
        let immed_count = walker.step::<u32>()?;
        let immed_data: Vec<ImmedValue> = (0..immed_count)
            .map(|_| walker.step::<u32>().map(ImmedValue::Simple))
            .collect::<Result<Vec<_>>>()?;
        let event_data_size = walker.step::<u32>()?;
        if event_data_size == 0 || event_data_size > walker.remaining() as u32 {
            return Err(anyhow!("Invalid EventDataSize: {}", event_data_size));
        }

        let event_series = Self::parse_series(walker, &tag_offsets, &event_exec_nums, event_data_size, walker.offset())?;

        let padding = (4 - (event_data_size as usize % 4)) % 4;
        if padding > 0 {
            walker.skip(padding);
        }

        Ok(Self {
            actor_name: None,
            actor_number,
            tag_count,
            tag_offsets,
            event_exec_nums,
            immed_count,
            immed_data,
            event_series,
        })
    }

    /// Splits EventData into individual event series using TagOffset boundaries.
    ///
    /// Each series spans from `TagOffset[i]` to `TagOffset[i+1]` (or `EventDataSize`
    /// for the last entry). The byte range includes both executable opcodes and any
    /// trailing inline data section that the opcodes reference.
    fn parse_series(walker: &mut impl ByteWalker, offsets: &[u16], event_exec_nums: &[u16], event_data_size: u32, event_data_start: usize) -> Result<Vec<EventSeries>> {
        let mut series_list = Vec::new();
        let offset_len = offsets.len();

        for (i, &_relative_offset) in offsets.iter().enumerate() {
            let start_offset = walker.offset(); // Absolute start position

            let end_offset = if i + 1 < offset_len {
                event_data_start + offsets[i + 1] as usize
            } else {
                event_data_start + event_data_size as usize
            };

            // Handle empty or invalid offsets
            if start_offset >= end_offset {
                series_list.push(EventSeries {
                    id: event_exec_nums[i],
                    parsed_data: ParsedData::RawBytes(Vec::new()),
                });
                continue;
            }

            if start_offset > end_offset {
                return Err(anyhow!(
                    "Invalid opcode boundaries: start=0x{:04X}, end=0x{:04X}",
                    start_offset,
                    end_offset
                ));
            }
            let parsed_data = Self::parse_opcodes(walker, end_offset)?;
            series_list.push(EventSeries {
                id: event_exec_nums[i],
                parsed_data,
            });
        }

        Ok(series_list)
    }

    /// Parses VM bytecode from the walker up to `end_offset`, returning the result
    /// as one of the `ParsedData` variants.
    ///
    /// Opcodes are parsed greedily: each byte is looked up in the opcode metadata
    /// table (0x00–0xD9), its size is determined (fixed, callback-based, or via
    /// lookahead disambiguation), and its parameter bytes are consumed.
    ///
    /// When parsing fails (unknown byte, callback returns None, or params exceed
    /// bounds), any remaining bytes from the failure point to `end_offset` are
    /// stored as a `data_section`. This handles the common case where event series
    /// contain executable opcodes followed by inline data (reference tables,
    /// string data) that the VM reads via `getworkofs_` but never executes.
    fn parse_opcodes(walker: &mut impl ByteWalker, end_offset: usize) -> Result<ParsedData> {
        let mut opcodes = Vec::new();
        let metadata_map = opcode_metadata();

        while walker.offset() < end_offset {
            let opcode_start = walker.offset();
            let opcode = walker.step::<u8>()?;

            let failed = if opcode == INVALID_OPCODE || walker.offset() > end_offset {
                true
            } else if let Some(meta) = metadata_map.get(&opcode) {
                let params_length = if meta.sizes.len() == 1 {
                    Some(meta.sizes[0])
                } else if let Some(callback) = meta.callback {
                    let data_slice = walker.read_bytes_at(walker.offset(), end_offset - walker.offset()).unwrap_or_default();
                    let prev: Vec<opcode_descriptions::EventOpcode> = opcodes
                        .iter()
                        .map(|o: &EventOpcode| opcode_descriptions::EventOpcode { opcode: o.opcode, params: o.params.clone() })
                        .collect();
                    match callback(opcode, data_slice, &prev) {
                        Some(size) => Some(size),
                        None => Self::check_opcode_size_by_validity(
                            &meta.sizes, walker.offset(), walker, end_offset,
                        )?,
                    }
                } else {
                    Self::check_opcode_size_by_validity(
                        &meta.sizes, walker.offset(), walker, end_offset,
                    )?
                };

                match params_length {
                    Some(size) => {
                        let adjusted = size.saturating_sub(1);
                        if adjusted > 0 && adjusted + walker.offset() > end_offset {
                            true
                        } else {
                            let params = if adjusted > 0 {
                                walker.take_bytes(adjusted)?.to_vec()
                            } else {
                                Vec::new()
                            };
                            opcodes.push(EventOpcode {
                                opcode,
                                params,
                                description: Some(meta.description.to_string()),
                                url: Some(meta.url.clone()),
                            });
                            false
                        }
                    }
                    None => true,
                }
            } else {
                true
            };

            if failed {
                walker.goto(opcode_start as u32);
                let data_section = walker.take_bytes(end_offset - opcode_start)?.to_vec();

                if opcodes.is_empty() {
                    let mut all_bytes = Vec::new();
                    for op in &opcodes {
                        all_bytes.push(op.opcode);
                        all_bytes.extend(&op.params);
                    }
                    all_bytes.extend(&data_section);
                    return Ok(ParsedData::RawBytes(all_bytes));
                }

                return Ok(ParsedData::OpcodesWithData {
                    opcodes,
                    data_section,
                });
            }
        }

        Ok(ParsedData::Opcodes(opcodes))
    }

    /// Checks each candidate length to see if it leads to a valid next opcode.
    /// Accepts 0x00..=0xFE as valid (0xFF is reserved as padding/invalid).
    /// If **exactly one** candidate passes the test, returns Some(length).
    /// Otherwise returns None (0 or multiple matches).
    fn check_opcode_size_by_validity(
        candidate_sizes: &[usize],
        start_offset: usize,
        walker: &mut impl ByteWalker,
        end_offset: usize,
    ) -> Result<Option<usize>> {
        let metadata_map = opcode_metadata();
        let mut valid_candidates = Vec::new();

        for &length in candidate_sizes {
            let next_opcode_offset = start_offset + length;
            if next_opcode_offset <= end_offset {
                let valid = if next_opcode_offset < end_offset {
                    let next_byte = walker.read_at::<u8>(next_opcode_offset)?;
                    next_byte != INVALID_OPCODE && metadata_map.contains_key(&next_byte)
                } else {
                    true // Fits exactly at end, no next byte to validate
                };
                if valid {
                    valid_candidates.push(length);
                }
            }
        }

        if valid_candidates.len() == 1 {
            Ok(Some(valid_candidates[0]))
        } else {
            Ok(None)
        }
    }


    /// Re-parses a raw byte slice using the same logic as `parse_opcodes` but returns
    /// a structured `ParseFailure` at the first point of failure instead of falling back.
    pub fn diagnose_raw_bytes(data: &[u8]) -> Option<ParseFailure> {
        let metadata_map = opcode_metadata();
        let mut offset = 0;
        let mut parsed_opcode_bytes: Vec<u8> = Vec::new();
        let mut preceding_details: Vec<(u8, usize, usize)> = Vec::new();
        let mut dummy_opcodes: Vec<EventOpcode> = Vec::new();

        let mk_failure = |offset: usize, opcode_byte: u8, sub_opcode: Option<u8>,
                          reason: ParseFailureReason, preceding_opcodes: Vec<u8>,
                          preceding_details: Vec<(u8, usize, usize)>| -> ParseFailure {
            ParseFailure { offset, opcode_byte, sub_opcode, reason, preceding_opcodes, preceding_details }
        };

        while offset < data.len() {
            let opcode = data[offset];

            if opcode == INVALID_OPCODE {
                return Some(mk_failure(offset, opcode, None, ParseFailureReason::InvalidOpcode, parsed_opcode_bytes, preceding_details));
            }

            let meta = match metadata_map.get(&opcode) {
                Some(m) => m,
                None => return Some(mk_failure(offset, opcode, data.get(offset + 1).copied(), ParseFailureReason::NoMetadata, parsed_opcode_bytes, preceding_details)),
            };

            let size = if meta.sizes.len() == 1 {
                meta.sizes[0]
            } else if let Some(callback) = meta.callback {
                let remaining = &data[offset + 1..];
                let prev: Vec<opcode_descriptions::EventOpcode> = dummy_opcodes
                    .iter()
                    .map(|o| opcode_descriptions::EventOpcode { opcode: o.opcode, params: o.params.clone() })
                    .collect();
                match callback(opcode, remaining, &prev) {
                    Some(s) => s,
                    None => return Some(mk_failure(offset, opcode, data.get(offset + 1).copied(), ParseFailureReason::CallbackReturnedNone, parsed_opcode_bytes, preceding_details)),
                }
            } else {
                return Some(mk_failure(offset, opcode, data.get(offset + 1).copied(), ParseFailureReason::MultiSizesNoCallback, parsed_opcode_bytes, preceding_details));
            };

            if offset + size > data.len() {
                return Some(mk_failure(offset, opcode, data.get(offset + 1).copied(), ParseFailureReason::ParamsExceededBounds, parsed_opcode_bytes, preceding_details));
            }

            let params = if size > 1 { data[offset + 1..offset + size].to_vec() } else { Vec::new() };
            dummy_opcodes.push(EventOpcode { opcode, params, description: None, url: None });
            parsed_opcode_bytes.push(opcode);
            preceding_details.push((opcode, offset, size));
            offset += size;
        }

        None
    }

    /// For a failure at `failure_offset` in `data`, tries all sizes 1..=max_size for the
    /// failing opcode and returns which sizes lead to a valid parse of the remaining bytes.
    pub fn try_alternative_sizes(data: &[u8], failure_offset: usize, max_size: usize) -> Vec<usize> {
        let mut valid_sizes = Vec::new();

        for candidate_size in 1..=max_size {
            let next_offset = failure_offset + candidate_size;
            if next_offset > data.len() {
                break;
            }
            if next_offset == data.len() {
                valid_sizes.push(candidate_size);
                continue;
            }
            if Self::can_parse_remainder(&data[next_offset..]) {
                valid_sizes.push(candidate_size);
            }
        }

        valid_sizes
    }

    /// Checks whether a byte slice can be fully parsed as a valid opcode stream.
    fn can_parse_remainder(data: &[u8]) -> bool {
        let metadata_map = opcode_metadata();
        let mut offset = 0;
        let mut dummy_opcodes: Vec<EventOpcode> = Vec::new();

        while offset < data.len() {
            let opcode = data[offset];
            if opcode == INVALID_OPCODE {
                return false;
            }

            let metadata = match metadata_map.get(&opcode) {
                Some(m) => m,
                None => return false,
            };

            let size = if metadata.sizes.len() == 1 {
                metadata.sizes[0]
            } else if let Some(callback) = metadata.callback {
                let remaining = &data[offset + 1..];
                let prev: Vec<opcode_descriptions::EventOpcode> = dummy_opcodes
                    .iter()
                    .map(|o| opcode_descriptions::EventOpcode { opcode: o.opcode, params: o.params.clone() })
                    .collect();
                match callback(opcode, remaining, &prev) {
                    Some(s) => s,
                    None => return false,
                }
            } else {
                return false;
            };

            if offset + size > data.len() {
                return false;
            }

            let params = if size > 1 { data[offset + 1..offset + size].to_vec() } else { Vec::new() };
            dummy_opcodes.push(EventOpcode {
                opcode,
                params,
                description: None,
                url: None,
            });
            offset += size;
        }

        true
    }

    pub fn write_to_walker<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write(self.actor_number);
        walker.write(self.tag_count);
        for offset in &self.tag_offsets {
            walker.write(*offset);
        }
        for exec_num in &self.event_exec_nums {
            walker.write(*exec_num);
        }
        walker.write(self.immed_count);
        for immed in &self.immed_data {
            walker.write(immed.to_u32());
        }

        let event_data_size = self.calculate_event_data_size();
        walker.write(event_data_size);

        self.write_event_data(walker, event_data_size)
    }

    fn calculate_event_data_size(&self) -> u32 {
        self.event_series
            .iter()
            .map(|series| match &series.parsed_data {
                ParsedData::Opcodes(opcodes) => {
                    opcodes.iter().map(|opcode| 1 + opcode.params.len() as u32).sum::<u32>()
                }
                ParsedData::OpcodesWithData { opcodes, data_section } => {
                    let opcodes_size: u32 = opcodes.iter().map(|opcode| 1 + opcode.params.len() as u32).sum();
                    opcodes_size + data_section.len() as u32
                }
                ParsedData::RawBytes(bytes) => bytes.len() as u32,
            })
            .sum::<u32>()
    }

    /// Write Event Data.
    fn write_event_data<T: WritingByteWalker>(&self, walker: &mut T, event_data_size: u32) -> Result<()> {
        for (_i, series) in self.event_series.iter().enumerate() {
            match &series.parsed_data {
                ParsedData::Opcodes(opcodes) => {
                    for opcode in opcodes {
                        walker.write(opcode.opcode);
                        walker.write_bytes(&opcode.params);
                    }
                }
                ParsedData::OpcodesWithData { opcodes, data_section } => {
                    for opcode in opcodes {
                        walker.write(opcode.opcode);
                        walker.write_bytes(&opcode.params);
                    }
                    walker.write_bytes(data_section);
                }
                ParsedData::RawBytes(bytes) => {
                    walker.write_bytes(bytes);
                }
            }
        }

        // Align to a 4-byte boundary
        let padding = (4 - (event_data_size % 4)) % 4;
        if padding > 0 {
            walker.write_bytes(&vec![0xFF; padding as usize]);
        }

        Ok(())
    }
}

impl Events {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        let header = EventHeader::parse(walker)?;
        let blocks = (0..header.block_count)
            .map(|_| EventBlock::parse(walker))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { header, blocks })
    }

    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write(self.header.block_count);
        for size in &self.header.block_sizes {
            walker.write(*size);
        }
        for block in &self.blocks {
            block.write_to_walker(walker)?;
        }
        Ok(())
    }
}

pub fn opcode_metadata() -> HashMap<u8, OpcodeMetadata> {
    let mut metadata = HashMap::new();
    for &(i, description, ref sizes, callback) in DESCRIPTIONS.iter() {
        let url = format!("https://github.com/atom0s/XiEvents/blob/main/OpCodes/0x00{:02X}.md", i);
        metadata.insert(
            i,
            OpcodeMetadata {
                description,
                url,
                sizes: sizes.to_vec(),
                callback,
            },
        );
    }
    metadata
}

impl DatFormat for Events {
    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Events::parse(walker)
    }

    fn check_type<T: ByteWalker>(walker: &mut T) -> Result<()> {
        EventHeader::get_header_values(walker)?;
        Ok(())
    }
}

/// Serialize a u8 as a hexadecimal string
pub fn serialize_u8_as_hex<S>(value: &u8, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("0x{:02X}", value))
}

/// Deserialize a hexadecimal string into a u8
pub fn deserialize_u8_from_hex<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?; // Deserialize into a string
    u8::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| DeError::custom(format!("Invalid hex string: {} ({})", s, e)))
}

/// Serialize a Vec<u8> as an array of hexadecimal strings
pub fn serialize_vec_u8_as_hex<S>(values: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Start a sequence serializer
    let mut seq = serializer.serialize_seq(Some(values.len()))?;

    // Format each byte as a hex string and serialize it
    for value in values {
        seq.serialize_element(&format!("0x{:02X}", value))?;
    }

    // End the sequence serialization
    seq.end()
}

/// Deserialize an array of hexadecimal strings into a Vec<u8>
pub fn deserialize_vec_u8_from_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct HexVecVisitor;

    impl<'de> Visitor<'de> for HexVecVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array of hexadecimal strings like ['0x01', '0x80']")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(hex_str) = seq.next_element::<String>()? {
                let value = u8::from_str_radix(hex_str.trim_start_matches("0x"), 16)
                    .map_err(|e| DeError::custom(format!("Invalid hex string: {} ({})", hex_str, e)))?;
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(HexVecVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn windurst_woods_from_dat() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/event_windurst_woods.DAT");

        Events::check_path(&dat_path).unwrap();

        let res = Events::from_path_checked(&dat_path).unwrap();
        assert!(res.header.block_count > 0);
        print_event_parse_summary(&res);
        summarize_decoding_stats(&res);
    }

    #[test]
    pub fn binary_round_trip() {
        use common::vec_byte_walker::VecByteWalker;

        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dats = [
            "resources/test/event_windurst_woods.DAT",
            "resources/test/event_southern_sandoria.DAT",
        ];

        for rel_path in &test_dats {
            let dat_path = base.join(rel_path);
            if !dat_path.exists() {
                println!("Skipping {} (file not found)", rel_path);
                continue;
            }

            let original_bytes = std::fs::read(&dat_path).unwrap();
            let events = Events::from_path(&dat_path).unwrap();

            let mut output = VecByteWalker::new();
            events.write(&mut output).unwrap();
            let written_bytes = output.into_vec();

            assert_eq!(
                original_bytes.len(),
                written_bytes.len(),
                "Size mismatch for {}: original={}, written={}",
                rel_path, original_bytes.len(), written_bytes.len()
            );

            for (i, (a, b)) in original_bytes.iter().zip(written_bytes.iter()).enumerate() {
                assert_eq!(
                    a, b,
                    "Byte mismatch at offset {} in {}: original=0x{:02X}, written=0x{:02X}",
                    i, rel_path, a, b
                );
            }

            println!("{}: binary round-trip OK ({} bytes)", rel_path, original_bytes.len());
        }
    }

    #[test]
    pub fn yaml_round_trip() -> Result<()> {
        use common::vec_byte_walker::VecByteWalker;

        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dat_path = base.join("resources/test/event_windurst_woods.DAT");
        if !dat_path.exists() {
            println!("Skipping yaml_round_trip (file not found)");
            return Ok(());
        }

        let events = Events::from_path(&dat_path)?;
        let yaml = serde_yaml::to_string(&events)?;
        let parsed_back: Events = serde_yaml::from_str(&yaml)?;

        let mut original_output = VecByteWalker::new();
        events.write(&mut original_output)?;

        let mut roundtrip_output = VecByteWalker::new();
        parsed_back.write(&mut roundtrip_output)?;

        assert_eq!(
            original_output.into_vec(),
            roundtrip_output.into_vec(),
            "DAT -> YAML -> DAT round-trip produced different binary output"
        );

        println!("yaml_round_trip: OK");
        Ok(())
    }

    fn print_event_parse_summary(event: &Events) {
        println!("Event file summary:");
        println!("  Block count: {}", event.header.block_count);

        for (block_index, block) in event.blocks.iter().enumerate() {
            println!("  Block #{}:", block_index);

            for (series_index, series) in block.event_series.iter().enumerate() {
                match &series.parsed_data {
                    ParsedData::Opcodes(opcodes) => {
                        println!(
                            "    Series #{} (ExecNum = {}): {} opcodes",
                            series_index, series.id, opcodes.len()
                        );
                    }
                    ParsedData::OpcodesWithData { opcodes, data_section } => {
                        println!(
                            "    Series #{} (ExecNum = {}): {} opcodes + {} bytes data section",
                            series_index, series.id, opcodes.len(), data_section.len()
                        );
                    }
                    ParsedData::RawBytes(bytes) => {
                        println!(
                            "    Series #{} (ExecNum = {}): RAW BYTES fallback (length = {})",
                            series_index, series.id, bytes.len()
                        );
                    }
                }
            }
        }
    }

    fn summarize_decoding_stats(event: &Events) {
        let mut total_series = 0;
        let mut series_opcodes = 0;
        let mut series_with_data = 0;
        let mut series_raw = 0;
        let mut total_opcodes = 0;
        let mut total_data_bytes = 0;
        let mut total_raw_bytes = 0;

        for block in &event.blocks {
            for series in &block.event_series {
                total_series += 1;
                match &series.parsed_data {
                    ParsedData::Opcodes(opcodes) => {
                        series_opcodes += 1;
                        total_opcodes += opcodes.len();
                    }
                    ParsedData::OpcodesWithData { opcodes, data_section } => {
                        series_with_data += 1;
                        total_opcodes += opcodes.len();
                        total_data_bytes += data_section.len();
                    }
                    ParsedData::RawBytes(bytes) => {
                        series_raw += 1;
                        total_raw_bytes += bytes.len();
                    }
                }
            }
        }

        println!("Decoding Stats:");
        println!("  Total EventSeries: {}", total_series);
        println!("  Decoded as opcodes only: {}", series_opcodes);
        println!("  Decoded as opcodes + data: {}", series_with_data);
        println!("  Fallback to raw bytes: {}", series_raw);
        println!("  Total opcodes parsed: {}", total_opcodes);
        println!("  Total data section bytes: {}", total_data_bytes);
        println!("  Total raw bytes: {}", total_raw_bytes);
    }

    #[derive(Debug)]
    struct FailureEntry {
        zone: String,
        actor_number: u32,
        series_id: u16,
        failure: ParseFailure,
        alternative_sizes: Vec<usize>,
        raw_bytes_sample: Vec<u8>,
        raw_bytes_len: usize,
    }

    fn run_diagnostic_on_events(zone_name: &str, events: &Events) -> Vec<FailureEntry> {
        let mut failures = Vec::new();

        for block in &events.blocks {
            for series in &block.event_series {
                if let ParsedData::RawBytes(bytes) = &series.parsed_data {
                    if bytes.is_empty() {
                        continue;
                    }
                    if let Some(failure) = EventBlock::diagnose_raw_bytes(bytes) {
                        let alt_sizes = EventBlock::try_alternative_sizes(bytes, failure.offset, 30);
                        let dump_start = failure.offset.saturating_sub(20);
                        let dump_end = (failure.offset + 20).min(bytes.len());
                        let raw_sample = bytes[dump_start..dump_end].to_vec();
                        failures.push(FailureEntry {
                            zone: zone_name.to_string(),
                            actor_number: block.actor_number,
                            series_id: series.id,
                            raw_bytes_len: bytes.len(),
                            failure,
                            alternative_sizes: alt_sizes,
                            raw_bytes_sample: raw_sample,
                        });
                    }
                }
            }
        }

        failures
    }

    fn print_diagnostic_report(all_failures: &[FailureEntry]) {
        if all_failures.is_empty() {
            println!("\n=== DIAGNOSTIC REPORT: No RawBytes failures found! ===");
            return;
        }

        println!("\n=== DIAGNOSTIC REPORT: {} total failures ===\n", all_failures.len());

        let mut by_opcode: std::collections::BTreeMap<(u8, Option<u8>, String), Vec<&FailureEntry>> =
            std::collections::BTreeMap::new();
        for entry in all_failures {
            let reason_str = format!("{:?}", entry.failure.reason);
            let key = (entry.failure.opcode_byte, entry.failure.sub_opcode, reason_str);
            by_opcode.entry(key).or_default().push(entry);
        }

        for ((opcode, sub_opcode, reason), entries) in &by_opcode {
            let sub_str = match sub_opcode {
                Some(s) => format!(" sub=0x{:02X}", s),
                None => String::new(),
            };
            println!(
                "--- Opcode 0x{:02X}{} | Reason: {} | Occurrences: {} ---",
                opcode, sub_str, reason, entries.len()
            );

            let metadata_map = opcode_metadata();
            if let Some(meta) = metadata_map.get(opcode) {
                println!("    Current sizes: {:?}", meta.sizes);
                println!("    Has callback: {}", meta.callback.is_some());
            } else {
                println!("    NO METADATA (opcode not in DESCRIPTIONS table)");
            }

            println!(
                "    atom0s doc: https://raw.githubusercontent.com/atom0s/XiEvents/main/OpCodes/0x00{:02X}.md",
                opcode
            );

            for entry in entries.iter().take(1) {
                println!(
                    "    Example: zone={}, actor=0x{:08X}, series={}, offset={}/{}, alt_sizes={:?}",
                    entry.zone,
                    entry.actor_number,
                    entry.series_id,
                    entry.failure.offset,
                    entry.raw_bytes_len,
                    entry.alternative_sizes,
                );

                let details = &entry.failure.preceding_details;
                let show_count = details.len().min(10);
                println!("    Last {} parsed opcodes before failure:", show_count);
                for &(op, off, sz) in details.iter().rev().take(show_count).collect::<Vec<_>>().iter().rev() {
                    println!("      offset={:>5}: opcode=0x{:02X}, size={}", off, op, sz);
                }

                println!("    Hex dump around failure (offset {}):", entry.failure.offset);
                let dump_start = entry.failure.offset.saturating_sub(20);
                let dump_end = (entry.failure.offset + 20).min(entry.raw_bytes_len);
                let hex: Vec<String> = entry.raw_bytes_sample[..dump_end - dump_start]
                    .iter()
                    .enumerate()
                    .map(|(i, b)| {
                        let abs_off = dump_start + i;
                        if abs_off == entry.failure.offset {
                            format!("[{:02X}]", b)
                        } else {
                            format!("{:02X}", b)
                        }
                    })
                    .collect();
                println!("      start={}:  {}", dump_start, hex.join(" "));
            }
            if entries.len() > 1 {
                println!("    ... and {} more occurrences", entries.len() - 1);
            }
            println!();
        }
    }

    #[test]
    pub fn diagnostic_scan() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dats = [
            ("Windurst_Woods", "resources/test/event_windurst_woods.DAT"),
            ("Southern_San_dOria", "resources/test/event_southern_sandoria.DAT"),
            ("Al_Zahbi", "resources/test/events_whitegate.DAT"),
        ];

        let mut all_failures = Vec::new();
        let mut total_series = 0usize;
        let mut total_raw = 0usize;

        for (zone_name, rel_path) in &test_dats {
            let dat_path = base.join(rel_path);
            if !dat_path.exists() {
                println!("Skipping {} (file not found)", rel_path);
                continue;
            }

            let events = match Events::from_path(&dat_path) {
                Ok(e) => e,
                Err(err) => {
                    println!("Error parsing {}: {}", zone_name, err);
                    continue;
                }
            };

            let mut zone_series = 0;
            let mut zone_raw = 0;
            let mut zone_with_data = 0;
            for block in &events.blocks {
                for series in &block.event_series {
                    zone_series += 1;
                    match &series.parsed_data {
                        ParsedData::RawBytes(b) if !b.is_empty() => zone_raw += 1,
                        ParsedData::OpcodesWithData { .. } => zone_with_data += 1,
                        _ => {}
                    }
                }
            }
            total_series += zone_series;
            total_raw += zone_raw;

            println!(
                "{}: {} series, {} with data sections, {} raw ({:.1}% parsed)",
                zone_name,
                zone_series,
                zone_with_data,
                zone_raw,
                (zone_series - zone_raw) as f64 / zone_series as f64 * 100.0
            );

            let failures = run_diagnostic_on_events(zone_name, &events);
            all_failures.extend(failures);
        }

        println!(
            "\nOverall: {} series, {} raw ({:.1}% success)",
            total_series,
            total_raw,
            if total_series > 0 {
                (total_series - total_raw) as f64 / total_series as f64 * 100.0
            } else {
                100.0
            }
        );

        print_diagnostic_report(&all_failures);
    }
}
