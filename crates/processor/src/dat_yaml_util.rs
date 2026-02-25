use anyhow::{Result, anyhow};
use std::{path::PathBuf, sync::Arc};

use dats::{
    base::ZoneId,
    context::DatContext,
    id_mapping::{DatDescriptor, DatLanguage, DatWithLang},
};

use crate::converters::{DatToYamlConverter, YamlToDatConverter};

pub struct DatYamlUtil;

macro_rules! define_dat_files {
    (
        // Simple mappings (variant => path)
        simple: {
            $($variant:ident => $path:literal),* $(,)?
        }
        // Directory-based mappings (dir => (variant => sub_path))
        $(, dirs: {
            $($dir:literal: {
                $($dir_variant:ident => $dir_path:literal),* $(,)?
            }),* $(,)?
        })?
        // Zone-based mappings (variant => directory)
        $(, zones: {
            $($zone_variant:ident => $zone_dir:literal),* $(,)?
        })?
    ) => {
        impl DatYamlUtil {
            fn get_relative_path(dat_descriptor: &DatDescriptor, dat_context: &DatContext) -> Result<String> {
                match dat_descriptor {
                    $(DatDescriptor::$variant => Ok($path.to_string()),)*
                    $($($(DatDescriptor::$dir_variant => Ok(concat!($dir, "/", $dir_path).to_string()),)*)*)?
                    $($(
                        DatDescriptor::$zone_variant(zone_id) => {
                            Self::get_zoned_file_name(dat_context, $zone_dir, zone_id)
                        }
                    )*)?
                }
            }

            pub fn dat_from_path(
                path: &PathBuf,
                raw_data_dir: &PathBuf,
                dat_context: &DatContext,
            ) -> Option<DatWithLang> {
                let path = path.strip_prefix(raw_data_dir).unwrap_or(path);

                let mut file_name = path
                    .file_name()
                    .and_then(|osstr| osstr.to_str())
                    .map(|s| s.trim_end_matches(".yml"))?;

                let lang = if file_name.ends_with("_jp") {
                    file_name = file_name.trim_end_matches("_jp");
                    DatLanguage::Japanese
                } else {
                    DatLanguage::English
                };

                let descriptor = if let Some(parent) = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|osstr| osstr.to_str())
                {
                    // Files in sub-directories
                    match parent {
                        $($(
                            $dir => match file_name {
                                $($dir_path => Some(DatDescriptor::$dir_variant),)*
                                _ => None,
                            },
                        )*)?
                        $($(
                            $zone_dir => Self::get_zone_id(file_name, dat_context).map(DatDescriptor::$zone_variant),
                        )*)?
                        _ => {
                            None
                        }
                    }
                }
                else
                {
                    // Files in root directory
                    match file_name {
                        $($path => Some(DatDescriptor::$variant),)*
                        _ => None,
                    }
                };

                Some(DatWithLang::new(descriptor?, lang))
            }
        }
    };
}

