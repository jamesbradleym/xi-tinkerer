mod analyze_meshes;
mod export_dat;
mod export_ximesh;
mod make_dats;
mod scan_dats;
mod util;

use std::{path::PathBuf, sync::Arc};

use analyze_meshes::analyze_zone_meshes;
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};

use dats::base::DatId;
use dats::context::DatContext;
use dats::id_mapping::{DatDescriptor, DatLanguage};
use export_ximesh::export_zone_meshes;
use make_dats::make_dats;
use processor::dat_yaml_util::DatYamlUtil;

use crate::{export_dat::export_dat, scan_dats::scan_dats};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    ExportZoneMesh {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: String,

        #[arg(value_name = "OUT_DIR")]
        out_dir: Option<String>,
    },

    AnalyzeZoneMesh {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: String,
    },

    MakeDats {
        #[arg(value_name = "PROJECT_DIR")]
        project_dir: PathBuf,

        #[arg(value_name = "YAML_FILES")]
        yaml_files: Vec<PathBuf>,

        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    ScanDats {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: PathBuf,
    },

    ExportDat {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: PathBuf,

        #[arg(long)]
        dat_path: Option<PathBuf>,

        #[arg(long)]
        dat_id: Option<u32>,

        #[arg(value_name = "OUT_PATH")]
        out_path: Option<PathBuf>,
    },
    ExportZoneEvents {
        #[arg(value_name = "FFXI_PATH")]
        ffxi_path: PathBuf,

        #[arg(value_name = "ZONE_NAME")]
        zone_name: String,

        #[arg(value_name = "OUT_DIR")]
        out_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::ExportZoneMesh { ffxi_path, out_dir } => {
            export_zone_meshes(
                PathBuf::from(ffxi_path),
                PathBuf::from(out_dir.unwrap_or(".".to_string())),
            )
            .await?;
        }
        Commands::AnalyzeZoneMesh { ffxi_path } => {
            analyze_zone_meshes(PathBuf::from(ffxi_path)).await?;
        }
        Commands::MakeDats {
            project_dir,
            yaml_files,
            out,
        } => {
            make_dats(project_dir, &yaml_files, out)?;
        }
        Commands::ScanDats { ffxi_path } => {
            scan_dats(ffxi_path)?;
        }
        Commands::ExportDat {
            ffxi_path,
            dat_path,
            dat_id,
            out_path,
        } => {
            export_dat(ffxi_path, dat_path, dat_id.map(DatId::from), out_path)?;
        }
        Commands::ExportZoneEvents {
            ffxi_path,
            zone_name,
            out_dir,
        } => {
            let dat_context = Arc::new(DatContext::from_ffxi_path(ffxi_path)?);
            let zone_id = dat_context
                .zone_name_to_id_map
                .get(&zone_name)
                .copied()
                .ok_or_else(|| anyhow!("Zone '{}' not found. Available zones can be listed with scan-dats.", zone_name))?;
            println!("Exporting events bundle for zone '{}' (id={})...", zone_name, zone_id);
            let raw_data_root = out_dir;

            let path = DatYamlUtil::dat_to_yaml(
                &DatDescriptor::Events(zone_id),
                DatLanguage::English,
                dat_context.clone(),
                raw_data_root.clone(),
            )?;
            println!("  events: {}", path.display());

            let path = DatYamlUtil::dat_to_yaml(
                &DatDescriptor::Dialog(zone_id),
                DatLanguage::English,
                dat_context.clone(),
                raw_data_root.clone(),
            )?;
            println!("  dialog: {}", path.display());

            if dats::id_mapping::DatIdMapping::get().dialog2.get_result(&zone_id).is_ok() {
                let path = DatYamlUtil::dat_to_yaml(
                    &DatDescriptor::Dialog2(zone_id),
                    DatLanguage::English,
                    dat_context.clone(),
                    raw_data_root.clone(),
                )?;
                println!("  dialog2: {}", path.display());
            }

            let path = DatYamlUtil::dat_to_yaml(
                &DatDescriptor::EntityNames(zone_id),
                DatLanguage::English,
                dat_context.clone(),
                raw_data_root.clone(),
            )?;
            println!("  entity_names: {}", path.display());

            println!("Done.");
        }
    }

    Ok(())
}
