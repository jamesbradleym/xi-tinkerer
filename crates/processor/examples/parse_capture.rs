//! Parse Captain packet capture logs and search for Harara conquest band values.
//!
//! Usage:
//!   cargo run -p processor --example parse_capture -- <path/to/incoming.log>
//!   cargo run -p processor --example parse_capture -- --dump 0x05E <path>
//!   cargo run -p processor --example parse_capture -- --decode 0x05E <path>
//!
//! Searches for: 500, 1000, 2000 (costs) and 15761, 15762, 15763 (item IDs)
//! as little-endian int32/int16 in packet payloads.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

enum Mode {
    Search,
    Dump(u16),
    Decode(u16),
}

fn find_capture_path() -> Option<PathBuf> {
    if let Ok(p) = env::var("XI_CAPTURE_PATH") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    let home = env::var("USERPROFILE").ok().or_else(|| env::var("HOME").ok())?;
    let candidates = [
        format!("{}\\Documents\\Windower\\addons\\captain\\captures", home),
        format!("{}\\Documents\\Ashita\\addons\\captain\\captures", home),
        "C:\\ffxi\\addons\\captain\\captures".to_string(),
    ];
    for base in &candidates {
        let base_path = PathBuf::from(base);
        if base_path.exists() {
            if let Ok(entries) = fs::read_dir(&base_path) {
                let mut dirs: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                dirs.sort_by(|a, b| {
                    let a_t = a.metadata().and_then(|m| m.modified()).ok();
                    let b_t = b.metadata().and_then(|m| m.modified()).ok();
                    b_t.cmp(&a_t)
                });
                for entry in dirs {
                    let char_dir = entry.path();
                    if let Ok(chars) = fs::read_dir(&char_dir) {
                        for c in chars.filter_map(|e| e.ok()) {
                            let log = c.path().join("packetviewer").join("incoming.log");
                            if log.exists() {
                                return Some(log);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn parse_args(args: &[String]) -> (PathBuf, Mode) {
    let mut path = PathBuf::new();
    let mut mode = Mode::Search;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--dump" && i + 1 < args.len() {
            let hex = args[i + 1].trim_start_matches("0x");
            mode = Mode::Dump(u16::from_str_radix(hex, 16).unwrap_or(0));
            i += 2;
        } else if args[i] == "--decode" && i + 1 < args.len() {
            let hex = args[i + 1].trim_start_matches("0x");
            mode = Mode::Decode(u16::from_str_radix(hex, 16).unwrap_or(0));
            i += 2;
        } else if !args[i].starts_with('-') {
            path = PathBuf::from(&args[i]);
            i += 1;
        } else {
            i += 1;
        }
    }
    if path.as_os_str().is_empty() {
        if let Some(p) = find_capture_path() {
            eprintln!("Using capture: {}", p.display());
            path = p;
        } else {
            eprintln!("Usage: parse_capture [--dump 0xID | --decode 0x05E] <path/to/incoming.log>");
            eprintln!("  Or set XI_CAPTURE_PATH, or place capture in Windower/Ashita captain captures folder.");
            panic!("No capture path provided or found.");
        }
    }
    (path, mode)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (path, mode) = parse_args(&args);
    let content = fs::read_to_string(&path).expect("Failed to read capture file");

    let packets = parse_captain_log(&content);

    match mode {
        Mode::Dump(dump_id) => {
            for p in &packets {
                if p.id == dump_id {
                    dump_packet(p);
                }
            }
            return;
        }
        Mode::Decode(decode_id) => {
            if decode_id == 0x05E {
                for p in &packets {
                    if p.id == 0x05E {
                        decode_conquest_packet(p);
                        search_conquest_packet_for_items(p);
                    }
                }
            } else {
                eprintln!("--decode only supports 0x05E (conquest) for now");
            }
            return;
        }
        Mode::Search => {}
    }
    println!("Parsed {} packets\n", packets.len());

    // Common items menu: Instant Reraise (7), Instant Warp (10), Return Ring (2500),
    // Homing Ring (9000), Chariot Band (500), Empress Band (1000), Emperor Band (2000)
    let targets_i32 = [
        (7u32, "cost (Instant Reraise)"),
        (10, "cost (Instant Warp)"),
        (2500, "cost (Return Ring)"),
        (9000, "cost (Homing Ring)"),
        (500, "cost (Chariot Band)"),
        (1000, "cost (Empress Band)"),
        (2000, "cost (Emperor Band)"),
        (4182, "item (Instant Reraise)"),
        (4181, "item (Instant Warp)"),
        (15542, "item (Return Ring)"),
        (15541, "item (Homing Ring)"),
        (15761, "item (Chariot Band)"),
        (15762, "item (Empress Band)"),
        (15763, "item (Emperor Band)"),
    ];

    let targets_i16 = [
        (7u16, "cost i16 (Instant Reraise)"),
        (10, "cost i16 (Instant Warp)"),
        (500, "cost i16 (Chariot)"),
        (1000, "cost i16 (Empress)"),
        (2000, "cost i16 (Emperor)"),
    ];

    let mut any_match = false;
    for packet in &packets {
        let matches_i32 = search_payload_i32(&packet.payload, &targets_i32);
        let matches_i16 = search_payload_i16(&packet.payload, &targets_i16);
        let mut seen = HashSet::new();
        let matches: Vec<_> = matches_i32
            .into_iter()
            .map(|(o, v, l)| (o, v as u32, l))
            .chain(
                matches_i16
                    .into_iter()
                    .map(|(o, v, l)| (o, v as u32, l)),
            )
            .filter(|(o, v, _)| seen.insert((*o, *v)))
            .collect();
        if !matches.is_empty() {
            any_match = true;
            println!(
                "[{}] Packet 0x{:03X} ({} bytes)",
                packet.timestamp,
                packet.id,
                packet.payload.len()
            );
            for (offset, value, label) in matches {
                println!("  offset 0x{:04X}: {} ({})", offset, value, label);
            }
            println!();
        }
    }

    if !any_match {
        println!("No target values found in any packet.");
        println!("Targets: 7, 10, 2500, 9000, 500, 1000, 2000 (costs); 4182, 4181, 15542, 15541, 15761, 15762, 15763 (items)");
    }

    // Summary: packets in Harara window (zone-in ~14:19:48 through EVENTUCOFF ~14:20:05)
    println!("--- Packet ID summary (Harara window) ---");
    let harara_packets: Vec<_> = packets
        .iter()
        .filter(|p| {
            let t = p.timestamp.as_str();
            t >= "2026-02-24 14:19:48" && t <= "2026-02-24 14:20:06"
        })
        .collect();
    let mut by_id: std::collections::HashMap<u16, Vec<&str>> =
        std::collections::HashMap::new();
    for p in &harara_packets {
        by_id
            .entry(p.id)
            .or_default()
            .push(p.timestamp.as_str());
    }
    let mut ids: Vec<_> = by_id.keys().collect();
    ids.sort();
    for id in ids {
        let timestamps = by_id.get(id).unwrap();
        println!("  0x{:03X}: {} occurrence(s)", id, timestamps.len());
    }
}

struct Packet {
    timestamp: String,
    id: u16,
    payload: Vec<u8>,
}

fn parse_captain_log(content: &str) -> Vec<Packet> {
    let mut packets = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(bracket) = rest.find(']') {
                let timestamp = rest[..bracket].to_string();
                if let Some(pkt_part) = rest[bracket + 1..].trim().strip_prefix("Packet 0x") {
                    let hex_id: String = pkt_part
                        .chars()
                        .take_while(|c| c.is_ascii_hexdigit())
                        .collect();
                    let id = u16::from_str_radix(&hex_id, 16).unwrap_or(0);

                    let mut payload = Vec::new();
                    while let Some(next) = lines.peek() {
                        if next.starts_with('[') || next.is_empty() {
                            break;
                        }
                        let data_line = lines.next().unwrap();
                        if let Some(hex_part) = extract_hex_section(data_line) {
                            for token in hex_part.split_ascii_whitespace() {
                                if token != "--" && token.len() == 2 {
                                    if let Ok(byte) = u8::from_str_radix(token, 16) {
                                        payload.push(byte);
                                    }
                                }
                            }
                        }
                    }
                    packets.push(Packet {
                        timestamp,
                        id,
                        payload,
                    });
                }
            }
        }
    }
    packets
}

fn dump_packet(p: &Packet) {
    println!("[{}] Packet 0x{:03X} ({} bytes)", p.timestamp, p.id, p.payload.len());
    for (i, chunk) in p.payload.chunks(16).enumerate() {
        let hex: String = chunk.iter().map(|b| format!("{:02X} ", b)).collect();
        println!("  {:04X} | {}", i * 16, hex);
    }
    println!();
}

fn decode_conquest_packet(p: &Packet) {
    println!("[{}] Packet 0x05E GP_SERV_COMMAND_CONQUEST ({} bytes)\n", p.timestamp, p.payload.len());

    if p.payload.len() < 8 {
        println!("  (payload too short)");
        return;
    }

    println!("  Header (0x00-0x07):");
    println!("    {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
        p.payload[0], p.payload[1], p.payload[2], p.payload[3],
        p.payload[4], p.payload[5], p.payload[6], p.payload[7]);
    println!();

    if p.payload.len() >= 0xB0 {
        println!("  Cost block (0xA4-0xAF) — 3 x int32 LE (Chariot/Empress/Emperor band costs):");
        for i in 0..3 {
            let off = 0xA4 + i * 4;
            let v = u32::from_le_bytes([
                p.payload[off],
                p.payload[off + 1],
                p.payload[off + 2],
                p.payload[off + 3],
            ]);
            println!("    [{}] offset 0x{:04X}: {}", i, off, v);
        }
        println!();
    }

    println!("  Table region (0x20-0x6F) — 4-byte pairs (u16 LE each); layout TBD:");
    let table = &p.payload[0x20..p.payload.len().min(0x70)];
    for (i, chunk) in table.chunks(4).enumerate() {
        if chunk.len() >= 4 {
            let a = u16::from_le_bytes([chunk[0], chunk[1]]);
            let b = u16::from_le_bytes([chunk[2], chunk[3]]);
            let off = 0x20 + i * 4;
            println!("    [{}] 0x{:04X}: {} {}", i, off, a, b);
        }
    }
}

/// Search 0x05E payload for item IDs 15761, 15762, 15763 in various encodings.
fn search_conquest_packet_for_items(p: &Packet) {
    const ITEMS: [u32; 3] = [15761, 15762, 15763];
    let payload = &p.payload;

    println!("\n  Item ID search (15761, 15762, 15763) in alternate encodings:");
    for &item in &ITEMS {
        let le = item.to_le_bytes();
        let be = item.to_be_bytes();
        for (i, window) in payload.windows(4).enumerate() {
            if window == le {
                println!("    {} at 0x{:04X} (i32 LE)", item, i);
                break;
            }
            if window == be {
                println!("    {} at 0x{:04X} (i32 BE)", item, i);
                break;
            }
        }
        // 2-byte: item as u16 would overflow (15761 > 65535). Try as index?
        if item <= 65535 {
            let le16 = (item as u16).to_le_bytes();
            for (i, window) in payload.windows(2).enumerate() {
                if window == le16 {
                    println!("    {} at 0x{:04X} (u16 LE)", item, i);
                    break;
                }
            }
        }
    }
}

fn extract_hex_section(line: &str) -> Option<&str> {
    let first = line.find('|')?;
    let after_first = &line[first + 1..];
    let second = after_first.find('|')?;
    Some(after_first[..second].trim())
}

fn search_payload_i32<'a>(
    payload: &[u8],
    targets: &'a [(u32, &'a str)],
) -> Vec<(usize, u32, &'a str)> {
    let mut matches = Vec::new();
    for (value, label) in targets.iter().copied() {
        let bytes = value.to_le_bytes();
        for (i, window) in payload.windows(4).enumerate() {
            if window == bytes {
                matches.push((i, value, label));
            }
        }
    }
    matches.sort_by_key(|m| m.0);
    matches
}

fn search_payload_i16<'a>(
    payload: &[u8],
    targets: &'a [(u16, &'a str)],
) -> Vec<(usize, u16, &'a str)> {
    let mut matches = Vec::new();
    for (value, label) in targets.iter().copied() {
        let bytes = value.to_le_bytes();
        for (i, window) in payload.windows(2).enumerate() {
            if window == bytes {
                matches.push((i, value, label));
            }
        }
    }
    matches.sort_by_key(|m| m.0);
    matches
}