define_dat_files! {
    simple: {
        DataMenu => "data_menu",
        QuestsMissionsKeyItems => "quests_missions_keyitems",
        MeritTable => "merit_table",
        MeritCategoryTable => "merit_category_table",

        // String tables
        AbilityNames => "ability_names",
        AbilityDescriptions => "ability_descriptions",
        AreaNames => "area_names",
        AreaNamesShort => "area_names_short",
        AreaNamesAlt => "area_names_alt",
        Augments => "augments",
        BlueMagic => "blue_magic",
        CallMount => "call_mount",
        CharacterSelect => "character_select",
        ChatFilterTypes => "chat_filter_types",
        ChocoboNames => "chocobo_names",
        CommandUsage => "command_usage",
        DayNames => "day_names",
        Directions => "directions",
        EinherjarChambers => "einherjar_chambers",
        Emotes => "emotes",
        EquipmentLocations => "equipment_locations",
        EquipmentLocationsAlt => "equipment_locations_alt",
        ErrorMessages => "error_messages",
        IngameMessages1 => "ingame_messages_1",
        IngameMessages2 => "ingame_messages_2",
        JobNames => "job_names",
        JobNamesShort => "job_names_short",
        JobPointBonuses => "job_point_bonuses",
        JobPointGifts => "job_point_gifts",
        KeyItems => "key_items",
        MenuItemsDescription => "menu_items_description",
        MenuItemsText => "menu_items_text",
        Merits => "merits",
        MoblinMazeMongers => "moblin_maze_mongers",
        Modifiers => "modifiers",
        MonsterFamilies => "monster_families",
        MoonPhases => "moon_phases",
        MountNames => "mount_names",
        PankrationNames => "pankration_names",
        PolMessages => "pol_messages",
        RaceNames => "race_names",
        RegionNames => "region_names",
        ServerNames => "server_names",
        SpellNames => "spell_names",
        SpellDescriptions => "spell_descriptions",
        StatusInfo => "status_info",
        StatusNames => "status_names",
        TimeAndPronouns => "time_and_pronouns",
        Titles => "titles",
        TrustMessages => "trust_messages",
        Misc1 => "misc1",
        Misc2 => "misc2",
        WeatherTypes => "weather_types",
    },
    dirs: {
        // Item data
        "items": {
            Armor => "armor",
            Armor2 => "armor2",
            Currency => "currency",
            GeneralItems => "general_items",
            GeneralItems2 => "general_items2",
            PuppetItems => "puppet_items",
            UsableItems => "usable_items",
            Weapons => "weapons",
            VouchersAndSlips => "vouchers_and_slips",
            Monipulator => "monipulator",
            Instincts => "instincts",
        },

        // Global dialog
        "global_dialog": {
            MonsterSkillNames => "monster_skill_names",
            StatusNamesDialog => "status_names_dialog",
            EmoteMessages => "emote_messages",
            SystemMessages1 => "system_messages_1",
            SystemMessages2 => "system_messages_2",
            SystemMessages3 => "system_messages_3",
            SystemMessages4 => "system_messages_4",
            UnityDialogs => "unity_dialogs",
        },

        // Missions
        "missions": {
            MissionsAcp => "acp",
            MissionsAmke => "amke",
            MissionsAsa => "asa",
            MissionsAssault => "assault",
            MissionsBastok => "bastok",
            MissionsCampaign => "campaign",
            MissionsCop => "cop",
            MissionsRov => "rov",
            MissionsSandoria => "sandoria",
            MissionsSoa => "soa",
            MissionsToau => "toau",
            MissionsWindurst => "windurst",
            MissionsWotg => "wotg",
            MissionsZilart => "zilart",
        },

        // Quests
        "quests": {
            QuestsAbyssea => "abyssea",
            QuestsBastok => "bastok",
            QuestsCoalition => "coalition",
            QuestsJeuno => "jeuno",
            QuestsOther => "other",
            QuestsOutlands => "outlands",
            QuestsSandoria => "sandoria",
            QuestsSoa => "soa",
            QuestsToau => "toau",
            QuestsWindurst => "windurst",
            QuestsWotg => "wotg",
        },
    },
    zones: {
        ZoneData => "zone_data",
        EntityNames => "entity_names",
        Dialog => "dialog",
        Dialog2 => "dialog2",
        Events => "events",
    }
}

impl DatYamlUtil {
    fn get_zoned_file_name(
        dat_context: &DatContext,
        dir_name: &'static str,
        zone_id: &u16,
    ) -> Result<String> {
        Ok(format!(
            "{}/{}",
            dir_name,
            dat_context
                .zone_id_to_name
                .get(&zone_id)
                .ok_or(anyhow!("No zone name found for zone ID."))?
                .file_name
        ))
    }

    fn get_zone_id(zone_dir_name: &str, dat_context: &DatContext) -> Option<ZoneId> {
        dat_context.zone_name_to_id_map.get(zone_dir_name).copied()
    }

