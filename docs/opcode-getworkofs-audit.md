# Opcode getworkofs Audit (XiEvents)

Audit of atom0s's XiEvents OpCodes to identify all opcodes that call `getworkofs` or
`getworkstrofs` — these can read from the References (immed_data) table. Opcodes we
scan for `all_referenced` reduce "dark indices" in WZ1700-context blocks.

**Source**: `../XiEvents/OpCodes/` (local clone)

## Currently Scanned (enrichment.rs)

| Opcode | Params | Purpose |
|--------|--------|---------|
| 0x02 | 1-2, 3-4 | Conditional compare |
| 0x03 | 0-1, 2-3 | Copy (get/store) |
| 0x05, 0x06, 0x0B, 0x0C | 0-1 | Set/increment/decrement |
| 0x07–0x15, 0x19 | 0-1, 2-3 | Arithmetic/bitwise/shift/swap |
| 0x24 | 0-1, 2-3, 4-5 | Dialog selection (CodeQUERY) |
| 0x3C, 0x3D, 0x3E | 0-1, 2-3, 4-5 | Compare + set/clear/test bit |
| 0x40 | 0-1, 2-3, 4-5, 6-7 | Set bit flags (menu options) |
| 0x41 | 0-1, 2-3, 4-5 | Get bit flags (menu options) |

## Not Yet Scanned (References-reading opcodes)

These opcodes use getworkofs/getworkstrofs and could be added to expand
`all_referenced` coverage. Param indices are byte offsets (1-based in XiEvents docs;
0-based in our params array: index N → params[N-1], params[N]).

| Opcode | Size | Param indices (1-based) | Notes |
|--------|------|-------------------------|-------|
| 0x1C | 3 | 1 | Set WaitTime |
| 0x1D | 3 | 1 | Load/print message (dialog ID) |
| 0x1F | 2,8 | varies | Update event position |
| 0x2B | 7 | 1,3,5 | Load message with entity |
| 0x31 | 2,10 | varies | Update position |
| 0x32 | 3 | 1 | Set MainSpeed |
| 0x34, 0x35 | 3 | 1 | Load zone |
| 0x36 | 7 | 1,3,5 | Update EventPos |
| 0x37 | 9 | 1,3,5,7 | Update EventPos + EventDir |
| 0x38 | 3 | 1 | Set CliEventModeLocal |
| 0x39 | 3 | 1 | Set EventDir |
| 0x3F | 7 | 1,3,5 | Remainder |
| 0x44 | 5 | 1,3 | Test entity valid |
| 0x47 | 2,10 | varies | Update player location |
| 0x48 | 3 | 1 | Load message (no speaker) |
| 0x49 | 7 | 1,3,5 | Load message |
| 0x4B | 7 | 1,3,5 | Update entity yaw |
| 0x4F | 3 | 1 | Set StatusEvent |
| 0x57 | 3 | 1 | Frame delay |
| 0x59 | 4,6,7,8 | varies | Entity data update |
| 0x63 | 3 | 1 | Play animation |
| 0x64 | 11 | 1,3,5,7,9 | Distance calc |
| 0x65 | 11 | 1,3,5,7,9 | 3D distance |
| 0x67 | 5 | 1,3 | Hide HUD |
| 0x69 | 4 | 1,3 | Set sound volume |
| 0x6A | 7 | 1,3,5 | Change sound volume |
| 0x6C | 9 | 1,3,5,7 | Fade entity |
| 0x6E | 7 | 1,3,5 | Play emote |
| 0x71 | 2,4,6,8,10 | varies | String input |
| 0x72 | 4,6,10 | varies | Weather |
| 0x73 | 11 | 1,3,5,7,9 | Cast magic |
| 0x75 | 2,4 | varies | Load room |
| 0x76 | 5 | 1,3 | Check entity flags |
| 0x77 | 5 | 1,3 | Set game time |
| 0x79 | 10,12 | varies | Look at entity |
| 0x7D | 3 | 1 | Rank up animation |
| 0x7E | 6,8,16,18 | varies | Chocobo/mount |
| 0x83 | 3 | 1 | Get game time |
| 0x89 | 3 | 1 | Open map |
| 0x8B | 25 | many | Set map marker |
| 0x8C | 2,8,10,12,14 | varies | Crafting |
| 0x8D | 5 | 1,3 | Open map |
| 0x91 | 3 | 1 | Set MainSpeedBase |
| 0x93 | 3 | 1 | Display item info |
| 0x95 | 3 | 1 | Event NPC setup |
| 0x97 | 5 | 1,3 | Wind values |
| 0x99 | 5 | 1,3 | Yield if anim playing |
| 0x9C | 3 | 1 | Store language id |
| 0x9D | 6,8,9,10,23 | varies | String handling |
| 0xA6 | 1,2,4 | varies | Request map number |
| 0xA7 | 2,4 | varies | Wait for server response |
| 0xA8 | 6 | 1,3,5 | Map markers |
| 0xA9 | 3 | 1 | Disable game time |
| 0xAA | 17 | many | Vana'diel timestamp |
| 0xAD | 12 | 1,3,5,7,9 | Scheduler actions |
| 0xAE | 6,8,10 | varies | Sub-cases |
| 0xAF | 8 | 1,3,5,7 | Camera position |
| 0xB0 | 12 | 1,3,5,7,9,11 | Load message |
| 0xB1 | 4 | 1,3 | Get flag value |
| 0xB2 | 2,4 | 1,3 | Delivery box |
| 0xB3 | 2,4,14,18 | varies | Ranking boards |
| 0xB4 | 2,3,4,6,12,20 | varies | Multi-purpose |
| 0xB5 | 4 | 1,3 | Set entity name |
| 0xB6 | 2,4,6,14,16,20 | varies | Entity looks/gear |
| 0xB7 | 8,10 | varies | Sub-usages |
| 0xB8 | 27 | many | Map add/set markers |
| 0xB9 | 8 | 1,3,5,7 | Edit marker |
| 0xBA | 13 | 1,3,5,7,9,11 | Entity position |
| 0xBF | 8,10 | 2,4,6 | Chocobo racing |
| 0xC0 | 3 | 1 | Render flags |
| 0xC1 | 5 | 1,3 | Kill action |
| 0xC2 | 2,4,6 | 2 | Party state |
| 0xC3 | 7 | 1,3,5 | Copy string |
| 0xC8 | 7 | 1,3,5 | Map window |
| 0xCC | 4,6,10,14 | varies | Info windows (items) |
| 0xD4 | 2,6,8,12 | varies | Map/input |
| 0xD8 | 6,8,12 | 1,3,5,7,9,11 | EventDir |

## Summary

- **Scanned**: 0x02, 0x03, 0x05–0x19, 0x24, 0x3C–0x3E, 0x40, 0x41
- **Not scanned**: 80+ opcodes use getworkofs; many have variable sizes or
  sub-opcodes. Adding the highest-impact ones (e.g. 0x1D dialog ID, 0x48/0x49
  message load) could further reduce dark indices.
