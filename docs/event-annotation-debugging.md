# Event Annotation Debugging Guide

This document describes how to debug the relationship between event opcodes and `immed_data` annotations, what information we're missing, and how to capture it.

---

## 1. The Problem

The Harara conquest band test expects items 15761, 15762, 15763 and costs 500, 1000, 2000 to be annotated. **Constraints**: No hardcoded values, no capture reliance (capture is for reverse engineering only). See [event-annotation-harara-prompt.md](event-annotation-harara-prompt.md) for the solver prompt.

---

## 2. What We're Missing

| Gap | Description | How to Capture |
|-----|-------------|----------------|
| **Cross-block data flow** | Work_Zone and Work_Zone_1700 are zone-wide. Block A may write References→WZ1700, Block B may read WZ1700 in 0xD4. We only trace within a single block. | Trace 0x03 and 0xD4 across *all* blocks in the zone, build zone-wide ref maps |
| **Harara has no 0xD4 0x04/0x05** | Debug dump shows Harara block has 0 D4 opcodes in parsed OpcodesWithData. Conquest bands (indices 40, 45, 47, 351, 352, 353) may be loaded by another block or different opcode. | Run `test_harara_debug_dump`; consider zone-wide tracing |
| **Param index fix** | 0xD4 params: item1 at params[3..4], cost1 at params[5..6], etc. (params[0]=subcase, params[1..2]=val1). Fixed in extract_d4_item_cost_pairs. | Verify with blocks that have 0xD4 0x04/0x05 |
| **Execution order** | Opcodes execute in sequence. A 0x03 in series N may populate WZ1700 before 0xD4 in series M reads it. We don't model execution order. | Document series order (TagOffset) and which series contain 0x03 vs 0xD4 |
| **indexShift parameter** | getworkofs(index, indexShift) uses `val = indexShift + eventgetcode(index)`. We assume indexShift=0. Some opcodes may pass non-zero. | Audit XiEvents pseudo-code for getworkofs call sites |
| **Alternative opcodes** | Conquest bands might be loaded via 0x24 (dialog), 0x40/0x41 (bit flags), or other getworkofs-using opcodes we don't scan | Grep XiEvents for `getworkofs`; add those opcodes to `all_referenced` |

---

## 3. Debugging Strategies

### 3.1 Run the Debug Dump Test

```bash
cargo test -p processor test_harara_debug_dump -- --nocapture
```

This prints:
- Harara block structure (series count, ParsedData types)
- All 0xD4 0x04/0x05 opcodes with raw bytes and decode_register results
- All 0x03 copies found (dest, src, decoded)
- Built ref_to_wz1700, ref_to_wz, ref_to_wzm maps
- d4_pairs extracted
- immed_data indices of 15761, 15762, 15763, 500, 1000, 2000

### 3.2 Cross-Block Tracing (Proposed)

If Work_Zone_1700 is populated by another block:

1. Load the full Event (all blocks)
2. Build `ref_to_wz1700` from 0x03 across *all* blocks
3. When processing Harara's 0xD4, use this zone-wide map
4. Harara's 0xD4 may read WZ1700(slot) that was written by block 239

### 3.3 XiEvents Opcode Audit

From the XiEvents repo (sibling directory):

```bash
cd ../XiEvents
rg "getworkofs" OpCodes/ --no-heading -l
```

For each opcode that uses getworkofs with References, note:
- Param byte indices (offset from opcode start)
- Whether indexShift is used
- Whether it reads item/cost-like values

### 3.4 Raw Byte Inspection

