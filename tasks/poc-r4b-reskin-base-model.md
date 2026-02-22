# PoC R4b: Reskin Free Base Model

**Status**: In Progress
**Date**: 2026-02-22
**Parent**: `tasks/3d-to-sprite-sheet-pipeline.md` (Approach B)

## Goal

Validate that Quaternius CC0 base models + modular outfits + material swaps can produce recognizable 64x64 top-down sprite sheets for all game characters. Zero cost, zero rigging, shared animations.

## Prerequisites

- Blender 4.x installed (headless capable)
- Python 3.x (for Blender scripting)

---

## Phase 1: Download and Inspect Assets ✓

### 1.1 Download Quaternius Packs ✓

**DONE.** All CC0 licensed, all free (Standard tier). Zips downloaded and unpacked into `art/quaternius/`:

| Pack | Subdirectory | What we got |
|------|-------------|-------------|
| Universal Base Characters | `base-characters/` | Superhero_Male_FullBody (~14k tris, 65-bone UE4 rig) |
| Universal Animation Library | `animation-library/` | 45 actions in UAL1_Standard.glb (walk, idle, death, sword, spell, etc.) |
| Universal Animation Library 2 | `animation-library-2/` | 43 actions in UAL2_Standard.glb (sword combos, zombie walk, etc.) |
| Modular Character Outfits - Fantasy | `outfits-fantasy/` | Standard tier: Peasant + Ranger outfits (male/female), 10 male modular parts |
| LowPoly Medieval Weapons | `medieval-weapons/` | 24 weapon FBX models |
| Fantasy Props MegaKit | `fantasy-props/` | 94 free prop glTF models (mugs, chains, coins, etc.) |

**Note:** Standard tier only includes Superhero body type (not Regular Male) and 2 outfit sets (Peasant, Ranger). The plan originally assumed Regular Male and 12 outfits — adjusted character assembly to work with what's available.

### 1.2 Inspect in Blender ✓

**DONE.** Automated inspection via `tools/inspect_assets.py`:
- **Rig**: 65 bones, UE4 naming convention (root→pelvis→spine_01, hand_l/hand_r, Head, etc.)
- **Animation compatibility**: 65/65 bones match exactly between base character, animation libraries, and outfits (100%)
- **Root motion**: Walk/run animations are in-place (no root motion to strip)
- **Outfit compatibility**: Both Peasant and Ranger outfits share the identical 65-bone armature
- **Key bones for prop attachment**: Head, neck_01, spine_01/02/03, hand_l, hand_r

**Decision gate**: PASSED — all assets fully compatible.

---

## Phase 2: Character Assembly in Blender ✓

### 2.1 Automated Assembly Script ✓

**DONE.** Instead of manual Blender work, wrote `tools/assemble_characters.py` — a headless Blender script that programmatically builds all character .blend files:

```
blender -b -P tools/assemble_characters.py
```

For each character, the script:
1. Imports base model or complete outfit file (same 65-bone armature)
2. Imports animation library 1 (45 actions), marks all as fake_user
3. Optionally imports extra modular parts and re-parents to main armature
4. Applies character-specific material colors (disconnects texture nodes, sets flat color)
5. Saves as `art/characters/{name}.blend`

Single-character validation: `blender -b -P tools/assemble_one.py -- soldier`

### 2.2 Character Definitions (Revised for Standard Tier) ✓

**DONE.** Adapted character assignments to available Peasant/Ranger outfits:

| Character | Model Source | Primary Color | Distinguishing Feature |
|-----------|-------------|---------------|----------------------|
| **candide_base** | Base Superhero_Male (no outfit) | Cream (0.9, 0.88, 0.82) | Bare skin — the "blank" character |
| **soldier** | Full Ranger outfit + Sword.fbx | Deep red (0.75, 0.12, 0.1) | Hooded + armored + sword in hand_r |
| **inquisitor** | Peasant outfit + Ranger hood | Purple (0.45, 0.12, 0.65) | Robed body with hood — mixed outfit |
| **thief** | Ranger outfit minus pauldron | Gold (0.85, 0.72, 0.08) | Lighter ranger silhouette |
| **brute** | Full Peasant outfit | Dark green (0.18, 0.55, 0.12) | Bulky peasant clothes |

### 2.3 Candide Luxury Variants — DEFERRED

Props are too small to read at 64x64 (tested grill, chain, goblet — only goblet barely visible). Disabled in `assemble_characters.py`. Revisit when we have a better approach (glow effects, larger/exaggerated props, or higher render resolution).

### 2.4 Weapon and Item Sprites

Not yet started. Deferred until character sprites are validated.

---

## Phase 3: Blender Headless Rendering Script ✓

### 3.1 Write `tools/render_sprites.py` ✓

**DONE.** Python script that Blender runs headless to produce sprite sheets:

```
blender -b art/characters/soldier.blend -P tools/render_sprites.py -- --output assets/sprites/soldier.png
```

The script:
1. Finds the armature and its animation actions (fuzzy name matching, prefers shortest match)
2. For each animation (walk, idle, attack, death):
   - Sets the active action
   - For directional animations (walk, attack): rotates model 0°/90°/180°/270° and renders each frame
   - For non-directional (idle, death): renders frames at default rotation
3. Renders each frame at 64x64 with transparent background (EEVEE)
4. Composites all frames into a single sprite sheet (configurable columns)
5. Writes per-character JSON metadata alongside the PNG

CLI options: `--size`, `--columns`, `--camera-angle`, `--camera-distance`, `--outline` (Freestyle)

