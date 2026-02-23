# Level Design Guidelines

Rules for creating ASCII maze files in `assets/maps/`.

## 1. Connectedness

Every walkable tile must be reachable from the player spawn (`P`). Characters cannot fly — there must be a contiguous path of walkable tiles connecting all money dots, weapon spawns, luxury spawns, and the enemy pen gate to the player spawn.

## 2. Tunnel Wrapping

Openings in the outside walls create wrap-around tunnels. If there is an opening on the left wall, there must be a matching opening on the right wall at the same row. If there is an opening on the top wall, there must be a matching opening on the bottom wall at the same column. Characters exiting one side re-enter from the opposite side.

## 3. Tile Legend

| Char | Meaning |
|------|---------|
| `#` | Wall (impassable) |
| `.` | Money dot (collectible) |
| ` ` | Empty floor (walkable, no pickup) |
| `P` | Player spawn (exactly one per map) |
| `G` | Enemy spawn (typically 4 per map, inside the pen) |
| `W` | Weapon spawn |
| `L` | Luxury item spawn |
| `-` | Pen gate (walkable for enemies, not for player) |

## 4. Enemy Pen

The pen is an enclosed area with walls on all sides except for one or two `-` gate tiles on top. Enemies spawn inside the pen and are released one at a time through the gate. The pen interior should use `G` for spawn positions and spaces for remaining floor tiles.

## 5. Row Width

All rows in a map must be the same width (pad shorter rows with spaces if needed). The parser pads automatically, but consistent widths make the map easier to read and edit.

## 6. The Garden (Level 13)

The Garden is a special level with no enemies, no weapons, and no narration. It uses only `#`, `.`, `P`, and ` ` — no `G`, `W`, `L`, or `-` tiles.
