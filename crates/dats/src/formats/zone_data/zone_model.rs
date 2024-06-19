use anyhow::{anyhow, Result};
use common::{byte_walker::ByteWalker, slice_byte_walker::SliceByteWalker};
use serde_derive::{Deserialize, Serialize};

use crate::serde_hex;

use super::{collision_mesh::CollisionMesh, ZoneData};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ZoneModel {
    pub data_len: u32,

    pub unknown_0x00: u32,
    pub unknown_0x04: u32,
    pub mesh_offset: u32,

    pub grid_width: u16,
    pub grid_height: u16,
    pub bucket_width: u8,
    pub bucket_height: u8,

    pub quadtree_offset: u32,
    pub objects_end_offset: u32,
    pub shortname_offset: u32,

    #[serde(with = "serde_hex")]
    pub unknown_data: Vec<u8>,

    pub mesh_model_count: u32,
    pub mesh_model_data: u32,
    pub mesh_grid_bucket_lists_count: u32,
    pub mesh_grid_bucket_lists: u32,
    pub grid_offset: u32,
    pub map_id_list_offset: u32,
    pub map_id_list_count: u32,

    pub collision_mesh: CollisionMesh,
}

impl ZoneModel {
    pub fn parse_from_zone_data(zone_data: &ZoneData) -> Result<ZoneModel> {
        let model_chunk_data = zone_data
            .chunks
            .iter()
            .find_map(|chunk| {
                if chunk.chunk_type == 0x1C {
                    Some(&chunk.data)
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow!("Did not find zone model in zone data."))?;

        let mut walker = SliceByteWalker::new(model_chunk_data);
        Self::parse(&mut walker)
    }

    pub fn parse<T: ByteWalker>(walker: &mut T) -> Result<ZoneModel> {
        let data_len = walker.len() as u32;

        let unknown_0x00 = walker.step::<u32>()?;
        let unknown_0x04 = walker.step::<u32>()?;
        let mesh_offset = walker.step::<u32>()?;

        if data_len <= mesh_offset {
            return Err(anyhow!(
                "Invalid mesh offset found: {mesh_offset}, expecting less than {data_len}"
            ));
        }

        let grid_width = walker.step::<u8>()? as u16 * 10;
        let grid_height = walker.step::<u8>()? as u16 * 10;
        let bucket_width = walker.step::<u8>()?;
        let bucket_height = walker.step::<u8>()?;

        let quadtree_offset = walker.step::<u32>()?;

        let objects_start_offset = 0x20;
        let objects_end_offset = walker.step::<u32>()?;
        let _objects_count = (objects_end_offset - objects_start_offset) / 0x64;

        let shortname_offset = walker.step::<u32>()?;
        let _shortname_count = (mesh_offset - shortname_offset) / 0x4C;

        let unknown_data = walker
            .take_bytes(mesh_offset as usize - walker.offset())?
            .to_vec();

        let mesh_model_count = walker.step::<u32>()?;
        let mesh_model_data = walker.step::<u32>()?;

        let mesh_grid_bucket_lists_count = walker.step::<u32>()?;
        let mesh_grid_bucket_lists = walker.step::<u32>()?;
        let grid_offset = walker.step::<u32>()?;

        let map_id_list_offset = walker.step::<u32>()?;
        let map_id_list_count = walker.step::<u32>()?;

        // Likely skipping some stuff here

        let collision_mesh = CollisionMesh::parse(walker, grid_offset, grid_height, grid_width)?;

        Ok(ZoneModel {
            data_len,

            unknown_0x00,
            unknown_0x04,
            mesh_offset,

            grid_width,
            grid_height,
            bucket_width,
            bucket_height,

            quadtree_offset,
            objects_end_offset,
            shortname_offset,

            unknown_data,

            mesh_model_count,
            mesh_model_data,
            mesh_grid_bucket_lists_count,
            mesh_grid_bucket_lists,
            grid_offset,
            map_id_list_offset,
            map_id_list_count,

            collision_mesh,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufWriter, path::PathBuf};

    use crate::{dat_format::DatFormat, formats::zone_data::ZoneData};

    use super::ZoneModel;

    #[test]
    pub fn zone_model() {
        let resources_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let names = ["Pashhow_Marshlands", "Yughott_Grotto"];

        for name in names {
            let mut path = resources_path.clone();
            path.push(format!("resources/test/zone_data_{name}.DAT"));

            let data = ZoneData::from_path(&path).unwrap();
            let res = ZoneModel::parse_from_zone_data(&data).unwrap();

            let file = File::create(format!("{name}.yml")).unwrap();
            serde_yaml::to_writer(BufWriter::new(file), &res).unwrap();
        }
    }
}
