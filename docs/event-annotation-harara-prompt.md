# Harara Conquest Bands Annotation — Solver Prompt

## Goal

Make `test_harara_conquest_bands_items_and_costs_annotated` pass. The test expects items 15761, 15762, 15763 (Chariot/Empress/Emperor bands) and costs 500, 1000, 2000 in Harara's `immed_data` to be annotated with their context (item names and cost labels).

## Constraints

1. **No hardcoded values** — Do not special-case item IDs (15761, 15762, 15763) or costs (500, 1000, 2000). The solution must be programmatic and general.
2. **No capture reliance** — Do not read Captain logs or `XI_CAPTURE_PATH` at runtime. Captures are for reverse engineering only. Use them to understand opcode behavior, then implement based on that understanding.
3. **Opcode tracing** — Annotation must flow from structural analysis of the event DAT (opcodes, register flow, dialog templates) or from item/cost lookups in DAT files (e.g. armor DAT for item names).

## Current State

- **Harara** (actor 17764543): Has `immed_data` with 15761, 15762, 15763 at indices 40, 45, 47 and 500, 1000, 2000 at indices 351, 352, 353. Has no 0xD4 0x04/0x05 opcodes (parsed or in `data_section`).
- **Block 239** (actor 17764608): Has 0xD4 0x04/0x05 and 6 d4_pairs, but different `immed_data` layout (no 15761). Uses Work_Zone_1700(0..5).
- **Zone-wide tracing**: Implemented. `build_global_ref_maps` merges 0x03 copies across blocks (first-write-wins). `extract_d4_item_cost_pairs` scans opcodes and `data_section` for 0xD4 0x04/0x05.
- **0x05E conquest packet** (from capture analysis): Cost block at 0xA4-0xAF (3× int32 LE) in order Chariot, Empress, Emperor. Use this only as reference to understand the runtime structure — do not parse captures at runtime.

## What to Investigate

1. **Why Harara has no 0xD4 0x04/0x05** — Is the conquest menu driven by a different block? Does Harara share event logic with block 239? Is there another opcode that reads these values?
2. **Cross-block value resolution** — When block A writes Ref[40]→WZ1700[0] and block B reads WZ1700[0], we use ref_idx 40. Block B's `immed_data[40]` may differ. Consider value-based resolution: given value X in WZ1700 slot, find index in current block where `immed_data[idx] == X`.
3. **Alternative opcodes** — Grep XiEvents for `getworkofs`; conquest bands might use opcodes we don't yet trace.
4. **Execution flow** — Which block runs when the player talks to Harara? Does it trigger block 239's event? Understanding the event trigger chain may reveal the data flow.

## Key Files

| File | Purpose |
|------|---------|
| `crates/processor/src/enrichment.rs` | Annotation pipeline, opcode tracing, `enrich_from_dialog_context` |
| `crates/dats/src/formats/event.rs` | Event DAT parser, opcode parsing |
| `docs/event-annotation-debugging.md` | Debugging guide, 0x05E layout, capture analysis |
| `docs/event-annotation-status.md` | Status, known limitations, Addendum A (conquest items) |
| `crates/processor/examples/parse_capture.rs` | Captain log parser — use as reference only |

## Test

```bash
cargo test -p processor test_harara_conquest_bands_items_and_costs_annotated
```

Run `test_harara_debug_dump` with `--nocapture` to inspect Harara's structure.
