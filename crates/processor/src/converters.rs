use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
    sync::Arc,
};

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use dats::{
    base::{Dat, DatId},
    context::DatContext,
    dat_format::DatFormat,
    formats::entity_names::EntityNames,
    id_mapping::{DatIdMapping, DatUsage},
};
use regex::Regex;
use serde::Serialize;

use crate::enrichment;
pub(crate) struct DatToYamlConverter {
    pub dat_context: Arc<DatContext>,
    pub raw_data_path: PathBuf,
}

impl DatToYamlConverter {
    pub(crate) fn export_enriched_event(self, zone_id: u16) -> Result<PathBuf> {
        let event_dat = DatIdMapping::get()
            .events
            .get_result(&zone_id)?
            .clone();
        let mut extracted = self.dat_context
            .get_data_from_dat(&event_dat)?;

        if let Ok(entities_dat) = DatIdMapping::get().entities.get_result(&zone_id) {
            if let Ok(entity_data) = self.dat_context.get_data_from_dat::<EntityNames>(entities_dat) {
                let name_map = entity_data.dat.id_to_name_map();
                for block in &mut extracted.dat.blocks {
                    block.actor_name = name_map.get(&block.actor_number).cloned();
                }
            }
        }

        enrichment::enrich_event_blocks(&mut extracted.dat.blocks, &self.dat_context, zone_id);

        let yaml = serde_yaml::to_string(&extracted.dat)?;
        let yaml = inline_immed_comments(&yaml);

        let parent = self
            .raw_data_path
            .parent()
            .ok_or_else(|| anyhow!("Events output path has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let mut file = File::create(&self.raw_data_path).map_err(|err| {
            anyhow!("Could not create file {}: {}", self.raw_data_path.display(), err)
        })?;
        file.write_all(yaml.as_bytes())?;

        Ok(extracted.path)
    }
}

static INLINE_IMMED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^(\s*)- value: (\d+)\n\s+comment: (.+)$").unwrap()
});

/// `- value: N` + `comment: X` → `- N # X`
fn inline_immed_comments(yaml: &str) -> String {
    INLINE_IMMED_RE.replace_all(yaml, "$1- $2 # $3").into_owned()
}

/// Events DAT id → zone_id. Ranges: 5820–6076, 84991–85247, 67911–68167.
fn event_zone_id_from_dat_id(id: u32) -> Option<u16> {
    if (5820..6076).contains(&id) {
        Some((id - 5820) as u16)
    } else if (84991..85247).contains(&id) {
        Some((256 + (id - 84991)) as u16)
    } else if (67911..68167).contains(&id) {
        Some((1000 + (id - 67911)) as u16)
    } else {
        None
    }
}

impl DatUsage<PathBuf> for DatToYamlConverter {
    fn use_dat<T: DatFormat + Serialize + for<'b> serde::Deserialize<'b>>(
        self,
        dat: Dat<T>,
    ) -> Result<PathBuf> {
        let dat_id: DatId = (&dat).into();
        if let Some(zone_id) = event_zone_id_from_dat_id(dat_id.get_inner()) {
            return self.export_enriched_event(zone_id);
        }

        let data = self.dat_context.get_data_from_dat(&dat)?;

        let parent = self
            .raw_data_path
            .parent()
            .ok_or_else(|| anyhow!("Raw data output path has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let file = File::create(&self.raw_data_path).map_err(|err| {
            anyhow!(
                "Could not create at file {}: {}",
                self.raw_data_path.display(),
                err
            )
        })?;

        serde_yaml::to_writer(BufWriter::new(file), &data.dat)?;

        Ok(data.path)
    }
}

pub(crate) struct YamlToDatConverter {
    pub dat_context: Arc<DatContext>,
    pub raw_data_path: PathBuf,
    pub dat_root_path: PathBuf,
}

impl DatUsage<PathBuf> for YamlToDatConverter {
    fn use_dat<T: DatFormat + Serialize + for<'a> serde::Deserialize<'a>>(
        self,
        dat: Dat<T>,
    ) -> Result<PathBuf> {
        let relative_dat_path = dat.get_relative_dat_path(&self.dat_context)?;
        let dat_path = self.dat_root_path.join(relative_dat_path);

        let parent = dat_path
            .parent()
            .ok_or_else(|| anyhow!("Dat output path has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let mut dat_file = File::create(&dat_path)
            .map_err(|err| anyhow!("Could not create file at {}: {}", dat_path.display(), err))?;

        let raw_data_file = File::open(&self.raw_data_path).map_err(|err| {
            anyhow!(
                "Could not open file at {}: {}",
                self.raw_data_path.display(),
                err
            )
        })?;
        let data: T = serde_yaml::from_reader(BufReader::new(raw_data_file))?;

        dat_file.write_all(&data.to_bytes()?)?;

        Ok(dat_path)
    }
}
