//! Export event DATs to YAML. Annotation is currently disabled.
//! See processor::enrichment module header for future implementation notes.
//!
//! Uses the project path from app persistence (same as the Tauri app). Run the app
//! and select a project folder first, or set XI_TINKERER_PROJECT env var.

use std::{env, fs::File, io::Write, path::PathBuf, sync::Arc};
use anyhow::{Context, Result};
use dats::{context::DatContext, formats::entity_names::EntityNames, id_mapping::DatIdMapping};
use processor::enrichment;
use project::project_path;

fn project_path() -> Option<PathBuf> {
    env::var("XI_TINKERER_PROJECT").ok().map(PathBuf::from).or_else(project::project_path)
}

fn main() -> Result<()> {
    let ffxi_path = PathBuf::from(r"C:\Program Files (x86)\PlayOnline\SquareEnix\FINAL FANTASY XI");
    let dat_context = Arc::new(DatContext::from_ffxi_path(ffxi_path)?);

    let test_zones: Vec<(u16, &str)> = vec![
        (100, "West Ronfaure"),
        (230, "Southern San d'Oria"),
        (231, "Northern San d'Oria"),
        (234, "Bastok Mines"),
        (235, "Bastok Markets"),
        (236, "Port Bastok"),
        (238, "Metalworks"),
        (240, "Windurst Waters"),
        (241, "Windurst Woods"),
        (242, "Port Windurst"),
        (243, "Windurst Walls"),
        (245, "Ru'Lude Gardens"),
        (248, "Aht Urhgan Whitegate"),
        (256, "Western Adoulin"),
        (257, "Eastern Adoulin"),
    ];

    for &(zone_id, zone_name) in &test_zones {
        let Ok(event_dat) = DatIdMapping::get().events.get_result(&zone_id).cloned() else {
            continue;
        };
        let Ok(mut extracted) = dat_context.get_data_from_dat(&event_dat) else {
            continue;
        };

        if let Ok(entities_dat) = DatIdMapping::get().entities.get_result(&zone_id) {
            if let Ok(entity_data) = dat_context.get_data_from_dat::<EntityNames>(entities_dat) {
                let name_map = entity_data.dat.id_to_name_map();
                for block in &mut extracted.dat.blocks {
                    block.actor_name = name_map.get(&block.actor_number).cloned();
                }
            }
        }

        enrichment::enrich_event_blocks(&mut extracted.dat.blocks, &dat_context, zone_id);

        if zone_id == 241 {
            let project = project_path()
                .context("No project path. Run the app and select a project, or set XI_TINKERER_PROJECT")?;
            let out_path = project
                .join("raw_data/events/Windurst_Woods.yml");
            std::fs::create_dir_all(out_path.parent().unwrap())?;
            let yaml = serde_yaml::to_string(&extracted.dat)?;
            let mut file = File::create(&out_path)?;
            file.write_all(yaml.as_bytes())?;
            println!("{} (zone {}): Exported to {}", zone_name, zone_id, out_path.display());
        }
    }

    println!("\nDone. Annotation is disabled; immed_data exported as plain values.");

    Ok(())
}
