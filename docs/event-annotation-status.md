# FFXI Event Annotation — Current Status & Architecture

This document summarizes the work done on event `immed_data` annotation in xi-tinkerer,
our current understanding of the FFXI event VM, what is and isn't being annotated, and
the reasoning behind each decision. It is intended as the baseline for the next phase.

---

## 1. Background: Event DAT Structure

Each zone has an Event DAT containing bytecode for the client's internal virtual machine.
The structure is documented by [atom0s](https://github.com/atom0s/XiEvents/blob/main/Event%20DAT%20Structures.md):

```
eventheader_t        – block count + block sizes
eventblock_t[]       – one per NPC/entity:
    ActorNumber      – entity server ID (0x7FFFFFFF = player/zone events)
    TagOffset[]      – byte offsets into EventData for each event series
    EventExecNum[]   – event IDs corresponding to each TagOffset
    ImmedData[]      – reference table of 32-bit values (the "References" array)
    EventData        – bytecode blob (VM opcodes + inline data sections)
```

The **ImmedData / References table** is the annotation target. It contains item IDs,
dialog IDs, entity references, costs, quantities, and other constants used by opcodes at
runtime. Our goal is to annotate each entry with what it represents (e.g. item name,
cost label, entity reference) when we can determine that with certainty.

---

## 2. The Event VM Register Model

The VM uses a 2-byte little-endian addressing scheme (documented by atom0s as
[`getworkofs`](https://github.com/atom0s/XiEvents/blob/main/Event%20VM%20Functions.md)).
Given bytes `[lo, hi]`, the address `val = lo + (hi << 8)` maps to:

| Range                  | Register              | Scope          |
|------------------------|-----------------------|----------------|
| `val < 80`             | `WorkLocal[val]`      | Per-event      |
| `0x1000..0x105F`       | `Work_Zone[val-0x1000]` | Zone-wide    |
| `0x1100..0x115F`       | `Work_Zone_Memorize[val-0x1100]` | Zone-wide (persistent) |
| `0x1700..0x175F`       | `Work_Zone_1700[val-0x1700]` | Zone-wide  |
| `val & 0x8000`         | `References[val & 0x7FFF]` | Read-only, per-block |

**WorkLocal** is the register set that dialog template substitutions read from. When a
dialog template contains `${item-singular: 3}` or `${number: 4}`, it reads
`WorkLocal[3]` and `WorkLocal[4]` respectively.

**References** is the `immed_data` array — the read-only constant table.

**Work_Zone, Work_Zone_Memorize, Work_Zone_1700** are zone-wide shared arrays that
persist across events and are writable by both client events and server packets.

---

## 3. Opcode 0x03: The Core Data-Flow Primitive

[Opcode 0x03](https://github.com/atom0s/XiEvents/blob/main/OpCodes/0x0003.md)
("Gets a value then stores it") is the primary copy instruction. It reads a value from
one register and writes it to another. The 5-byte encoding is:

```
03 [dest_lo] [dest_hi] [src_lo] [src_hi]
```

This is the only opcode we trace for data-flow analysis. It enables us to follow how
values from `References` (immed_data) reach `WorkLocal` (dialog parameters).

### Direct mapping (simple case)

```
03 [wl_lo] 00 [ref_lo] 80    →  WorkLocal[wl_lo] = References[ref_lo]
```

The dialog template reads `WorkLocal[wl_lo]`, so we know `immed_data[ref_lo]` is the
value being displayed with the template's semantic context.

### Two-stage mapping (conquest/selection menus)

Many conquest NPCs and selection-line menus use an indirect pattern:

```
Stage 1:  03 [wz1700_lo] 17 [ref_lo] 80    →  Work_Zone_1700[wz1700_lo] = References[ref_lo]
Stage 2:  03 [wl_lo]     00 [wz1700_lo] 17  →  WorkLocal[wl_lo] = Work_Zone_1700[wz1700_lo]
```

These stages can occur in different event series within the same block. We compose them:
`References[ref_lo] → Work_Zone_1700[wz1700_lo] → WorkLocal[wl_lo]`.

The same pattern applies through `Work_Zone` (hi=0x10) and `Work_Zone_Memorize`
(hi=0x11).

---

## 4. Dialog Template Analysis

Zone dialog DATs contain template strings with substitution markers:

```
${item-singular: 0}     →  reads WorkLocal[0] as an item ID
${number: 1}            →  reads WorkLocal[1] as a numeric display value
```

We parse these templates to build a `DialogLayout`:
- **item_indices**: which WorkLocal slots hold item IDs
- **number_to_item**: maps each number slot to its paired item slot, plus semantic
  context inferred from surrounding text ("pts"/"gil" → cost, "minute" → time, etc.)

The dialog layout combined with opcode tracing lets us determine:
- Which `immed_data` indices are items (annotated with item name from DAT lookups)
- Which are costs/quantities (annotated with "cost", "time", etc.)

---

## 5. Annotation Pipeline (enrichment.rs)

The pipeline runs a single pass per block: `enrich_from_dialog_context`

Traces opcodes and uses dialog templates to annotate with certainty:

1. **Phase 1 — Paired item+cost**: When both the item slot and number slot have traced
   `References` mappings, annotate both (e.g. "Fire Crystal (cost)").

2. **Phase 2 — Standalone cost**: When only the number slot has a mapping and the dialog
   context says "cost" (text contains "pts", "gil", etc.), annotate the cost value even
   though the paired item is server-populated.

3. **Phase 2b — Standalone item (server-priced)**: When only the item slot has a mapping
   and the cost is server-defined, annotate the item with "(price is server-defined)".

4. **Phase 3 — Unpaired items**: Item-slot indices that weren't already annotated in
   Phase 1 or 2b.

---

## 6. What IS Being Annotated

- **Items with direct WorkLocal mapping**: When opcode 0x03 traces
  `References[idx] → WorkLocal[item_slot]` and the dialog template tags that slot as an
  item, and the value matches a known item ID.

- **Costs paired with items**: When both item and number slots trace to `References`
  indices, annotated as "Item Name (cost)".

- **Standalone costs (server items)**: When only the cost slot traces but dialog text
  contains cost keywords ("pts", "gil", "points"). Annotated as
  "cost (item set by server)".

- **Items with server-defined prices**: When only the item slot traces and its pair is a
  cost context. Annotated as "Item Name (price is server-defined)".

---

## 7. What Is NOT Being Annotated (and Why)

### 7a. "Dark" indices in WZ1700 context blocks

**Problem**: In blocks that use Work_Zone_1700 for mixed item+cost storage (conquest
menus, Harara, etc.), some `References` indices are never accessed by any traced opcode.
These "dark" indices could be items or costs — we have no structural evidence either way.

**Principle**: We only annotate when we have certainty. If an index has no traceable
register access, we cannot determine its role, so we leave it unannotated.

**What's lost**: Some legitimate items (e.g. Instant Reraise scroll, 4246) that happen
to be dark indices in WZ1700 blocks don't get annotated. This is a conscious trade-off
for zero false positives.

**Potential fix**: Deeper opcode tracing (see §10) might reveal accesses to these indices
through opcodes we don't currently trace, or through indirect addressing patterns.

### 7b. Server-populated values (runtime injection)

**Problem**: Values populated by the server at runtime via event update packets (e.g.
conquest point costs for Chariot/Empress/Emperor bands: 500, 1000, 2000) never appear
in the event DAT's opcode stream. There is no `References → Work_Zone_1700` copy
instruction for them — the server writes directly into Work_Zone registers.

**Packet mechanism**: Capture analysis shows **0x005E** (`GP_SERV_COMMAND_CONQUEST`) — the conquest packet. Costs appear at offsets 0xA4–0xAF (3× int32). 0x005C was not observed. See [event-annotation-debugging.md](event-annotation-debugging.md) for `parse_capture` and `--decode 0x05E`.

**Why unsolvable statically**: These values are determined by server-side game logic
(player rank, ongoing campaigns, etc.) and have no static representation in the DAT.

**Examples**: Conquest rank-specific costs, seasonal event pricing, dynamically stocked
vendor items.

### 7c. Standard merchant/guild vendor inventories

**Problem**: Standard merchants (weapon shops, armor shops) and guild vendors use the
shop window system (opcodes like 0x3C) rather than dialog templates. Their inventories
and prices are entirely server-driven.

**Why not traced**: These NPCs don't use `${item-singular: N}` / `${number: N}` dialog
substitution patterns. The server sends shop contents via game packets, and the client
opens a native shop UI. There's no item or price data in the event DAT to annotate.

### 7d. Conquest rank costs stored ÷10

**Problem**: Some conquest costs appear to be stored divided by 10 in the DAT (e.g.
5600 in immed_data = 56,000 CP displayed in-game). The annotation shows the raw stored
value.

**Validated**: Harara's immed_data contains 100, 200, 400, 800, 1600, 2400, 3200, 4000,
4800, 5600 — matching Addendum A.2 (Rank 1–10 CP costs ÷10). Common items (Return Ring
2,500, Ornate Stool 10,000, Nation Flags 90,000) are stored at full value. The ÷10
pattern applies only to rank-specific conquest items.

**Why we show raw**: This is a display-layer transformation applied by the client or
dialog template (possibly via `${number: N}` formatting). We don't currently model
display transformations.

### 7e. Sentinel / default values in cost slots

**Problem**: When a WorkLocal cost slot traces to a References index that holds a value
like 255 (0xFF) or 1, it gets annotated as "cost (item set by server)" even though it's
likely a sentinel or default value rather than an actual price.

**Why**: We can confirm with certainty that the index serves a cost role (the dialog
template says so, the opcode trace confirms it). The annotation is technically correct —
the value *is* in a cost slot — but the value itself may not be a meaningful price.

---

## 10. Known Limitations & Future Work

### 10a. Opcode 0x03 is the only data-flow opcode traced

We only trace copy instructions (0x03) for building the
`References → WorkLocal` mapping. Other opcodes that might load values into WorkLocal
(e.g. arithmetic results, conditional moves) are not followed. This means some
legitimate data flows are invisible to us.

**atom0s reference**: The full opcode list (0x00–0xD9) is documented at
https://github.com/atom0s/XiEvents/tree/main/OpCodes

### 10b. Within-series overwrites

A single event series may write to the same WorkLocal or Work_Zone_1700 slot multiple
times (e.g. loading different menu pages). Our per-series tracking uses last-write-wins,
which captures the final page but misses earlier ones.

### 10c. Raw byte scanning for unparsed series

When event series are stored as `RawBytes` (unparsed bytecode), we scan for the 0x03
byte pattern heuristically. This could produce false positives if 0x03 appears as a
parameter of another opcode, though the 5-byte constraint makes this rare in practice.

### 10d. Cross-block tracking (implemented)

**Implemented**: Zone-wide 0x03/0xD4 tracing. `enrich_event_blocks` builds ref maps from
*all* blocks in the zone (first-write-wins) and precomputes d4_pairs per block. This
recovers mappings when block A populates Work_Zone_1700 and block B reads it in 0xD4.

**Limitation**: Cross-block resolution uses ref indices. When block A writes Ref[40]→WZ1700[0]
and block B reads WZ1700[0], we use ref_idx 40. If block B has different immed_data layout,
block_b.immed_data[40] may not match. Value-based resolution (find index where value=X) is
not yet implemented.

### 10e. Dialog template parsing limitations

- Only `${item-singular}`, `${keyitem-singular}`, `${item-article}`,
  `${keyitem-article}`, `${item-given-plurality}` are recognized as item markers.
- Only `${number}` is recognized as a numeric parameter.
- Other substitution types (`${string}`, `${flag}`, custom markers) are ignored.
- Context inference is keyword-based ("pts", "gil", "minute"), which may miss
  non-English or unusual phrasing.

### 10f. No opcode-level semantic analysis

We don't analyze what happens to values after they reach WorkLocal. For example, if
WorkLocal[3] is later compared against a threshold (opcode 0x02), that comparison could
tell us something about the value's nature (flag vs. quantity vs. ID). This would
require building a per-slot value-flow graph, which is significantly more complex.

---

## 11. Reliable Annotation Strategy

### When DAT tracing works

1. **Direct 0x03 → WorkLocal**: Block has `References[idx] → WorkLocal[slot]` and dialog template tags that slot. Annotate both item and cost when paired.

2. **Two-stage 0x03 → WZ1700 → WorkLocal**: Block (or another block in zone) has `References → Work_Zone_1700` and `Work_Zone_1700 → WorkLocal`. Zone-wide tracing merges these. Annotate when 0xD4 0x04/0x05 reads the same slots.

3. **0xD4 0x04/0x05 in same block**: Block has conquest/selection menu opcode. We scan parsed opcodes and `data_section`/RawBytes. Resolve item/cost via ref maps. Annotate pairs.

4. **Cross-block with same immed_data layout**: Block A populates WZ1700, block B reads it. If both share the same immed_data layout (ref_idx valid for both), annotation works.


### When DAT tracing fails

1. **No 0xD4 0x04/0x05 in block**: Block has conquest data in immed_data but no opcode that reads it. Cannot trace.

2. **Server-populated values**: Packet 0x05E (conquest) or other server packets write directly to Work_Zone. No References copy in DAT.

3. **Different immed_data layouts**: Block A writes Ref[40]→WZ1700[0], block B reads WZ1700[0]. Block B's immed_data[40] may differ. Value-based resolution not implemented.

4. **Unparsed opcodes**: Series falls back to RawBytes; we scan for 0x03 and 0xD4 patterns. If conquest uses a different opcode, we miss it.

### Harara conquest bands (open)

Items 15761, 15762, 15763 and costs 500, 1000, 2000 in Harara's immed_data must be annotated via opcode tracing. No hardcoded values, no capture reliance. See [event-annotation-harara-prompt.md](event-annotation-harara-prompt.md) for the solver prompt.

---

## 12. Core Principle

> **We only annotate when we have structural certainty.** If we cannot deterministically
> prove — through opcode tracing or direct database matching — that a value is an item,
> cost, entity reference, or other known type, we leave it unannotated. Plausibility
> heuristics (e.g. "divisible by 50 so probably a cost") were explicitly rejected in
> favor of this approach.

---

## 13. Key Source Files

| File | Purpose |
|------|---------|
| `crates/processor/src/enrichment.rs` | Annotation pipeline: opcode tracing, dialog layout parsing, item/cost/entity annotation |
| `crates/processor/src/converters.rs` | Export converter: triggers enrichment during DAT→YAML export |
| `crates/dats/src/id_mapping.rs` | Dat ID ranges for events; export routing uses these |
| `crates/dats/src/formats/events.rs` | Event DAT parser: block/series/opcode structures, ImmedValue type |
| `crates/dats/src/formats/opcode_descriptions.rs` | Opcode size determination and descriptions |
| `crates/dats/src/formats/dialog.rs` | Dialog DAT parser: template strings with substitution markers |

---

## 14. External References

- **atom0s/XiEvents**: https://github.com/atom0s/XiEvents
  - [Event DAT Structures](https://github.com/atom0s/XiEvents/blob/main/Event%20DAT%20Structures.md)
  - [Event VM Functions](https://github.com/atom0s/XiEvents/blob/main/Event%20VM%20Functions.md) (getworkofs, GetActorIndex)
  - [Opcode Reference](https://github.com/atom0s/XiEvents/tree/main/OpCodes) (0x00–0xD9)
  - [Opcode 0x03](https://github.com/atom0s/XiEvents/blob/main/OpCodes/0x0003.md) (copy/store — our primary traced opcode)

---

## Addendum A: Windurst Conquest Point Items (Ground Truth)

Reference data for validating annotations against Harara, W.W. (Windurst Woods K-10)
and all Windurst War Warlocks. Source: ffxiclopedia / BG Wiki.

The same conquest block is shared across all three nations' War Warlocks. Harara's
`immed_data` contains items from San d'Oria (Royal/Knight/Guard lines), Bastok
(Iron Musketeer/Legionnaire/Musketeer Commander lines), and Windurst
(Freesword/Mercenary/Combat Caster/Tactician Magician/Wise Wizard/Master Caster lines).

### A.1 Common Items (All Ranks)

| Item ID | Item Name              | CP Cost | Notes |
|---------|------------------------|---------|-------|
| 4182    | Instant Reraise        | 7       | |
| 4181    | Instant Warp           | 10      | |
| 15542   | Return Ring            | 2,500   | |
| 15541   | Homing Ring            | 9,000   | |
| 17585   | Fed. Signet Staff      | 5,000   | Windurst citizen, Rank 10 |
| 15763   | Chariot Band           | 500     | |
| 15763   | Empress Band           | 1,000   | Same item ID, different cost |
| 15763   | Emperor Band           | 2,000   | Same item ID, different cost |
| 28540   | Warp Ring              | 5,000   | |
| 10114   | Cipher: Tenzen         | 1,000   | Extravaganza set A |
| 10139   | Cipher: Rahal          | 1,000   | Extravaganza set B |
| 6379    | Ornate Stool           | 10,000  | Windurst citizen, Rank 10 |
| 6380    | Refined Chair          | 20,000  | Rank 10 (any nation) |
| 10146   | Cipher: Kukki          | 1,000   | Extravaganza set A |
| 10180   | Cipher: Makki          | 1,000   | Extravaganza set B |
| 183     | Windurstian Flag       | 90,000  | Windurst citizen, Rank 10 |

Also in block for other nations:
| 17583   | Kgd. Signet Staff      | 5,000   | San d'Orian citizen, Rank 10 |
| 17584   | Rep. Signet Staff      | 5,000   | Bastokan citizen, Rank 10 |
| 181     | San d'Orian Flag       | 90,000  | San d'Orian citizen, Rank 10 |
| 182     | Bastokan Flag          | 90,000  | Bastokan citizen, Rank 10 |

### A.2 Windurst Rank Items

Costs are stored ÷10 in the DAT. E.g. Rank 1 = 1,000 CP → stored as 100.

| Rank | CP Cost | DAT Value | Windurst Items |
|------|---------|-----------|----------------|
| 1    | 1,000   | 100       | 17159 Freesword's Bow, 17028 Freesword's Club, 16442 Fsd. Baghnakhs, 12915 Freesword's Slops (1st/2nd), 17130 Freesword's Staff (1st) |
| 2    | 2,000   | 200       | 17103 Mercenary's Pole, 12484 Mrc. Hachimaki, 12653 Mercenary's Gi, 12719 Mrc. Tekko, 12855 Mrc. Sitabaki, 12975 Mrc. Kyahan, 16746 Mrc. Knife (1st/2nd), 16930 Mrc. Greatsword (1st) |
| 3    | 4,000   | 400       | 16776 Mrc.Cpt. Scythe, 12470 Mrc.Cpt. Headgear, 12598 Mrc.Cpt. Doublet, 12726 Mrc.Cpt. Gloves, 12854 Mrc.Cpt. Hose, 12982 Mrc.Cpt. Gaiters, 16747 Mrc.Cpt. Kukri (1st/2nd), 13221 Mrc.Cpt. Belt (1st), 13496 Windurstian Ring (1st) |
| 4    | 8,000   | 800       | 16463 Cmb.Cst. Dagger, 17282 Cmb.Cst. B'merang, 13101 Green Scarf, 12614 Cmb.Cst. Cloak, 12743 Cmb.Cst. Mitts, 12870 Cmb.Cst. Slacks, 12998 Cmb.Cst. Shoes, 16807 Cmb.Cst. Scimitar (1st/2nd), 16669 Cmb.Cst. Axe (1st) |
| 5    | 16,000  | 1600      | 17082 Tct.Mag. Wand, 13102 Paisley Scarf, 12478 Tct.Mgc. Hat, 12606 Tct.Mgc. Coat, 12734 Tct.Mgc. Cuffs, 12862 Tct.Mgc. Slops, 12990 Tct.Mgc. Pigaches, 16810 Tct.Mag. Espadon (1st/2nd), 16694 Tct.Mag. Hooks (1st) |
| 6    | 24,000  | 2400      | 13103 Checkered Scarf, 13581 Fed. Army Mantle, 17094 Wis.Wiz. Staff (1st/2nd), 16808 Wis.Wiz. Bilbo (1st/2nd), 16809 Wis.Wiz. Anelace (1st) |
| 7    | 32,000  | 3200      | 15958 Cmb.Cst. Quiver, 12363 Ptr.Prt. Shield (1st/2nd), 13559 Ptr.Prt. Ring (1st) |
| 8    | 40,000  | 4000      | 14016 Mst.Cst. Mitts, 14017 Mst.Cst. Bracelets (1st/2nd), 13142 Windurstian Scarf (1st) |
| 9    | 48,000  | 4800      | 18145 Mst.Cst. Bow, 17530 Mst.Cst. Pole, 17508 Mst.Cst. Baghnakhs (1st/2nd), 17617 Mst.Cst. Knife (1st) |
| 10   | 56,000  | 5600      | 14430 Federation Aketon (1st) |

### A.3 Harara Block Annotation Status

Harara's event block (`actor_number: 17764543`) contains 354 `immed_data` entries.
The block uses Work_Zone_1700 for mixed item+cost storage (`has_wz1700_cost_context`
= true), which triggers the "dark index" filtering rule.

**Currently annotated** (have `# Name` or `# cost` comments):

| Category | Count | Examples |
|----------|-------|---------|
| Items (dialog-traced or item_map match) | ~75 | Ryl.Arc. Longbow, Freesword's Bow, Mrc.Cpt. Scythe, Cmb.Cst. Dagger, Federation Aketon |
| Costs (Phase 2: dialog-traced) | 1 | 5600 → "cost (item set by server)" |
| Entity references | 1 | 0x7FFFFFFF → "sentinel: max int32" |

**Not annotated — items (dark indices):**

These are confirmed item IDs that exist in the item database but are not referenced by
any traced opcode, so they fail the `all_referenced` check and are skipped:

| Item ID | Item Name | Why unannotated |
|---------|-----------|-----------------|
| 16544   | Ryl.Arc. Sword       | Dark index in WZ1700 block |
| 12753   | Ryl.Ftm. Gloves      | Dark index |
| 13004   | Ryl.Ftm. Boots       | Dark index |
| 16691   | Ryl.Arc. Cesti       | Dark index |
| 17223   | Lgn. Crossbow        | Dark index |
| 12509   | Legionnaire's Cap    | Dark index |
| 12752   | Lgn. Mittens         | Dark index |
| 13003   | Lgn. Leggings        | Dark index |
| 17028   | Freesword's Club     | Dark index |
| 12915   | Freesword's Slops    | Dark index |
| 17130   | Freesword's Staff    | Dark index |
| 12484   | Mrc. Hachimaki       | Dark index |
| 12719   | Mrc. Tekko           | Dark index |
| 12855   | Mrc. Sitabaki        | Dark index |
| 12975   | Mrc. Kyahan          | Dark index |
| 12470   | Mrc.Cpt. Headgear    | Dark index |
| 12726   | Mrc.Cpt. Gloves      | Dark index |
| 12854   | Mrc.Cpt. Hose        | Dark index |
| 12982   | Mrc.Cpt. Gaiters     | Dark index |
| 13496   | Windurstian Ring      | Dark index |
| 17282   | Cmb.Cst. B'merang    | Dark index |
| 12614   | Cmb.Cst. Cloak       | Dark index |
| 12743   | Cmb.Cst. Mitts       | Dark index |
| 12870   | Cmb.Cst. Slacks      | Dark index |
| 16669   | Cmb.Cst. Axe         | Dark index |
| 13102   | Paisley Scarf        | Dark index |
| 12606   | Tct.Mgc. Coat        | Dark index |
| 12734   | Tct.Mgc. Cuffs       | Dark index |
| 12862   | Tct.Mgc. Slops       | Dark index |
| 16694   | Tct.Mag. Hooks       | Dark index |
| 13581   | Fed. Army Mantle     | Dark index |
| 16808   | Wis.Wiz. Bilbo       | Dark index |
| 16809   | Wis.Wiz. Anelace     | Dark index |
| 12363   | Ptr.Prt. Shield      | Dark index |
| 14017   | Mst.Cst. Bracelets   | Dark index |
| 17530   | Mst.Cst. Pole        | Dark index |
| 17617   | Mst.Cst. Knife       | Dark index |
| ... | *(~40 more San d'Orian / Bastokan rank items)* | Dark index |

**Not annotated — common items (dark indices + missing from item_map):**

| Item ID | Item Name | Why unannotated |
|---------|-----------|-----------------|
| 4182    | Instant Reraise      | Not in any loaded item DAT |
| 4181    | Instant Warp         | Not in any loaded item DAT |
| 15542   | Return Ring          | Dark index (in item DB but unreferenced) |
| 15541   | Homing Ring          | Dark index |
| 28540   | Warp Ring            | Dark index |
| 10114   | Cipher: Tenzen       | Dark index |
| 10139   | Cipher: Rahal        | Dark index |
| 10146   | Cipher: Kukki        | Dark index |
| 10180   | Cipher: Makki        | Dark index |
| 6379    | Ornate Stool         | Not in any loaded item DAT |
| 6380    | Refined Chair        | Not in any loaded item DAT |
| 6377    | *(unknown furniture)* | Not in any loaded item DAT |
| 6378    | *(unknown furniture)* | Not in any loaded item DAT |
| 17583   | Kgd. Signet Staff    | Dark index |
| 17584   | Rep. Signet Staff    | Dark index |
| 17585   | Fed. Signet Staff    | Dark index |
| 183     | Windurstian Flag     | Dark index (in item DB but unreferenced) |
| 182     | Bastokan Flag        | Dark index |
| 181     | San d'Orian Flag     | Dark index |

**Not annotated — costs:**

| Value  | Meaning | Why unannotated |
|--------|---------|-----------------|
| 100    | Rank 1 (1,000 CP ÷10)  | Dark index; not traced to cost slot |
| 200    | Rank 2 (2,000 CP ÷10)  | Dark index |
| 400    | Rank 3 (4,000 CP ÷10)  | Dark index |
| 800    | Rank 4 (8,000 CP ÷10)  | Dark index |
| 1600   | Rank 5 (16,000 CP ÷10) | Dark index |
| 2400   | Rank 6 (24,000 CP ÷10) | Dark index |
| 3200   | Rank 7 (32,000 CP ÷10) | Dark index |
| 4000   | Rank 8 (40,000 CP ÷10) | Dark index |
| 4800   | Rank 9 (48,000 CP ÷10) | Dark index |
| 5000   | Fed. Signet Staff / Warp Ring cost | Dark index |
| 10000  | Ornate Stool cost    | Dark index |
| 20000  | Refined Chair cost   | Dark index |
| 90000  | Nation Flag cost     | Dark index |
| 2500   | Return Ring cost     | Dark index |
| 500    | Chariot Band cost    | Dark index |
| 1000   | Empress Band cost    | Dark index |
| 2000   | Emperor Band cost    | Dark index |

Note: 5600 (Rank 10 ÷10) IS annotated because it was the only rank cost traced to a
cost slot via opcode 0x03.

**Not annotated — dialog IDs:**

Values like 8959–9108 are dialog IDs for the conquest menu system. These are correctly
left unannotated (they're filtered by the `dialog_ids` set).

**Not annotated — control values:**

Values like 0–31, 32, 48, 63, 64, 80, 96, 112, 128, 144, 150, 160, 201, 255, 260,
497, 504, 1024 are control/flag values, offsets, or bit patterns used by the VM. These
are correctly left unannotated.

### A.4 Annotation Gap Analysis

Of Harara's 354 `immed_data` entries:
- ~77 are annotated (~22%)
- ~100+ are dialog IDs (correctly unannotated)
- ~30 are control values (correctly unannotated)
- ~75 are items that SHOULD be annotated but are "dark" (unreferenced by opcodes)
- ~17 are costs that SHOULD be annotated but are "dark"
- ~6 items are not in the item_map (4181, 4182, 6377–6380)

**Root cause**: The vast majority of annotation gaps are "dark indices" — entries not
referenced by any of the opcodes we scan (0x02–0x15, 0x19). This means either:
1. The items/costs are loaded through opcodes we don't trace (e.g. 0x41 `GetBitFlag`,
   selection-line opcodes, or other opcodes that read from References)
2. The items/costs are addressed through inline data sections rather than parsed opcodes
3. The server directly populates Work_Zone registers via packets, bypassing event DAT
   opcodes entirely

Investigating which of these cases applies is the primary goal of the next work phase.

### A.5 Items Missing from Item Database

The following IDs appear in the conquest vendor data but are NOT found in any of the
exported item DATs (weapons, armor, armor2, general_items, general_items2):

| ID   | Known Name (from wiki) | Likely DAT |
|------|------------------------|------------|
| 4181 | Instant Warp           | usable_items (not currently exported) |
| 4182 | Instant Reraise        | usable_items (not currently exported) |
| 6377 | *(Furniture/Furnishing)* | Not in standard item DATs |
| 6378 | *(Furniture/Furnishing)* | Not in standard item DATs |
| 6379 | Ornate Stool           | Not in standard item DATs |
| 6380 | Refined Chair          | Not in standard item DATs |

The `load_item_id_to_name_map` function loads from: weapons, armor, armor2,
puppet_items, general_items, general_items2, usable_items, currency, vouchers_and_slips,
monipulator, instincts. Items not in any loaded DAT or where `log_name()` returns
`None` are not annotated.
