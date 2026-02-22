# Design: 3D-to-Sprite-Sheet Pipeline

**Status**: Proposed
**Date**: 2026-02-22

## Context

PoC R3 (procedural 16x16 pixel art) failed — code-generated sprites aren't recognizable enough for 30+ distinct characters. The "zero art budget" constraint needs to be relaxed. This plan replaces procedural pixel art with AI-generated 3D models rendered as 2D sprite sheets, keeping the entire 2D Bevy architecture intact.

## Approach

Generate 3D humanoid models via Meshy API, rig and animate via Mixamo, render top-down sprite sheets via Blender headless, load as TextureAtlas in Bevy.

**Resolution**: 64x64 per frame (good detail, manageable file size, standard for modern retro)
**Camera**: Orthographic top-down with slight tilt (~30-45 degrees) for depth
**Format**: PNG sprite sheets with transparent background, loaded via `TextureAtlasLayout::from_grid()`

---

## Pipeline Steps

### Step 1: Generate Base 3D Models (Meshy API)

Generate humanoid character models via Meshy REST API:
- **Candide** — naive young man, simple clothing, wide-eyed expression
- **Soldier** — military figure, red uniform
- **Inquisitor** — robed religious figure, purple vestments
- **Thief** — hooded figure, yellow/gold clothing
- **Slaver** — heavy-set figure, green clothing
- **Weapons** — 5 separate models (brass knuckles, bat, knife, axe, chainsaw)
- **Luxury items** — 6 separate models (gold grill, chain, Rolex, goblet, fur coat, gold toilet)

**API flow**: POST text-to-3d → poll task status → download GLB

For art style consistency: use similar prompt structure for all characters (e.g., "stylized low-poly [character description], game character, cel-shaded").

### Step 2: Rig and Animate (Mixamo — manual step)

Upload each humanoid GLB to Mixamo (web interface, no API):
1. Auto-rig with marker placement
2. Download these animation clips per character (FBX format, "without skin" for animations, "with skin" for base):
   - **Walk cycle** (in-place)
   - **Idle**
   - **Attack/punch** (for weapon use)
   - **Death/fall**
   - **Frightened/scared run** (enemies only)

This is a one-time manual process per character model (~30 min per character).

### Step 3: Render Sprite Sheets (Blender headless)

Write a Python script (`tools/render_sprites.py`) that:
1. Imports rigged+animated FBX into Blender
2. Sets up orthographic camera at top-down angle
3. For each animation clip:
   - Renders each frame at 64x64 resolution
   - For walk cycles: renders 4 directions (rotate model 0/90/180/270 degrees)
4. Composites frames into a single sprite sheet PNG per character
5. Outputs metadata (frame count, animation ranges) as JSON

Run headless: `blender -b -P tools/render_sprites.py -- --model soldier.fbx --output assets/sprites/`

**Frame budget per character**:

| Animation | Frames | Directions | Total |
|-----------|--------|------------|-------|
| Walk      | 6      | 4          | 24    |
| Idle      | 2      | 1          | 2     |
| Attack    | 4      | 4          | 16    |
| Death     | 4      | 1          | 4     |
| **Total** |        |            | **46** |

Sprite sheet layout: 8 columns, 6 rows = 48 cells (512x384 pixels per sheet).

### Step 4: Candide Luxury Variants

For each luxury item overlay:
1. Attach the luxury item model to Candide's rig in Blender (parent to appropriate bone)
2. Re-render the full sprite sheet
3. Produces 7 Candide variants (base + 6 luxury items)

### Step 5: Integrate into Bevy

Replace `src/procgen/` with `src/plugins/sprites.rs`:

```rust
// Load sprite sheets from assets/sprites/
// Create TextureAtlasLayout::from_grid(UVec2::new(64, 64), 8, 6, None, None)
// Map animation states to frame ranges
// SpriteGenPlugin inserts sprite handles as resources during Loading state
```

Update `assets/` structure:
```
assets/
├── sprites/
│   ├── candide_base.png       # 512x384 sprite sheet
│   ├── candide_grill.png      # luxury variant
│   ├── candide_chain.png
│   ├── ...
│   ├── soldier.png
│   ├── inquisitor.png
│   ├── thief.png
│   ├── slaver.png
│   ├── weapons.png            # all 5 weapons on one sheet
│   ├── items.png              # all 6 luxury items on one sheet
│   └── tiles.png              # wall/floor/dot tiles
├── sprites.json               # frame metadata
```

### Step 6: Animation System

Add sprite animation to the existing movement system:
- `AnimationTimer` component ticks through frames
- `MoveDirection` selects the correct row in the sprite sheet
- `ActiveWeapon` triggers attack animation
- Death/frightened states select appropriate frame ranges

---

## Architecture Changes

### What changes
- `src/procgen/` → deleted (was procedural pixel art generation)
- `src/plugins/sprites.rs` → new (loads sprite sheet assets)
- `ARCHITECTURE.md` Section 3 → updated (sprite sheets replace procedural art)
- `assets/sprites/` → new directory with PNG sprite sheets
- Risk R3 → resolved with new approach

### What stays the same
- Grid-based movement, 2D camera, all game logic
- All other plugins (maze, player, enemies, combat, collectibles, narration, etc.)
- Bevy 2D rendering pipeline
- `TextureAtlas` usage pattern (same concept, different source)

---

## Tools Required

- **Meshy API account** — for 3D model generation (paid, ~$20/month)
- **Mixamo** — free, web-based, for rigging + animation library
- **Blender 4.x** — free, for headless sprite sheet rendering
- **Python 3** — for Blender scripting

## PoC Before Full Implementation

Before building the full pipeline, validate with a single character:
1. Generate one humanoid model via Meshy API
2. Rig + apply walk cycle in Mixamo
3. Render 64x64 sprite sheet in Blender (4-direction walk + idle)
4. Load into a Bevy test window
5. Verify: character is recognizable, walk animation is smooth, sprite quality is acceptable

**Pass criteria**: Character is clearly identifiable, walk cycle looks natural, visual quality is a clear improvement over procedural pixel art.

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Art style inconsistency between characters | Medium | Use consistent prompt structure, same model style |
| Meshy API quality insufficient | Medium | Fall back to Tripo API or manual modeling |
| Mixamo rigging fails on AI models | Low | AccuRIG 2 as backup, manual Blender rigging as last resort |
| Blender rendering doesn't look good at 64x64 | Low | Adjust camera angle, lighting, try 128x128 |
| Pipeline too slow to iterate | Low | Script everything, cache intermediate results |
