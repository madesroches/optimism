# Design: 3D-to-Sprite-Sheet Pipeline

**Status**: Proposed
**Date**: 2026-02-22

## Context

PoC R3 (procedural 16x16 pixel art) failed — code-generated sprites aren't recognizable enough for 30+ distinct characters. The "zero art budget" constraint needs to be relaxed. This plan replaces procedural pixel art with AI-generated 3D models rendered as 2D sprite sheets, keeping the entire 2D Bevy architecture intact.

## Characters Needed

- **Candide** — naive young man, simple clothing, wide-eyed expression
- **Soldier** — military figure, red uniform
- **Inquisitor** — robed religious figure, purple vestments
- **Thief** — hooded figure, yellow/gold clothing
- **Slaver** — heavy-set figure, green clothing
- **Weapons** — 5 models (brass knuckles, bat, knife, axe, chainsaw)
- **Luxury items** — 6 models (gold grill, chain, Rolex, goblet, fur coat, gold toilet)
- **Candide variants** — base + 6 luxury item overlays = 7 total

## Shared Parameters

**Resolution**: 64x64 per frame (good detail, manageable file size, standard for modern retro)
**Camera**: Orthographic top-down with slight tilt (~30-45 degrees) for depth
**Format**: PNG sprite sheets with transparent background, loaded via `TextureAtlasLayout::from_grid()`

**Frame budget per character**:

| Animation | Frames | Directions | Total |
|-----------|--------|------------|-------|
| Walk      | 6      | 4          | 24    |
| Idle      | 2      | 1          | 2     |
| Attack    | 4      | 4          | 16    |
| Death     | 4      | 1          | 4     |
| **Total** |        |            | **46** |

Sprite sheet layout: 8 columns, 6 rows = 48 cells (512x384 pixels per sheet).

**Animations needed**: Walk cycle (in-place), Idle, Attack/punch, Death/fall, Frightened/scared run (enemies only).

---

## Two Approaches — Validate with PoCs

We have two viable approaches for producing character sprites. Each has a different risk profile. We'll run both PoCs in parallel and pick the winner.

---

## Approach A: Meshy End-to-End (AI-Generated Models)

Generate unique 3D models per character via Meshy API, rig and animate via Meshy's own rigging/animation API, render sprite sheets in Blender.

### Pipeline

1. **Generate models** — `POST /openapi/v1/text-to-3d` per character with consistent prompt structure (e.g., "stylized low-poly [character description], game character, cel-shaded"). Poll task → download GLB.
2. **Rig** — `POST /openapi/v1/rigging` with `input_task_id` from generation. 5 credits per rig.
3. **Animate** — `POST /openapi/v1/animations` per animation clip. 500+ built-in animations available. 3 credits per animation. ~20 credits per character for 5 animations.
4. **Render sprite sheets** — Blender headless script (`tools/render_sprites.py`): import rigged+animated FBX, orthographic camera, render 4 directions for walk/attack, composite into sprite sheet PNG + JSON metadata.
5. **Candide variants** — Generate luxury item models separately via Meshy, attach to Candide's rig in Blender, re-render.

### Pros
- Each character has a unique silhouette and mesh
- Fully scriptable REST API end-to-end (no manual steps)
- Single vendor for generation + rigging + animation

### Cons
- Art style consistency between characters is not guaranteed
- Meshy rigging may fail on its own generated meshes
- Each character rigged independently — animation issues multiply
- Cost: ~250 credits for all characters ($20/month Pro gives 1000)

### Tools Required
- Meshy API account (~$20/month Pro)
- Blender 4.x + Python 3 (free)

### PoC R4a: Meshy End-to-End

1. Generate one humanoid model via `POST /openapi/v1/text-to-3d` (Candide: "stylized low-poly naive young man, simple clothing, wide eyes, game character, cel-shaded")
2. Rig via `POST /openapi/v1/rigging` with `input_task_id`
3. Apply walk cycle + idle via `POST /openapi/v1/animations`
4. Download FBX
5. Render 64x64 sprite sheet in Blender (4-direction walk + idle)
6. Load into a Bevy test window

