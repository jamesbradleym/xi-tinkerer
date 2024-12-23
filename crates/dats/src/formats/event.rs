/// Implementation based on ideas and guidance from atom0s
/// Source: https://github.com/atom0s/XiEvents/blob/main/Event%20DAT%20Structures.md
/// Credits to atom0s for valuable references and insights.
///

use std::collections::HashMap;
use std::fmt;

use anyhow::{anyhow, Result};
use serde::de::{Deserializer, Error as DeError, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use common::{byte_walker::ByteWalker, writing_byte_walker::WritingByteWalker};
use crate::dat_format::DatFormat;
use crate::formats::opcode_descriptions::DESCRIPTIONS;

const INVALID_OPCODE: u8 = 0xFF; // Use 0xFF to clearly indicate an invalid or empty opcode
const VALID_OPCODE_RANGE: std::ops::RangeInclusive<u8> = 0x00..=0xD9;

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
pub struct Event {
    pub header: EventHeader,       // The header containing block metadata.
    pub blocks: Vec<EventBlock>,   // A list of all event blocks in the file.
}

/// Represents a single event block.
/// Contains all event-related data for a specific entity or zone.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventBlock {
    pub actor_number: u32,        // Server ID of the entity. Special value `0x7FFFFFFF` represents player/zone events.
    pub tag_count: u32,           // Number of events contained in this block.
    pub tag_offsets: Vec<u16>,    // Offsets into the `event_data` table where each event starts.
    pub event_exec_nums: Vec<u16>, // IDs for each event in this block.
    pub immed_count: u32,         // Number of immediate data entries.
    pub immed_data: Vec<u32>,     // Immediate data table (e.g., references to item IDs, string IDs, etc.).
    pub event_series: Vec<EventSeries>, // Parsed opcodes from event_data
}

/// Represents a series of events.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventSeries {
    pub id: u16,
    pub parsed_data: ParsedData,
}

/// Represents the parsed data within an event series.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ParsedData {
    Opcodes(Vec<EventOpcode>),
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
    pub callback: Option<OpcodeSizeCallback>,   // Optional callback for dynamic size determination
}

