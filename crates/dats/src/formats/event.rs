/// Implementation based on ideas and guidance from atom0s
/// Source: https://github.com/atom0s/XiEvents/blob/main/Event%20DAT%20Structures.md
/// Credits to atom0s for valuable references and insights.

use std::fmt;

use anyhow::{anyhow, Result};
use serde::de::{Deserializer, Error as DeError, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

use common::{byte_walker::ByteWalker, writing_byte_walker::WritingByteWalker};
use crate::dat_format::DatFormat;

/// Represents the header of the event file.
/// Contains metadata about the blocks, including the number of blocks and their sizes.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventHeader {
    pub block_count: u32,        // Number of event blocks in the file.
    pub block_sizes: Vec<u32>,   // Sizes of each block in bytes.
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
    pub event_opcodes: Vec<EventOpcode>, // Parsed opcodes from event_data
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventOpcode {
    #[serde(
        serialize_with = "serialize_u8_as_hex",
        deserialize_with = "deserialize_u8_from_hex"
    )]
    pub opcode: u8, // Serialize/Deserialize the opcode as a hex string

    #[serde(
        serialize_with = "serialize_vec_u8_as_hex",
        deserialize_with = "deserialize_vec_u8_from_hex"
    )]
    pub params: Vec<u8>, // Serialize/Deserialize params as a list of hex strings
}

const INVALID_OPCODE: u8 = 0xFF; // Use 0xFF to clearly indicate an invalid or empty opcode
const VALID_OPCODE_RANGE: std::ops::RangeInclusive<u8> = 0x00..=0xD9;

/// Represents the entire event file.
/// Combines the header and all blocks into one structure.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub header: EventHeader,       // The header containing block metadata.
    pub blocks: Vec<EventBlock>,   // A list of all event blocks in the file.
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

    /// Parse the event header from the binary data.
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {

        // Read BlockCount
        let block_count = walker.step::<u32>()?; // Ensure correct endianness

        if block_count == 0 || block_count > 1024 {
            return Err(anyhow!("Invalid BlockCount: {}", block_count));
        }

        // Read BlockSizes
        let mut block_sizes = Vec::new();
        for i in 0..block_count {
            let size = walker.step::<u32>()?;
            if size == 0 {
                return Err(anyhow!("Invalid block size (0) at index {}", i));
            }
            block_sizes.push(size);
        }

        Ok(Self {
            block_count,
            block_sizes,
        })
    }
}

impl EventBlock {
    /// Parse a single event block from the binary data.
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        // ActorNumber
        let actor_number = walker.step::<u32>()?;

        // TagCount
        let tag_count = walker.step::<u32>()?;

        // TagOffsets
        let mut tag_offsets = Vec::new();
        for _i in 0..tag_count {
            let offset = walker.step::<u16>()?;
            tag_offsets.push(offset);
        }

        // EventExecNums
        let mut event_exec_nums = Vec::new();
        for _i in 0..tag_count {
            let exec_num = walker.step::<u16>()?;
            event_exec_nums.push(exec_num);
        }

        // ImmedCount
        let immed_count = walker.step::<u32>()?;

        // ImmedData
        let mut immed_data = Vec::new();
        for _i in 0..immed_count {
            let data = walker.step::<u32>()?;
            immed_data.push(data);
        }

        // EventDataSize
        let event_data_size = walker.step::<u32>()?;

        if event_data_size == 0 || event_data_size > walker.remaining() as u32 {
            return Err(anyhow!("Invalid EventDataSize: {}", event_data_size));
        }

        // Read EventData as opcodes
        // TODO: Add info on what these codes are doing for parsed yaml
        let event_opcodes = Self::parse_opcodes(walker, &tag_offsets, event_data_size, walker.offset())?;