**Bugs fixed during validation:**
- Camera position had sin/cos swapped — camera was pointed away from character (empty renders)
- EEVEE engine name: Blender 4.0.x uses `BLENDER_EEVEE`, not `BLENDER_EEVEE_NEXT` (that's 4.2+)
- Action matching: changed from "first match" to "shortest name match" to avoid picking Walk_Formal over Walk_Loop

### 3.2 Write `tools/render_all.py` ✓

**DONE.** Python script that renders all characters:

```
python3 tools/render_all.py
```

### 3.3 Render and Validate ✓

**DONE.** Two render passes:

**Pass 1 (failed):** Colors not visible — GLTF texture nodes overrode flat color. Fixed by disconnecting texture links before setting Base Color.

**Pass 2 (passed):** All 5 characters rendered, clearly distinguishable:

| Criteria | Status | Notes |
|----------|--------|-------|
| Walk cycle reads as walking? | ✓ | 4-directional walk cycles look good |
| Attack animation has clear motion? | ✓ | Sword swing visible in all 4 directions |
| Death animation reads clearly? | ✓ | Character falls convincingly |
| Outfit pieces clip during animation? | ✓ | No visible clipping at 64x64 |
| Characters distinguishable by color? | ✓ | Red/purple/gold/green clearly different after texture disconnect fix |
| Characters distinguishable by silhouette? | ✓ | Soldier (sword), Candide (bare skin), Inquisitor (hood+robe), Thief (hood, lighter), Brute (bulky peasant) |

---

## Phase 4: Bevy Integration ✓

### 4.1 Load Sprite Sheets ✓

**DONE.** `src/plugins/sprites.rs` provides:
- `SpriteSheetLibrary` resource — loads PNG + per-character JSON sidecar, builds `TextureAtlasLayout::from_grid`
- `CharacterSheet` — bundles image handle, layout handle, and parsed metadata
- `SpriteSheetPlugin` — registers the library resource and animation system

### 4.2 Animation System ✓

**DONE.** Components:
- `AnimationTimer` — ticks through frames based on elapsed time
- `AnimationState` — current animation key, looping flag, finished flag
- `FacingDirection` — Down/Left/Up/Right, selects directional animation variant
- `CharacterSheetRef` — links entity to its sheet in the library
- `animate_sprites` system reads JSON metadata to map state+direction to frame range

Helpers: `resolve_animation_key()` (directional fallback), `set_animation()` (switch with reset)

### 4.3 Test Window ✓

**DONE.** `examples/sprite_test.rs` — run with:
```
cargo run --example sprite_test -- [character_name]
```
- Arrow keys move the character (switches walk direction animations)
- Space triggers attack, D triggers death, I triggers idle
- HUD shows controls

---

## Phase 5: Full Character Lineup Test

Once the pipeline works for one character:
1. Assemble all 5 enemy characters + 7 Candide variants
2. Render all sprite sheets via batch script
3. Load all into Bevy test scene
4. Display all characters side-by-side to assess visual distinctiveness

**Pass criteria**:
- Each character is identifiable by silhouette + color at 64x64
- All animations play smoothly
- Visual quality is a clear improvement over procedural pixel art (PoC R3)
- Pipeline is repeatable: change a material → re-render → updated sprite sheet

---

## Output Files

```
art/                              # gitignored — binary sources
├── quaternius/                   # downloaded + unpacked asset packs
│   ├── base-characters/
│   ├── animation-library/
│   ├── animation-library-2/
│   ├── outfits-fantasy/
│   ├── medieval-weapons/
│   └── fantasy-props/
├── characters/                  # generated by assemble_characters.py
│   ├── candide_base.blend
│   ├── soldier.blend
│   ├── inquisitor.blend
│   ├── thief.blend
│   └── brute.blend

tools/                            # committed
├── assemble_characters.py       # Blender headless: build all .blend files from assets
├── assemble_one.py              # Blender headless: build single character for testing
├── inspect_assets.py            # Blender headless: inspect rig/bones/animations
├── render_sprites.py            # Blender headless: render .blend → sprite sheet
└── render_all.py                # Python: batch render all characters

assets/sprites/                   # committed — final output
├── candide_base.png + .json     # sprite sheets (8x6 grid, 46 frames each)
├── soldier.png + .json
├── inquisitor.png + .json
├── thief.png + .json
├── brute.png + .json
├── weapons.png                  # not yet — 5 weapons, static sprites
├── items.png                    # not yet — 6 luxury items, static sprites
└── tiles.png                    # not yet — wall/floor/dot tiles

src/plugins/
├── mod.rs                       # committed — module declaration
└── sprites.rs                   # committed — Bevy sprite loading + animation

examples/
└── sprite_test.rs               # committed — interactive sprite validation
```

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Quaternius low-poly style doesn't read well at 64x64 | Medium | Adjust camera angle/distance, try 128x128, add outline shader in render script |
| Modular outfit pieces clip during animations | Medium | Test all animations per character, adjust outfit placement or pick different pieces |
| Animation library missing a needed animation (e.g., frightened run) | Low | Use fast walk/jog variant, or import single animation from Mixamo as fallback |
| Walk animations have root motion (not in-place) | Low | Strip root motion in Blender script before rendering |
| Luxury item props deform weirdly when parented to bones | Low | Use rigid body constraint instead of direct parenting |

---

## Time Estimate

| Phase | Effort |
|-------|--------|
| Phase 1: Download + inspect | 1-2 hours |
| Phase 2: Character assembly | 4-6 hours (manual Blender work) |
| Phase 3: Render script | 3-4 hours (Python/Blender scripting) |
| Phase 4: Bevy integration | 2-3 hours |
| Phase 5: Full lineup test | 1-2 hours |
| **Total** | **~12-16 hours** |