pub type OpcodeSizeCallback = fn(opcode: u8, data: &[u8], previous_opcodes: &[EventOpcode]) -> Option<usize>;

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
        let immed_data: Vec<u32> = (0..immed_count)
            .map(|_| walker.step::<u32>())
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
            actor_number,
            tag_count,
            tag_offsets,
            event_exec_nums,
            immed_count,
            immed_data,
            event_series,
        })
    }

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

            // Parse opcodes within the series and group them together
            match Self::parse_opcodes(walker, end_offset)? {
                ParsedData::Opcodes(opcodes) => series_list.push(EventSeries {
                    id: event_exec_nums[i],
                    parsed_data: ParsedData::Opcodes(opcodes),
                }),
                ParsedData::RawBytes(bytes) => series_list.push(EventSeries {
                    id: event_exec_nums[i],
                    parsed_data: ParsedData::RawBytes(bytes),
                }),
            }
        }

        Ok(series_list)
    }

    fn parse_opcodes(walker: &mut impl ByteWalker, end_offset: usize) -> Result<ParsedData> {
        let mut opcodes = Vec::new();
        let start_offset = walker.offset();

        while walker.offset() < end_offset {
            // Read the opcode (1 byte)
            let opcode = walker.step::<u8>()?;
            if opcode == INVALID_OPCODE || walker.offset() > end_offset {
                // Bad opcode found
                eprintln!(
                    "Warning: No metadata available for opcode 0x{:02X}. Fallback to raw bytes.",
                    opcode
                );
                walker.goto(start_offset as u32);
                let remaining = walker.take_bytes(end_offset - start_offset)?.to_vec();
                return Ok(ParsedData::RawBytes(remaining));
            }

            // Retrieve opcode metadata
            let metadata_map = opcode_metadata();
            let metadata = metadata_map.get(&opcode);

            let description = metadata.map(|meta| meta.description.to_string());
            let url = metadata.map(|meta| meta.url.clone());

            // Determine parameter length
            let params_length = if let Some(meta) = metadata {
                if meta.sizes.len() == 1 {
                    // Single size
                    meta.sizes[0]
                } else if let Some(callback) = meta.callback {
                    let size_from_lookahead = Self::check_opcode_size_by_validity(
                                    &meta.sizes,
                                    start_offset,
                                    walker,
                                    end_offset,
                                )?;
                    if let Some(unique_size) = size_from_lookahead {
                        // We have a single, unambiguous size from lookahead
                        unique_size
                    } else {
                        // Use the callback
                        match callback(
                            opcode,
                            walker.read_bytes_at(walker.offset(), end_offset - walker.offset()).unwrap_or_default(),
                            &opcodes,
                        ) {
                            Some(size) => size,
                            None => {
                                eprintln!(
                                    "Warning: Failed to determine size dynamically for opcode 0x{:02X}. Fallback to raw bytes.",
                                    opcode
                                );
                                walker.goto(start_offset as u32);
                                let remaining = walker.take_bytes(end_offset - start_offset)?.to_vec();
                                return Ok(ParsedData::RawBytes(remaining));
                            }
                        }
                    }
                } else {
                    // Multiple sizes without callback
                    eprintln!(
                        "Warning: Opcode 0x{:02X} has multiple sizes, but no callback. Fallback to raw bytes.",
                        opcode
                    );
                    walker.goto(start_offset as u32); // Ensure we rewind to start
                    let remaining = walker.take_bytes(end_offset - start_offset)?.to_vec();
                    return Ok(ParsedData::RawBytes(remaining));
                }
            } else {
                // No metadata
                eprintln!(
                    "Warning: No metadata available for opcode 0x{:02X}. Fallback to raw bytes.",
                    opcode
                );

                walker.goto(start_offset as u32); // Ensure we rewind to start
                let remaining = walker.take_bytes(end_offset - start_offset)?.to_vec();
                return Ok(ParsedData::RawBytes(remaining));
            };

            // Adjust parameter length to exclude the opcode byte
            let adjusted_params_length = if params_length > 0 {
                params_length.saturating_sub(1) // Subtract 1 for the opcode
            } else {
                0
            };

            // Read parameters
            let params = if adjusted_params_length > 0 {
                if adjusted_params_length + walker.offset() > end_offset {
                    eprintln!(
                        "Warning: Opcode 0x{:02X} params exceeded expected size. Fallback to raw bytes.",
                        opcode
                    );
                    walker.goto(start_offset as u32); // Ensure we rewind to start
                    let remaining = walker.take_bytes(end_offset - start_offset)?.to_vec();
                    return Ok(ParsedData::RawBytes(remaining));
                }
                walker.take_bytes(adjusted_params_length)?.to_vec()
            } else {
                Vec::new()
            };

            // Add the opcode to the list
            opcodes.push(EventOpcode {
                opcode,
                params,
                description,
                url,
            });
        }

        Ok(ParsedData::Opcodes(opcodes))
    }

    /// Checks each candidate length to see if it leads to a valid next opcode (0x00..=0xD9).
    /// If **exactly one** candidate passes the test, returns Some(length).
    /// Otherwise returns None (0 or multiple matches).
    fn check_opcode_size_by_validity(
        candidate_sizes: &[usize],
        start_offset: usize,
        walker: &mut impl ByteWalker,
        end_offset: usize,
    ) -> Result<Option<usize>> {
        let mut valid_candidates = Vec::new();

        for &length in candidate_sizes {
            let next_offset = start_offset + length;
            // Must be strictly less than end_offset to read the next byte
            if next_offset < end_offset {
                // Peek the would-be next opcode
                let next_byte = walker.read_at::<u8>(next_offset)?;
                // let next_byte = walker.read_at(next_offset)?;
                // If it looks like a valid opcode, keep it
                if (0x00..=0xD9).contains(&next_byte) {
                    valid_candidates.push(length);
                }
            }
        }

        // If exactly 1 candidate remains, perfect
        if valid_candidates.len() == 1 {
            Ok(Some(valid_candidates[0]))
        } else {
            Ok(None)
        }
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
            walker.write(*immed);
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
                ParsedData::RawBytes(bytes) => bytes.len() as u32,
            })
            .sum::<u32>()
    }

    /// Write Event Data.
    fn write_event_data<T: WritingByteWalker>(&self, walker: &mut T, event_data_size: u32) -> Result<()> {
        for (i, series) in self.event_series.iter().enumerate() {
            match &series.parsed_data {
                ParsedData::Opcodes(opcodes) => {
                    for opcode in opcodes {
                        walker.write(opcode.opcode); // Write the opcode
                        walker.write_bytes(&opcode.params); // Write parameters
                    }
                }
                ParsedData::RawBytes(bytes) => {
                    walker.write_bytes(bytes); // Write raw bytes directly
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

impl Event {
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
        let url = format!("https://github.com/atom0s/XiEvents/blob/main/OpCodes/0x{:02X}.md", i);
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

impl DatFormat for Event {
    fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        self.write(walker)
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        Event::parse(walker)
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
    use std::{fs::File, io::BufReader, path::PathBuf};

    #[test]
    pub fn windurst_woods_from_dat() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // dat_path.push("resources/test/event_windurst_woods.DAT");
        dat_path.push("resources/test/event_southern_sandoria.DAT");

        // Check header validity
        Event::check_path(&dat_path).unwrap();

        // Parse the file and validate results
        let res = Event::from_path_checked(&dat_path).unwrap();
        assert!(res.header.block_count > 0);
        print_event_parse_summary(&res);
        summarize_decoding_stats(&res);
    }

    #[test]
    pub fn windurst_woods_from_yaml() -> Result<()> {
        let mut yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        yaml_path.push("resources/test/event_windurst_woods.yml");

        let raw_data_file = File::open(&yaml_path).map_err(|err| {
            anyhow!(
                "Could not open file at {}: {}",
                &yaml_path.display(),
                err
            )
        })?;

        let data: Event = serde_yaml::from_reader(BufReader::new(raw_data_file))
            .map_err(|err| anyhow!("Failed to parse YAML: {}", err))?;

        Ok(())
    }

    fn print_event_parse_summary(event: &Event) {
        println!("Event file summary:");
        println!("  Block count: {}", event.header.block_count);

        for (block_index, block) in event.blocks.iter().enumerate() {
            println!("  Block #{}:", block_index);

            // For each event series in this block
            for (series_index, series) in block.event_series.iter().enumerate() {
                match &series.parsed_data {
                    ParsedData::Opcodes(opcodes) => {
                        println!(
                            "    Series #{} (ExecNum = {}): {} opcodes",
                            series_index,
                            series.id,
                            opcodes.len()
                        );
                    }
                    ParsedData::RawBytes(bytes) => {
                        println!(
                            "    Series #{} (ExecNum = {}): RAW BYTES fallback (length = {})",
                            series_index,
                            series.id,
                            bytes.len()
                        );
                    }
                }
            }
        }
    }

    fn summarize_decoding_stats(event: &Event) {
        let mut total_series = 0;
        let mut series_opcodes = 0;
        let mut series_raw = 0;
        let mut total_opcodes = 0;
        let mut total_raw_bytes = 0;

        for block in &event.blocks {
            for series in &block.event_series {
                total_series += 1;
                match &series.parsed_data {
                    ParsedData::Opcodes(opcodes) => {
                        series_opcodes += 1;
                        total_opcodes += opcodes.len();
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
        println!("  Decoded as opcodes: {}", series_opcodes);
        println!("  Fallback to raw bytes: {}", series_raw);
        println!("  Total opcodes parsed: {}", total_opcodes);
        println!("  Total raw bytes: {}", total_raw_bytes);
    }
}