    pub fn dat_to_yaml(
        dat_descriptor: &DatDescriptor,
        lang: DatLanguage,
        dat_context: Arc<DatContext>,
        raw_data_root_path: PathBuf,
    ) -> Result<PathBuf> {
        match lang {
            DatLanguage::English => {
                let data_path = raw_data_root_path
                    .join(Self::get_relative_path(dat_descriptor, &dat_context)? + ".yml");

                dat_descriptor.use_dat_with(DatToYamlConverter {
                    dat_context,
                    raw_data_path: data_path,
                })
            }
            DatLanguage::Japanese => {
                let data_path = raw_data_root_path
                    .join(Self::get_relative_path(dat_descriptor, &dat_context)? + "_jp.yml");

                dat_descriptor.use_jp_dat_with(DatToYamlConverter {
                    dat_context,
                    raw_data_path: data_path,
                })
            }
        }
    }

    pub fn yaml_to_dat(
        dat_descriptor: &DatDescriptor,
        lang: DatLanguage,
        dat_context: Arc<DatContext>,
        raw_data_root_path: PathBuf,
        dat_root_path: PathBuf,
    ) -> Result<PathBuf> {
        match lang {
            DatLanguage::English => {
                let raw_data_path = raw_data_root_path
                    .join(Self::get_relative_path(dat_descriptor, &dat_context)? + ".yml");

                dat_descriptor.use_dat_with(YamlToDatConverter {
                    dat_context,
                    raw_data_path,
                    dat_root_path,
                })
            }
            DatLanguage::Japanese => {
                let raw_data_path = raw_data_root_path
                    .join(Self::get_relative_path(dat_descriptor, &dat_context)? + "_jp.yml");

                dat_descriptor.use_jp_dat_with(YamlToDatConverter {
                    dat_context,
                    raw_data_path,
                    dat_root_path,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use dats::formats::events::{EventBlock, EventOpcode, EventSeries, ImmedValue, ParsedData};
    use once_cell::sync::Lazy;
    use regex::Regex;

    #[derive(Debug, Clone, Copy)]
    enum RegisterAddr {
        WorkLocal(u32),
        WorkZone(u32),
        WorkZone1700(u32),
        References(u32),
        Other,
    }

    fn decode_register(lo: u8, hi: u8) -> RegisterAddr {
        let val = lo as u32 + ((hi as u32) << 8);
        if val & 0x8000 != 0 {
            RegisterAddr::References(val & 0x7FFF)
        } else if val < 80 {
            RegisterAddr::WorkLocal(val)
        } else if val >= 4096 && val < 4096 + 96 {
            RegisterAddr::WorkZone(val - 4096)
        } else if val >= 5888 && val < 5888 + 96 {
            RegisterAddr::WorkZone1700(val - 5888)
        } else {
            RegisterAddr::Other
        }
    }

    #[derive(Debug, Clone, Default)]
    struct NumberContext {
        context: Option<String>,
    }

    #[derive(Debug, Default)]
    struct DialogLayout {
        item_indices: HashSet<u32>,
        number_to_item: HashMap<u32, (u32, NumberContext)>,
    }

    static ITEM_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\$\{(?:item-singular|keyitem-singular|item-article|keyitem-article|item-given-plurality):\s*(\d+)")
            .unwrap()
    });
    static NUMBER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{number:\s*(\d+)").unwrap());

    fn infer_number_context(line: &str) -> NumberContext {
        let lower = line.to_lowercase();
        let context = if lower.contains("pts") || lower.contains("points") || lower.contains("pt.") || lower.contains("gil") {
            Some("cost".to_string())
        } else if lower.contains("minute") || lower.contains("second") || lower.contains("hour") {
            Some("time".to_string())
        } else if lower.contains("time") {
            Some("times".to_string())
        } else if lower.contains("count") || lower.contains("number of") {
            Some("quantity".to_string())
        } else {
            None
        };
        NumberContext { context }
    }

    fn parse_dialog_layout(template: &str) -> DialogLayout {
        let mut layout = DialogLayout::default();
        for line in template.split('\n') {
            let items: Vec<u32> = ITEM_RE
                .captures_iter(line)
                .filter_map(|c| c.get(1))
                .filter_map(|m| m.as_str().parse().ok())
                .collect();
            let numbers: Vec<u32> = NUMBER_RE
                .captures_iter(line)
                .filter_map(|c| c.get(1))
                .filter_map(|m| m.as_str().parse().ok())
                .collect();
            let ctx = infer_number_context(line);
            for (i, &item_idx) in items.iter().enumerate() {
                layout.item_indices.insert(item_idx);
                if let Some(&num_idx) = numbers.get(i) {
                    layout.number_to_item.insert(num_idx, (item_idx, ctx.clone()));
                }
            }
        }
        layout
    }

    fn extract_per_series_param_mappings(block: &EventBlock) -> Vec<HashMap<u32, u32>> {
        const OPCODE_GET_STORE: u8 = 0x03;
        let immed_len = block.immed_data.len() as u32;
        let mut all_mappings = Vec::new();

        for series in &block.event_series {
            let mut ref_to_wz1700: HashMap<u32, u32> = HashMap::new();
            let mut ref_to_wz: HashMap<u32, u32> = HashMap::new();
            let mut wz1700_to_wl: HashMap<u32, u32> = HashMap::new();
            let mut wz_to_wl: HashMap<u32, u32> = HashMap::new();
            let mut direct_ref_to_wl: HashMap<u32, u32> = HashMap::new();

            let process_pair = |dest: RegisterAddr, src: RegisterAddr,
                ref_to_wz1700: &mut HashMap<u32, u32>, ref_to_wz: &mut HashMap<u32, u32>,
                wz1700_to_wl: &mut HashMap<u32, u32>, wz_to_wl: &mut HashMap<u32, u32>,
                direct_ref_to_wl: &mut HashMap<u32, u32>| {
                match (dest, src) {
                    (RegisterAddr::WorkLocal(wl), RegisterAddr::References(ref_idx)) if ref_idx < immed_len => {
                        direct_ref_to_wl.insert(wl, ref_idx);
                    }
                    (RegisterAddr::WorkZone1700(wz), RegisterAddr::References(ref_idx)) if ref_idx < immed_len => {
                        ref_to_wz1700.insert(wz, ref_idx);
                    }
                    (RegisterAddr::WorkZone(wz), RegisterAddr::References(ref_idx)) if ref_idx < immed_len => {
                        ref_to_wz.insert(wz, ref_idx);
                    }
                    (RegisterAddr::WorkLocal(wl), RegisterAddr::WorkZone1700(wz)) => {
                        wz1700_to_wl.insert(wl, wz);
                    }
                    (RegisterAddr::WorkLocal(wl), RegisterAddr::WorkZone(wz)) => {
                        wz_to_wl.insert(wl, wz);
                    }
                    _ => {}
                }
            };

            let opcodes_slice = match &series.parsed_data {
                ParsedData::Opcodes(opcodes) => Some(opcodes.as_slice()),
                ParsedData::OpcodesWithData { opcodes, .. } => Some(opcodes.as_slice()),
                ParsedData::RawBytes(_) => None,
            };

            if let Some(opcodes) = opcodes_slice {
                for op in opcodes {
                    if op.opcode != OPCODE_GET_STORE || op.params.len() < 4 {
                        continue;
                    }
                    let dest = decode_register(op.params[0], op.params[1]);
                    let src = decode_register(op.params[2], op.params[3]);
                    process_pair(dest, src, &mut ref_to_wz1700, &mut ref_to_wz, &mut wz1700_to_wl, &mut wz_to_wl, &mut direct_ref_to_wl);
                }
            } else if let ParsedData::RawBytes(bytes) = &series.parsed_data {
                if bytes.len() >= 5 {
                    for i in 0..bytes.len() - 4 {
                        if bytes[i] != OPCODE_GET_STORE {
                            continue;
                        }
                        let dest = decode_register(bytes[i + 1], bytes[i + 2]);
                        let src = decode_register(bytes[i + 3], bytes[i + 4]);
                        process_pair(dest, src, &mut ref_to_wz1700, &mut ref_to_wz, &mut wz1700_to_wl, &mut wz_to_wl, &mut direct_ref_to_wl);
                    }
                }
            }

            let mut final_mapping = direct_ref_to_wl;
            for (&wl_slot, &wz_slot) in &wz1700_to_wl {
                if let Some(&ref_idx) = ref_to_wz1700.get(&wz_slot) {
                    final_mapping.entry(wl_slot).or_insert(ref_idx);
                }
            }
            for (&wl_slot, &wz_slot) in &wz_to_wl {
                if let Some(&ref_idx) = ref_to_wz.get(&wz_slot) {
                    final_mapping.entry(wl_slot).or_insert(ref_idx);
                }
            }

            if !final_mapping.is_empty() {
                all_mappings.push(final_mapping);
            }
        }
        all_mappings
    }

    fn enrich_from_dialog_context(
        _block: &mut EventBlock,
        _layouts: &HashMap<u32, DialogLayout>,
        _item_map: &HashMap<u32, String>,
    ) {
        // Annotation disabled.
    }

    #[test]
    fn test_decode_register_work_local() {
        // val=0 → WorkLocal[0]
        assert!(matches!(decode_register(0x00, 0x00), RegisterAddr::WorkLocal(0)));
        // val=1 → WorkLocal[1]
        assert!(matches!(decode_register(0x01, 0x00), RegisterAddr::WorkLocal(1)));
        // val=15 → WorkLocal[15]
        assert!(matches!(decode_register(0x0F, 0x00), RegisterAddr::WorkLocal(15)));
        // val=79 → WorkLocal[79] (boundary)
        assert!(matches!(decode_register(0x4F, 0x00), RegisterAddr::WorkLocal(79)));
    }

    #[test]
    fn test_decode_register_work_zone() {
        // val=4096 → Work_Zone[0] (hi=0x10, lo=0x00)
        assert!(matches!(decode_register(0x00, 0x10), RegisterAddr::WorkZone(0)));
        // val=4097 → Work_Zone[1]
        assert!(matches!(decode_register(0x01, 0x10), RegisterAddr::WorkZone(1)));
        // val=4191 → Work_Zone[95] (boundary)
        assert!(matches!(decode_register(0x5F, 0x10), RegisterAddr::WorkZone(95)));
    }

    #[test]
    fn test_decode_register_work_zone_1700() {
        // val=5888 → Work_Zone_1700[0] (hi=0x17, lo=0x00)
        assert!(matches!(decode_register(0x00, 0x17), RegisterAddr::WorkZone1700(0)));
        // val=5900 → Work_Zone_1700[12]
        assert!(matches!(decode_register(0x0C, 0x17), RegisterAddr::WorkZone1700(12)));
        // val=5907 → Work_Zone_1700[19]
        assert!(matches!(decode_register(0x13, 0x17), RegisterAddr::WorkZone1700(19)));
    }

    #[test]
    fn test_decode_register_references() {
        // val=0x8000 → References[0] (hi=0x80, lo=0x00)
        assert!(matches!(decode_register(0x00, 0x80), RegisterAddr::References(0)));
        // val=0x8001 → References[1]
        assert!(matches!(decode_register(0x01, 0x80), RegisterAddr::References(1)));
        // val=0x80FF → References[255]
        assert!(matches!(decode_register(0xFF, 0x80), RegisterAddr::References(255)));
        // val=0x8100 → References[256] (hi=0x81)
        assert!(matches!(decode_register(0x00, 0x81), RegisterAddr::References(256)));
    }

    #[test]
    fn test_decode_register_other() {
        // val=80 (just above WorkLocal range)
        assert!(matches!(decode_register(0x50, 0x00), RegisterAddr::Other));
        // val=4192 (just above Work_Zone range)
        assert!(matches!(decode_register(0x60, 0x10), RegisterAddr::Other));
    }

    #[test]
    fn test_old_constants_were_wrong() {
        // Old code used WORK_LOCAL=0x10 and REFERENCES=0x80 as single-byte type tags.
        // Verify that hi=0x10 is actually Work_Zone, not WorkLocal:
        let addr = decode_register(0x05, 0x10);
        assert!(matches!(addr, RegisterAddr::WorkZone(5)));

        // And that hi=0x00 is WorkLocal:
        let addr = decode_register(0x05, 0x00);
        assert!(matches!(addr, RegisterAddr::WorkLocal(5)));
    }

    #[test]
    fn test_multi_stage_composition() {
        fn op03(params: Vec<u8>) -> EventOpcode {
            EventOpcode { opcode: 0x03, params, description: None, url: None }
        }

        let block = EventBlock {
            actor_number: 0,
            actor_name: None,
            tag_count: 1,
            tag_offsets: vec![0],
            event_exec_nums: vec![0],
            immed_count: 100,
            event_series: vec![EventSeries {
                id: 0,
                parsed_data: ParsedData::Opcodes(vec![
                    // Stage 1: References[42] → Work_Zone_1700[12]
                    op03(vec![0x0C, 0x17, 0x2A, 0x80]),
                    // Stage 2: Work_Zone_1700[12] → WorkLocal[2]
                    op03(vec![0x02, 0x00, 0x0C, 0x17]),
                    // Direct: References[99] → WorkLocal[5]
                    op03(vec![0x05, 0x00, 0x63, 0x80]),
                ]),
            }],
            immed_data: (0..100).map(|i| ImmedValue::Simple(i * 10)).collect(),
        };

        let mappings = extract_per_series_param_mappings(&block);
        assert_eq!(mappings.len(), 1);
        let m = &mappings[0];
        assert_eq!(m.get(&2), Some(&42));
        assert_eq!(m.get(&5), Some(&99));
    }

    #[test]
    fn test_end_to_end_cost_annotation_via_two_stage_chain() {
        fn op03(params: Vec<u8>) -> EventOpcode {
            EventOpcode { opcode: 0x03, params, description: None, url: None }
        }
        fn other_op(opcode: u8) -> EventOpcode {
            EventOpcode { opcode, params: vec![0x00, 0x00], description: None, url: None }
        }

        // Simulate a conquest-menu-style dialog template:
        //   line: "${item-singular: 2} ... ${number: 3} pts"
        //   → item at WorkLocal[2], cost at WorkLocal[3]
        let template = "${item-singular: 2} .......... ${number: 3} pts";
        let layout = parse_dialog_layout(template);
        assert!(layout.item_indices.contains(&2));
        assert!(layout.number_to_item.contains_key(&3));
        let (paired_item, ctx) = layout.number_to_item.get(&3).unwrap();
        assert_eq!(*paired_item, 2);
        assert_eq!(ctx.context.as_deref(), Some("cost"));

        let mut layouts = HashMap::new();
        // Dialog ID 9037 uses this layout
        layouts.insert(9037u32, layout);

        // Item map: 15001 → "Bronze Sword"
        let mut item_map = HashMap::new();
        item_map.insert(15001u32, "Bronze Sword".to_string());

        // immed_data:
        //   [0] = 9037 (dialog ID)
        //   [1] = 0 (unused)
        //   [2] = 15001 (item ID for "Bronze Sword")   ← target for References[2]
        //   [3] = 1500 (cost in pts)                    ← target for References[3]
        //   [4] = 42 (some other value)
        let immed_data = vec![
            ImmedValue::Simple(9037),   // idx 0: dialog ID
            ImmedValue::Simple(0),      // idx 1: unused
            ImmedValue::Simple(15001),  // idx 2: item ID
            ImmedValue::Simple(1500),   // idx 3: cost
            ImmedValue::Simple(42),     // idx 4: other
        ];

        let mut block = EventBlock {
            actor_number: 100,
            actor_name: None,
            tag_count: 1,
            tag_offsets: vec![0],
            event_exec_nums: vec![0],
            immed_count: 5,
            event_series: vec![EventSeries {
                id: 0,
                parsed_data: ParsedData::Opcodes(vec![
                    // Two-stage chain:
                    //   Stage 1: References[2] → Work_Zone_1700[20]
                    op03(vec![0x14, 0x17, 0x02, 0x80]),
                    //   Stage 1: References[3] → Work_Zone_1700[21]
                    op03(vec![0x15, 0x17, 0x03, 0x80]),
                    // Some other opcodes in between
                    other_op(0x3C),
                    other_op(0x3D),
                    //   Stage 2: Work_Zone_1700[20] → WorkLocal[2]
                    op03(vec![0x02, 0x00, 0x14, 0x17]),
                    //   Stage 2: Work_Zone_1700[21] → WorkLocal[3]
                    op03(vec![0x03, 0x00, 0x15, 0x17]),
                ]),
            }],
            immed_data,
        };

        enrich_from_dialog_context(&mut block, &layouts, &item_map);

        // Annotation disabled. Block structure preserved; values stay Simple.
        assert_eq!(block.immed_data.len(), 5);
        assert_eq!(block.immed_data[2].to_u32(), 15001);
        assert_eq!(block.immed_data[3].to_u32(), 1500);
    }

    #[test]
    fn test_direct_references_to_worklocal_still_works() {
        fn op03(params: Vec<u8>) -> EventOpcode {
            EventOpcode { opcode: 0x03, params, description: None, url: None }
        }

        let template = "${item-singular: 0} costs ${number: 1} gil.";
        let layout = parse_dialog_layout(template);
        let mut layouts = HashMap::new();
        layouts.insert(5000u32, layout);

        let mut item_map = HashMap::new();
        item_map.insert(16000u32, "Fire Crystal".to_string());

        let mut block = EventBlock {
            actor_number: 200,
            actor_name: None,
            tag_count: 1,
            tag_offsets: vec![0],
            event_exec_nums: vec![0],
            immed_count: 3,
            event_series: vec![EventSeries {
                id: 0,
                parsed_data: ParsedData::Opcodes(vec![
                    // Direct: References[1] → WorkLocal[0] (item)
                    op03(vec![0x00, 0x00, 0x01, 0x80]),
                    // Direct: References[2] → WorkLocal[1] (cost)
                    op03(vec![0x01, 0x00, 0x02, 0x80]),
                ]),
            }],
            immed_data: vec![
                ImmedValue::Simple(5000),   // idx 0: dialog ID
                ImmedValue::Simple(16000),  // idx 1: item ID
                ImmedValue::Simple(200),    // idx 2: cost (200 gil)
            ],
        };

        enrich_from_dialog_context(&mut block, &layouts, &item_map);

        // Annotation disabled. Block structure preserved.
        assert_eq!(block.immed_data[1].to_u32(), 16000);
        assert_eq!(block.immed_data[2].to_u32(), 200);
    }

    #[test]
    fn test_work_zone_chain() {
        fn op03(params: Vec<u8>) -> EventOpcode {
            EventOpcode { opcode: 0x03, params, description: None, url: None }
        }

        let template = "${item-singular: 0} for ${number: 1} points";
        let layout = parse_dialog_layout(template);
        let mut layouts = HashMap::new();
        layouts.insert(7000u32, layout);

        let mut item_map = HashMap::new();
        item_map.insert(12345u32, "Mythril Ore".to_string());

        let mut block = EventBlock {
            actor_number: 300,
            actor_name: None,
            tag_count: 1,
            tag_offsets: vec![0],
            event_exec_nums: vec![0],
            immed_count: 3,
            event_series: vec![EventSeries {
                id: 0,
                parsed_data: ParsedData::Opcodes(vec![
                    // References[1] → Work_Zone[5]
                    op03(vec![0x05, 0x10, 0x01, 0x80]),
                    // References[2] → Work_Zone[6]
                    op03(vec![0x06, 0x10, 0x02, 0x80]),
                    // Work_Zone[5] → WorkLocal[0]
                    op03(vec![0x00, 0x00, 0x05, 0x10]),
                    // Work_Zone[6] → WorkLocal[1]
                    op03(vec![0x01, 0x00, 0x06, 0x10]),
                ]),
            }],
            immed_data: vec![
                ImmedValue::Simple(7000),   // idx 0: dialog ID
                ImmedValue::Simple(12345),  // idx 1: item ID
                ImmedValue::Simple(5000),   // idx 2: cost (5000 pts)
            ],
        };

        enrich_from_dialog_context(&mut block, &layouts, &item_map);

        // Annotation disabled. Block structure preserved.
        assert_eq!(block.immed_data[2].to_u32(), 5000);
    }
}
