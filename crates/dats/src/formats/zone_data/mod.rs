pub mod collision_mesh;
pub mod zone_model;

use anyhow::{anyhow, Result};
use common::{
    byte_walker::ByteWalker, vec_byte_walker::VecByteWalker, writing_byte_walker::WritingByteWalker,
};
use encoding::chunk_key_tables::KEY_TABLE_1;
use serde_derive::{Deserialize, Serialize};

use crate::{dat_format::DatFormat, serde_hex};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZoneData {
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chunk {
    pub four_char_code: String,
    pub chunk_type: u8,
    pub unknown_0x08: u32,
    pub unknown_0x12: u32,
    #[serde(with = "serde_hex")]
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ChunkData {
    _05 {
        #[serde(with = "serde_hex")]
        data: Vec<u8>,
    },
    _1B {
        #[serde(with = "serde_hex")]
        data: Vec<u8>,
    },
    _FFFF {
        #[serde(with = "serde_hex")]
        data: Vec<u8>,
    },
    Plain {
        #[serde(with = "serde_hex")]
        data: Vec<u8>,
    },
}

impl ChunkData {
    pub fn get_data(&self) -> &[u8] {
        match self {
            ChunkData::_05 { data } => data,
            ChunkData::_1B { data } => data,
            ChunkData::_FFFF { data } => data,
            ChunkData::Plain { data } => data,
        }
    }
}

impl ZoneData {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ZoneData> {
        let mut chunks = vec![];

        while walker.remaining() > 0 {
            let chunk = Chunk::parse(walker)?;
            chunks.push(chunk);
        }

        Ok(ZoneData { chunks })
    }
}

impl Chunk {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<Chunk> {
        let four_char_code = std::str::from_utf8(walker.take_bytes(4)?)?.to_string();

        let type_and_length = walker.step::<u32>()? & 0xFFFFFFF;
        let unknown_0x08 = walker.step::<u32>()?;
        let unknown_0x12 = walker.step::<u32>()?;

        let chunk_type = (type_and_length & 0x7F) as u8;
        let length = ((type_and_length >> 7) << 4) - 0x10;

        let data = ChunkData::parse(walker, length)?;

        Ok(Chunk {
            four_char_code,
            chunk_type,
            unknown_0x08,
            unknown_0x12,
            data,
        })
    }
}

impl ChunkData {
    pub fn parse<T: ByteWalker>(walker: &mut T, length: u32) -> Result<Vec<u8>> {
        let data = walker.take_bytes(length as usize)?;

        let chunk_data = if length < 8 {
            data.into()
        } else if data[3] == 0x05 {
            // TODO: decrypt
            data.into()
        } else if data[3] == 0x1B {
            Self::decrypt_1b(data.to_vec())?
        } else if data[6] == 0xFF && data[7] == 0xFF {
            // TODO: decrypt
            data.into()
        } else {
            // TODO: unknown
            data.into()
        };

        Ok(chunk_data)
    }

    fn decrypt_1b(data: Vec<u8>) -> Result<Vec<u8>> {
        let mut walker = VecByteWalker::on(data);
        let len = walker.read_at::<u32>(0)? & 0xFFFFFF;

        let index = walker.read_at::<u8>(7)? ^ 0xFF;
        let mut key = KEY_TABLE_1[index as usize] as usize;

        let mut key_counter = 0;
        let mut pos = 8;
        while pos < len {
            let xor_len = ((key >> 4) & 7) + 0x10;

            if key & 1 == 1 && pos + (xor_len as u32) < len {
                for i in 0..xor_len {
                    let idx = (pos + i as u32) as usize;
                    let new_value = walker.read_at::<u8>(idx)? ^ 0xFF;
                    walker.write_at::<u8>(idx, new_value);
                }
            }
            key_counter += 1;
            key += key_counter;
            pos += xor_len as u32;
        }

        let node_count = (walker.read_at::<u32>(4)? & 0xFFFFFF) as usize;

        for i in 0..node_count {
            for j in 0..0x10 {
                let idx = (0x20 + i * 0x64 + j) as usize;
                let new_value = walker.read_at::<u8>(idx)? ^ 0x55;
                walker.write_at::<u8>(idx, new_value);
            }
        }

        Ok(walker.into_vec())
    }
}

impl DatFormat for ZoneData {
    fn write<T: WritingByteWalker>(&self, _walker: &mut T) -> Result<()> {
        Err(anyhow!("Can't write DAT"))
    }

    fn from<T: ByteWalker>(walker: &mut T) -> Result<Self> {
        ZoneData::parse(walker)
    }

    fn check_type<T: ByteWalker>(_walker: &mut T) -> Result<()> {
        Err(anyhow!("Can't check type"))
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufWriter, path::PathBuf};

    use crate::dat_format::DatFormat;

    use super::ZoneData;

    #[test]
    pub fn zone_data() {
        let mut dat_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dat_path.push("resources/test/zone_data_Pashhow_Marshlands.DAT");

        ZoneData::check_path(&dat_path).unwrap();
        let res = ZoneData::from_path(&dat_path).unwrap();

        let file = File::create("Pashhow_Marshlands.yml").unwrap();
        serde_yaml::to_writer(BufWriter::new(file), &res).unwrap();
    }
}