For RawBytes series (e.g. Harara's 32759):

1. Dump the raw bytes to a file
2. Search for `D4 04` and `D4 05` patterns
3. Manually decode each 2-byte param pair
4. Compare with expected immed_data indices (40, 41, 42, 43, 44, 45)

### 3.5 Compare with Working Block (239)

Block 239 has 0xD4 with Work_Zone(6) and Work_Zone_1700(0..5) that we understand. Compare:

- Block 239's immed_data layout vs Harara's
- Block 239's 0x03 copies vs Harara's
- Whether conquest bands in Harara could come from block 239's WZ1700 writes

---

## 4. XiPackets: Server Populates Work_Zone

**Packet 0x005C** (`GP_SERV_COMMAND_PENDINGNUM`) — XiPackets documents this as populating Work_Zone[2..9]:

- **XiPackets**: [world/server/0x005C](https://github.com/atom0s/XiPackets/blob/main/world/server/0x005C/README.md)
- **Payload**: `int32_t num[8]` — 8 event parameter values
- **Client behavior**: Copies into `PTR_Work_Zone` starting at index **2**

**However**, 0x005C was **not observed** in a capture that included zone-in → talk to Harara → open conquest menu → scroll → exit.

**Packet 0x005E** (`GP_SERV_COMMAND_CONQUEST`) — **This is the conquest menu packet**:

- **XiPackets**: [world/server/0x005E](https://github.com/atom0s/XiPackets/blob/main/world/server/0x005E/README.md)
- **Client handler**: `RecvConquest`
- **Observed**: Sent when opening Harara's conquest menu
- **Payload layout** (from capture analysis):
  - `0x00-0x07`: Header (id, size, sync + 4 bytes)
  - `0x08-0x1F`: Zeros
  - `0x20-0x6F`: Structured table (likely item/cost pairs; format TBD)
  - `0xA4-0xAF`: Three int32 values — **interpretation unvalidated**. Previously assumed to be Chariot/Empress/Emperor band costs, but a capture with the common items menu open (showing 500, 1000, 2000 for bands) had 2812 at these offsets, which does not match. May be a different field.

**To validate packet layout**: Run `parse_capture` in search mode on a capture from: talk to Harara → spend conquest points → common items → view bands. The tool searches for 7, 10, 2500, 9000, 500, 1000, 2000 (costs) and 4182, 4181, 15542, 15541, 15761, 15762, 15763 (items). Use `--decode 0x05E` to dump the 0x005E packet structure.

**Packet 0x005D** populates event *strings* (4×16-byte blocks into `PTR_EventStrings`). Its `num[9]` array is **ignored** by the client.

---

## 5. Runtime Capture for Debugging

Use **Captain** (Windower/Ashita addon) to capture packets when opening Harara's conquest menu:

1. Install [Captain](https://github.com/zach2good/captain) in Windower addons
2. `//lua load captain` then `/captain start`
3. Zone to Windurst Woods, talk to Harara, open conquest bands menu
4. `/captain stop`
5. Inspect `captures/<timestamp>/<char>/packetviewer/incoming.log` for **0x005E** (conquest) packets

Use `parse_capture` to search and decode:
```bash
# With explicit path:
cargo run -p processor --example parse_capture -- <path/to/incoming.log>
cargo run -p processor --example parse_capture -- --decode 0x05E <path>

# Auto-discover from Windower/Ashita captain captures, or set XI_CAPTURE_PATH:
cargo run -p processor --example parse_capture --
```

**Other packet tools**:
- **PacketViewer** (Arcon)
- **Packeteer** (atom0s)
- **capture** (ibm2431)

---

## 6. Capture Analysis Plan (Harara Conquest Menu)

Since **0x005C was not observed** in the capture (which included zone-in through menu exit), the conquest band data likely uses a different packet. This plan structures how to analyze the capture and infer the correct parsing.

### 6.1 Goals

1. **Identify** which packet(s) carry item IDs (15761, 15762, 15763) and costs (500, 1000, 2000).
2. **Map** payload layout to Work_Zone/Work_Zone_1700 slots that 0xD4 0x04/0x05 reads (getworkofs 2, 4, 6, 8, 10).
3. **Decide** whether we can annotate Harara from DAT alone, or need runtime/packet-derived data.

### 6.2 Phase 1: Parse and Catalog the Capture

1. **Parse the log** into structured records: `{ timestamp, packet_id, raw_hex }`.
2. **Define the Harara window**: first packet after zone-in (14:19:48) through last packet before 0x052 EVENTUCOFF (14:20:05).
3. **List all unique packet IDs** in that window with counts and sizes.

### 6.3 Phase 2: Search for Known Values

Search all payload bytes (after 4-byte header) for little-endian int32:

| Value | Hex (LE) | Meaning |
|-------|----------|---------|
| 500 | `F4 01 00 00` | Rank 1 Chariot cost |
| 1000 | `E8 03 00 00` | Rank 1 Empress cost |
| 2000 | `D0 07 00 00` | Rank 1 Emperor cost |
| 15761 | `D1 3D 00 00` | Chariot Band item ID |
| 15762 | `D2 3D 00 00` | Empress Band item ID |
| 15763 | `D3 3D 00 00` | Emperor Band item ID |

**Note**: If the capture used a different rank, use that rank’s costs (e.g. Rank 2: 1000, 2000, 4000). The 0x05E packet had `FC 0A 00 00` (2812) — record the player’s conquest rank for interpretation.

### 6.4 Phase 3: Candidate Packet Analysis

For each packet containing one or more target values:

1. **Document**: packet ID, total size, offset(s) where values appear.
2. **Infer layout**: e.g. `[count][item1][cost1][item2][cost2][item3][cost3]` or similar.
3. **Check XiPackets**: look up the packet ID in the XiPackets repo for a documented structure.
4. **Map to 0xD4**: 0xD4 0x04/0x05 reads getworkofs(2), (4), (6), (8), (10). Determine which payload bytes would map to which Work_Zone slot.

### 6.5 Phase 4: Cross-Validate

1. **Second capture**: Run with a different conquest rank (or different nation’s conquest NPC) and confirm the same packet/offsets change as expected.
2. **Outgoing correlation**: Check client→server packets (0x05A, 0x05B, etc.) when opening the menu; the server may send the conquest data in response to a specific request.

### 6.6 Phase 5: Annotation Strategy

- **If packet found**: Document the format. If it’s not 0x005C, update the debugging doc and note that conquest menus use a different mechanism. Static annotation from DAT alone may remain impossible for server-populated values.
- **If packet not found**: Values may be in a sub-packet, chunked, or use a non-standard encoding. Consider a hex-dump tool that searches across packet boundaries or a Windower addon that logs packet payloads with a custom filter.

### 6.7 Implementation: Capture Parser

The `parse_capture` example parses Captain logs and searches for conquest band values:

```bash
cargo run -p processor --example parse_capture -- <path/to/incoming.log>
cargo run -p processor --example parse_capture -- --dump 0x05E <path>
cargo run -p processor --example parse_capture -- --decode 0x05E <path>
```

- **Search mode**: Finds 500, 1000, 2000, 15761, 15762, 15763, 2812 as i32/i16 (little-endian).
- **--dump 0xID**: Dumps raw hex of all packets with the given ID.
- **--decode 0x05E**: Decodes conquest packet (0x005E) layout: header, table region, cost block at 0xA0.

---

## 7. Key XiEvents References

- [Event VM Functions](https://github.com/atom0s/XiEvents/blob/main/Event%20VM%20Functions.md) — getworkofs, eventgetcode, register ranges
- [OpCodes/0x00D4](https://github.com/atom0s/XiEvents/blob/main/OpCodes/0x00D4.md) — conquest/selection menu opcode
- [Event DAT Structures](https://github.com/atom0s/XiEvents/blob/main/Event%20DAT%20Structures.md) — block layout, ImmedData

---

## 8. Debug Test: `test_harara_debug_dump`

The test `test_harara_debug_dump` in `enrichment.rs` dumps the above information. Run with `--nocapture` to see output. Use this to:

1. Verify which series are RawBytes vs Opcodes
2. See exact byte values for each 0xD4 param
3. Confirm decode_register results
4. Check if d4_pairs is empty and why
