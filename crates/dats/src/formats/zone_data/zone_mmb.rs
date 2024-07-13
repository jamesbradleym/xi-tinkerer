use anyhow::{anyhow, Result};
use common::byte_walker::ByteWalker;
use serde_derive::{Deserialize, Serialize};

use crate::serde_hex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmb {
    pub data_len: u32,
    pub top_header: ZoneMmbTopHeader,
    pub header: ZoneMmbHeader,
    pub models_list: Vec<Vec<ZoneMmbVertexIndices>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ZoneMmbTopHeader {
    Type1 {
        mmb_type: u8,
        next: u32,
        first_u32: u32,
        second_u32: u32,
        third_u32: u32,
    },
    Type2 {
        len: u32,
        d1: u8,
        d3: u8,
        d4: u8,
        d5: u8,
        d6: u8,
        unknown_0x08: u32,
        unknown_0x12: u32,
    },
}

impl ZoneMmbTopHeader {
    fn get_size(&self) -> u32 {
        match self {
            ZoneMmbTopHeader::Type1 { next, .. } => next * 16,
            ZoneMmbTopHeader::Type2 { len, .. } => len.clone(),
        }
    }

    fn has_d_values(&self) -> bool {
        match self {
            ZoneMmbTopHeader::Type1 { .. } => false,
            ZoneMmbTopHeader::Type2 { d3, .. } => *d3 == 2,
        }
    }

    fn has_triangle_list(&self) -> bool {
        match self {
            ZoneMmbTopHeader::Type1 { mmb_type, .. } => *mmb_type == 0,
            ZoneMmbTopHeader::Type2 { d3, .. } => *d3 == 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbHeader {
    img_id: [u8; 16],
    pieces: u32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    z1: f32,
    z2: f32,
    offset_block_header: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbBlockHeader {
    model_count: u32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    z1: f32,
    z2: f32,
    face_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbModelHeader {
    texture_name: [u8; 16],
    vertex_count: u16,
    blending: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GlDrawType {
    TriangleList,
    TriangleStrip,
    Line,
    LineLoop,
    Polygon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbBlockVertex {
    x: f32,
    y: f32,
    z: f32,
    dx: f32,
    dy: f32,
    dz: f32,
    hx: f32,
    hy: f32,
    hz: f32,
    color: u32,
    u: f32,
    v: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZoneMmbVertexIndices {
    draw_type: GlDrawType,
    texture_name: [u8; 16],
    vertex_count: u16,
    blending: u16,
    vertices: Vec<ZoneMmbBlockVertex>,
    index_count: u32,
    indices: Vec<u16>,
}

impl ZoneMmb {
    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmb> {
        let start_offset = walker.offset();
        let top_header = Self::parse_top_header(walker)?;
        let header = Self::parse_header(walker)?;

        let current_offset = (walker.offset() - start_offset) as u32;
        let offsets = Self::parse_offsets(walker, &header, current_offset)?;

        let mut models_list = Vec::with_capacity(offsets.len());

        for offset in offsets {
            walker.goto_usize(offset as usize);
            let block_header = Self::parse_block_header(walker)?;
            eprintln!("{:?}", block_header);
            if block_header.model_count > 50 {
                return Err(anyhow!(
                    "Corrupt MMB model counts: {}",
                    block_header.model_count
                ));
            }

            let models = (0..block_header.model_count)
                .into_iter()
                .map(|_| {
                    let model_header = Self::parse_model_header(walker)?;

                    let vertices = if top_header.has_d_values() {
                        (0..model_header.vertex_count)
                            .map(|_| Self::parse_block_vertex::<_, true>(walker))
                            .collect::<Result<Vec<_>>>()?
                    } else {
                        (0..model_header.vertex_count)
                            .map(|_| Self::parse_block_vertex::<_, false>(walker))
                            .collect::<Result<Vec<_>>>()?
                    };

                    let index_count = walker.step::<u32>()? & 0xFFFF;

                    let mut indices = Vec::with_capacity(index_count as usize);
                    if top_header.has_triangle_list() {
                        for _ in 0..index_count {
                            indices.push((walker.step::<u32>()? & 0xFFFF) as u16);
                        }
                    } else {
                        for _ in 0..index_count {
                            indices.push((walker.step::<u32>()? & 0xFFFF) as u16);
                        }
                    };

                    Ok(ZoneMmbVertexIndices {
                        draw_type: if top_header.has_triangle_list() {
                            GlDrawType::TriangleList
                        } else {
                            GlDrawType::TriangleStrip
                        },
                        texture_name: model_header.texture_name,
                        blending: model_header.blending,
                        vertex_count: model_header.vertex_count,
                        vertices,
                        index_count,
                        indices,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            models_list.push(models);
        }

        let data_len = walker.len() as u32;
        Ok(ZoneMmb {
            data_len,
            top_header,
            header,
            models_list,
        })
    }

    fn parse_top_header<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmbTopHeader> {
        let first_three_bytes = walker.read_bytes(3)?;
        let char_code = std::str::from_utf8(first_three_bytes);

        // Parse out the top header
        let header = if char_code == Ok("MMB") {
            walker.skip(3);
            let first_u32 = walker.step::<u32>()?;
            let second_u32 = walker.step::<u32>()?;
            let third_u32 = walker.step::<u32>()?;

            let mmb_type = (first_u32 & 0x7F) as u8;
            let next = first_u32 & 0x3FFFFFF;
            ZoneMmbTopHeader::Type1 {
                mmb_type,
                next,
                first_u32,
                second_u32,
                third_u32,
            }
        } else {
            let first_u32 = walker.step::<u32>()?;
            let len = first_u32 & 0xFFFFFF;
            let d1 = (first_u32 >> 24 & 0xFF) as u8;

            ZoneMmbTopHeader::Type2 {
                len,
                d1,
                d3: walker.step::<u8>()?,
                d4: walker.step::<u8>()?,
                d5: walker.step::<u8>()?,
                d6: walker.step::<u8>()?,
                unknown_0x08: walker.step::<u32>()?,
                unknown_0x12: walker.step::<u32>()?,
            }
        };

        Ok(header)
    }

    fn parse_header<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmbHeader> {
        Ok(ZoneMmbHeader {
            img_id: walker.take_bytes(16)?.try_into()?,
            pieces: walker.step()?,
            x1: walker.step()?,
            x2: walker.step()?,
            y1: walker.step()?,
            y2: walker.step()?,
            z1: walker.step()?,
            z2: walker.step()?,
            offset_block_header: walker.step()?,
        })
    }

    fn parse_block_header<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmbBlockHeader> {
        Ok(ZoneMmbBlockHeader {
            model_count: walker.step()?,
            x1: walker.step()?,
            x2: walker.step()?,
            y1: walker.step()?,
            y2: walker.step()?,
            z1: walker.step()?,
            z2: walker.step()?,
            face_id: walker.step()?,
        })
    }

    fn parse_model_header<T: ByteWalker>(walker: &mut T) -> Result<ZoneMmbModelHeader> {
        Ok(ZoneMmbModelHeader {
            texture_name: walker.take_bytes(16)?.try_into()?,
            vertex_count: walker.step()?,
            blending: walker.step()?,
        })
    }

    fn parse_block_vertex<T: ByteWalker, const WITH_D: bool>(
        walker: &mut T,
    ) -> Result<ZoneMmbBlockVertex> {
        Ok(ZoneMmbBlockVertex {
            x: walker.step()?,
            y: walker.step()?,
            z: walker.step()?,
            dx: if WITH_D { walker.step()? } else { 0.0 },
            dy: if WITH_D { walker.step()? } else { 0.0 },
            dz: if WITH_D { walker.step()? } else { 0.0 },
            hx: walker.step()?,
            hy: walker.step()?,
            hz: walker.step()?,
            color: walker.step()?,
            u: walker.step()?,
            v: walker.step()?,
        })
    }

    fn parse_offsets<T: ByteWalker>(
        walker: &mut T,
        header: &ZoneMmbHeader,
        current_offset: u32,
    ) -> Result<Vec<u32>> {
        let offsets = if header.offset_block_header == 0 {
            if header.pieces != 0 {
                let mut list = vec![];
                for _ in 0..8 {
                    let offset = walker.step::<u32>()?;
                    if offset != 0 {
                        list.push(offset)
                    }
                }
                list
            } else {
                vec![current_offset]
            }
        } else {
            let mut list = vec![header.offset_block_header];
            let max_range = header.offset_block_header - current_offset;
            if max_range > 0 {
                for _ in 0..8 {
                    let offset = walker.step::<u32>()?;
                    if offset != 0 {
                        list.push(offset)
                    }
                }
                if list.len() != header.pieces as usize {
                    return Err(anyhow!("Mismatched offsets"));
                }
            }
            list
        };

        Ok(offsets)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufWriter, path::PathBuf};

    use crate::{
        dat_format::DatFormat,
        formats::zone_data::{ChunkData, ZoneData},
    };

    #[test]
    pub fn zone_mmb() {
        let resources_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let names = ["Pashhow_Marshlands", "Yughott_Grotto"];

        for name in names {
            let mut path = resources_path.clone();
            path.push(format!("resources/test/zone_data_{name}.DAT"));

            let data = ZoneData::from_path(&path).unwrap();
            let zone_mmb = data
                .chunks
                .into_iter()
                .find_map(|chunk| match chunk.data {
                    ChunkData::ZoneMmb { zone_mmb } => Some(zone_mmb),
                    _ => None,
                })
                .unwrap();

            let file = File::create(format!("{name}.yml")).unwrap();
            serde_yaml::to_writer(BufWriter::new(file), &zone_mmb).unwrap();
        }
    }
}