        // Align EventData to a 4-byte boundary
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
            event_opcodes,
        })
    }

    fn parse_opcodes(walker: &mut impl ByteWalker, offsets: &[u16], event_data_size: u32, event_data_start: usize) -> Result<Vec<EventOpcode>> {
        let mut opcodes = Vec::new();

        for (i, &relative_offset) in offsets.iter().enumerate() {
            let start_offset = event_data_start + relative_offset as usize; // Absolute start position

            let end_offset = if i + 1 < offsets.len() {
                event_data_start + offsets[i + 1] as usize
            } else {
                event_data_start + event_data_size as usize
            };

            if start_offset == end_offset {
                opcodes.push(EventOpcode {
                    opcode: INVALID_OPCODE,
                    params: Vec::new(),
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

            // Seek to the starting offset if necessary
            if walker.offset() != start_offset {
                walker.goto_usize(start_offset);
            }

            // Read the opcode (1 byte) directly
            let opcode = walker.step::<u8>()?;

            // Calculate parameters length and read them
            let params_length = end_offset - start_offset - 1; // Subtract opcode size (1 byte)
            let params = walker.take_bytes(params_length)?.to_vec();

            opcodes.push(EventOpcode {
                opcode,
                params,
            });
        }

        Ok(opcodes)
    }

    fn calculate_event_data_size(&self) -> u32 {
        self.event_opcodes
            .iter()
            .filter(|opcode| !opcode.is_empty()) // Skip empty entries for event data size calc
            .map(|opcode| 1 + opcode.params.len() as u32) // 1 byte for opcode + params size
            .sum()
    }

    /// Write Event Data and return the padded size.
    fn write_event_data<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        for opcode in &self.event_opcodes {
            if opcode.is_empty() {
                continue;
            }

            walker.write(opcode.opcode); // Write the opcode
            walker.write_bytes(&opcode.params); // Write parameters
        }

        // Align to 4-byte boundary
        let total_size = self.calculate_event_data_size();
        let padding = (4 - (total_size as usize % 4)) % 4;

        if padding > 0 {
            walker.write_bytes(&vec![0xFF; padding]); // Write padding
        }

        Ok(())
    }
}

impl EventOpcode {
    /// Determine if the opcode is valid (within the defined range).
    pub fn is_valid(&self) -> bool {
        VALID_OPCODE_RANGE.contains(&self.opcode)
    }

    /// Check if the opcode is empty (invalid and has no parameters).
    pub fn is_empty(&self) -> bool {
        self.opcode == INVALID_OPCODE && self.params.is_empty()
    }
}

impl Event {
    /// Parse the entire event file from binary data.
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        // Parse the header
        let header = EventHeader::parse(walker)?;

        // Parse the blocks
        let mut blocks = Vec::new();
        for _i in 0..header.block_count {
            let block = EventBlock::parse(walker)?;
            blocks.push(block);
        }

        Ok(Self { header, blocks })
    }

    /// Write the event structure back to binary format.
    pub fn write<T: WritingByteWalker>(&self, walker: &mut T) -> Result<()> {
        walker.write(self.header.block_count);
        for size in &self.header.block_sizes {
            walker.write(*size);
        }

        for block in &self.blocks {
            walker.write(block.actor_number);

            walker.write(block.tag_count);

            for offset in &block.tag_offsets {
                walker.write(*offset);
            }

            for exec_num in &block.event_exec_nums {
                walker.write(*exec_num);
            }

            walker.write(block.immed_count);

            for immed in &block.immed_data {
                walker.write(*immed);
            }

            let event_data_size = block.calculate_event_data_size();
            walker.write(event_data_size);

            block.write_event_data(walker)?;
        }

        Ok(())
    }
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
    use std::path::PathBuf;

    #[test]
    pub fn windurst_woods() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/event_windurst_woods.DAT");

        // Check header validity
        Event::check_path(&dat_path).unwrap();

        // Parse the file and validate results
        let res = Event::from_path_checked(&dat_path).unwrap();
        assert!(res.header.block_count > 0);
    }
}
