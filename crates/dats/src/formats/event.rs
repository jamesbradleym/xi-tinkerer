use anyhow::{anyhow, Result};
use serde_derive::{Deserialize, Serialize};

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
    pub event_data_size: u32,     // Size of the raw event bytecode data.
    pub event_data: Vec<u8>,      // Raw event bytecode data (4-byte aligned).
}

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

        // Read EventData
        let mut event_data = walker.take_bytes(event_data_size as usize)?.to_vec();

        // Align EventData to a 4-byte boundary
        let padding = (4 - (event_data.len() % 4)) % 4;
        if padding > 0 {
            event_data.resize(event_data.len() + padding, 0xFF); // Add padding bytes
            walker.skip(padding);
        }

        Ok(Self {
            actor_number,
            tag_count,
            tag_offsets,
            event_exec_nums,
            immed_count,
            immed_data,
            event_data_size,
            event_data,
        })
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
        // Write the header
        walker.write(self.header.block_count);
        for (_i, size) in self.header.block_sizes.iter().enumerate() {
            walker.write(*size);
        }

        // Write each block
        for (_i, block) in self.blocks.iter().enumerate() {
            walker.write(block.actor_number);
            walker.write(block.tag_count);

            // Validate tag count before writing
            if block.tag_count == 0 || block.tag_count > 1024 {
                return Err(anyhow!("Invalid tag count: {}", block.tag_count));
            }

            for (_j, offset) in block.tag_offsets.iter().enumerate() {
                walker.write(*offset);
            }

            for (_j, exec_num) in block.event_exec_nums.iter().enumerate() {
                walker.write(*exec_num);
            }

            walker.write(block.immed_count);
            for (_j, immed) in block.immed_data.iter().enumerate() {
                walker.write(*immed);
            }

            walker.write(block.event_data_size);
            walker.write_bytes(&block.event_data);
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