**Pass criteria**: Character is clearly identifiable at 64x64, walk cycle looks natural, pipeline is fully scriptable without manual steps.

---

## Approach B: Reskin Free Base Model (Quaternius + Modifications)

Start from a single free pre-rigged humanoid model, duplicate and modify (outfits, materials, retexturing) to create all characters. All variants share the same rig and animations automatically.

### Pipeline

1. **Base model** — Download Quaternius Universal Base Characters (CC0, free). Use "Regular Male" as the base. Pre-rigged humanoid with retargeting support.
2. **Animations** — Download Quaternius Universal Animation Library (CC0, free). 45 animations including walk, idle, combat, death. Same rig = zero retargeting work.
3. **Character differentiation** — For each character:
   - Apply modular outfit pieces from Quaternius Modular Character Outfits - Fantasy (CC0, free) — robes, hoods, armor parts
   - Material/color swaps in Blender (red for Soldier, purple for Inquisitor, etc.)
   - Optionally retexture via Meshy API (`POST /openapi/v1/retexture`, 10 credits) for richer textures
4. **Render sprite sheets** — Same Blender headless script as Approach A.
5. **Candide variants** — Create simple luxury item props in Blender, parent to appropriate bones, re-render.

### Pros
- Guaranteed art style consistency (same base model)
- All characters share animations automatically (same rig = zero per-character rigging)
- $0 asset cost (all CC0), optional Meshy retexture for polish
- Lowest risk — proven working rig, no AI rigging failures
- Fastest iteration — change a material, re-render

### Cons
- Less silhouette variety (mitigated by modular outfit pieces + accessories)
- Low-poly Quaternius style may not fit desired aesthetic
- Some manual Blender work for outfit assembly (one-time, scriptable after first setup)

### Tools Required
- Blender 4.x + Python 3 (free)
- Meshy API account (optional, for retexturing — ~$20/month)

### PoC R4b: Reskin Base Model

1. Download Quaternius Regular Male + Universal Animation Library
2. In Blender: apply walk cycle animation, swap material to red (soldier test)
3. Render 64x64 sprite sheet (4-direction walk + idle)
4. Load into a Bevy test window

**Pass criteria**: Character is clearly identifiable at 64x64, walk cycle looks natural, low-poly style reads well at small size, visual quality is a clear improvement over procedural pixel art.

---

## Comparison

| Dimension | Approach A (Meshy) | Approach B (Reskin) |
|---|---|---|
| Art consistency | Medium — prompt engineering | High — same base model |
| Silhouette variety | High — unique meshes | Medium — outfits + accessories |
| Animation risk | Medium — per-character rigging | None — shared rig |
| Automation | Full API | Blender scripting + optional API |
| Asset cost | ~250 credits | $0 (CC0) |
| Manual effort | None (if API works) | Initial outfit assembly |
| Pipeline complexity | Higher (3 API stages) | Lower (file-based) |
| Fallback if PoC fails | Approach B | Manual modeling |

---

## Shared: Bevy Integration

Both approaches produce the same output — PNG sprite sheets + JSON metadata. The Bevy side is identical regardless of which approach wins.

Replace `src/procgen/` with `src/plugins/sprites.rs`:

```rust
// Load sprite sheets from assets/sprites/
// Create TextureAtlasLayout::from_grid(UVec2::new(64, 64), 8, 6, None, None)
// Map animation states to frame ranges
// SpriteGenPlugin inserts sprite handles as resources during Loading state
```

Asset structure:
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

Animation system:
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

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Meshy-generated models fail auto-rigging | Medium | Approach B as fallback |
| Art style inconsistency (Approach A) | Medium | Consistent prompt structure, same model style |
| Quaternius low-poly style too simple | Medium | Meshy retexture API, or fall back to Approach A |
| Meshy retexture doesn't preserve rig | Medium | Skip retexturing, use material swaps only |
| Blender rendering doesn't look good at 64x64 | Low | Adjust camera angle, lighting, try 128x128 |
| Pipeline too slow to iterate | Low | Script everything, cache intermediate results |
