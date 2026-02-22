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

## Phase 1: Download and Inspect Assets

### 1.1 Download Quaternius Packs

All CC0 licensed, all free (Standard tier):

| Pack | URL | What we need |
|------|-----|-------------|
| Universal Base Characters | https://quaternius.itch.io/universal-base-characters | "Regular Male" model (~13k tris, humanoid rig) |
| Universal Animation Library | https://quaternius.itch.io/universal-animation-library | Walk, idle, death animations (45 free, 120+ total) |
| Universal Animation Library 2 | https://quaternius.itch.io/universal-animation-library-2 | Melee combat combos, armed combat (130+ animations) |
| Modular Character Outfits - Fantasy | https://quaternius.itch.io/modular-character-outfits-fantasy | Robes, hoods, armor parts (62 modular pieces, 12 outfits) |
| LowPoly Medieval Weapons | https://quaternius.itch.io/lowpoly-medieval-weapons | 22 weapon models |
| Fantasy Props MegaKit | https://quaternius.itch.io/fantasy-props-megakit | Props for luxury items (200+ models, 94 free) |

Place downloads in `art/quaternius/` (gitignored — binary assets stay out of the repo).

### 1.2 Inspect in Blender

Open the Regular Male GLB/FBX in Blender and verify:
- Bone names and hierarchy (expected: Godot SkeletonProfileHumanoid convention, ~56 bones)
- That animation library FBX files retarget cleanly onto the base rig (same bone names since v2.0)
- Whether walk animations are in-place or root-motion (we need in-place for sprite rendering)
- That modular outfit pieces share the same armature and snap onto the base model
- Triangle count and overall silhouette at top-down orthographic view

**Decision gate**: If the rig or animations are incompatible, stop and assess before continuing.

---

## Phase 2: Character Assembly in Blender

### 2.1 Set Up Base Character Template

Create a Blender file (`art/characters/template.blend`) with:
- Regular Male base mesh with armature
- Animation actions linked from the animation library (walk, idle, attack, death)
- Orthographic camera at top-down ~30-45 degree tilt
- Transparent background render settings
- 64x64 output resolution

### 2.2 Assemble Each Character

Duplicate the template for each character. Differentiate by outfit pieces + material colors:

| Character | Outfit Approach | Primary Color | Distinguishing Feature |
|-----------|----------------|---------------|----------------------|
| **Candide** | Base model, minimal clothing | Neutral/white | Simple, unadorned — the "blank" character |
| **Soldier** | Armor/military outfit pieces from modular pack | Red | Heavy shoulders, weapon-ready pose |
| **Inquisitor** | Robe/cloak outfit pieces | Purple | Long flowing robes, tall silhouette |
| **Thief** | Hood + light armor pieces | Yellow/gold | Hood is the key identifier |
| **Slaver** | Heavy clothing, belt/chains accessories | Green | Bulkier silhouette via outfit layering |

For each character:
1. Start from template.blend
2. Attach appropriate modular outfit pieces (pre-weighted to same skeleton)
3. Assign materials with character-specific colors
4. Verify animations still play correctly with outfit pieces attached
5. Save as `art/characters/{name}.blend`

### 2.3 Candide Luxury Variants

For each luxury item, create a variant of Candide's .blend file:
1. Model or source a simple prop (from Fantasy Props MegaKit or quick manual modeling)
2. Parent the prop to the appropriate bone (hand bone for goblet/Rolex, head for grill, spine for chain/coat)
3. Save as `art/characters/candide_{item}.blend`

| Variant | Prop | Parent Bone |
|---------|------|-------------|
| candide_grill | Simple gold mouth piece | Head |
| candide_chain | Oversized necklace | Neck/UpperChest |
| candide_rolex | Wristwatch/bracelet | LeftHand |
| candide_goblet | Goblet held overhead | RightHand |
| candide_furcoat | Puffy coat mesh | Chest/Spine |
| candide_toilet | Gold toilet carried | RightHand |

### 2.4 Weapon and Item Sprites

Weapons and luxury items also need sprite sheets for pickup display:
- Source from LowPoly Medieval Weapons + Fantasy Props MegaKit
- For weapons not in the pack (brass knuckles, chainsaw): quick manual models or find on OpenGameArt/itch.io (CC0)
- Render as static sprites (no animation needed), single 64x64 frame each
- Composite onto single sheets: `weapons.png` (5 items), `items.png` (6 items)

---

## Phase 3: Blender Headless Rendering Script ✓

### 3.1 Write `tools/render_sprites.py` ✓

**DONE.** Python script that Blender runs headless to produce sprite sheets:

```
blender -b art/characters/soldier.blend -P tools/render_sprites.py -- --output assets/sprites/soldier.png
```

The script:
1. Finds the armature and its animation actions (fuzzy name matching)
2. For each animation (walk, idle, attack, death):
   - Sets the active action
   - For directional animations (walk, attack): rotates model 0°/90°/180°/270° and renders each frame
   - For non-directional (idle, death): renders frames at default rotation
3. Renders each frame at 64x64 with transparent background (EEVEE)
4. Composites all frames into a single sprite sheet (configurable columns)
5. Writes per-character JSON metadata alongside the PNG

CLI options: `--size`, `--columns`, `--camera-angle`, `--camera-distance`, `--outline` (Freestyle)

### 3.2 Write `tools/render_all.py` ✓

**DONE.** Python script that renders all characters:

```
python3 tools/render_all.py
```

### 3.3 Render and Validate

Run the batch script. Inspect each sprite sheet:
- Characters distinguishable from each other at 64x64?
- Walk cycle reads as walking (not sliding/glitching)?
- Attack animation has clear motion?
- Death animation reads clearly?
- Outfit pieces don't clip or deform badly during animation?

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
├── quaternius/                   # downloaded asset packs
├── characters/
│   ├── template.blend           # base character template
│   ├── candide_base.blend
│   ├── candide_grill.blend
│   ├── candide_chain.blend
│   ├── candide_rolex.blend
│   ├── candide_goblet.blend
│   ├── candide_furcoat.blend
│   ├── candide_toilet.blend
│   ├── soldier.blend
│   ├── inquisitor.blend
│   ├── thief.blend
│   └── slaver.blend

tools/                            # committed
├── render_sprites.py            # Blender headless rendering script
└── render_all.py                # batch render all characters

assets/sprites/                   # committed — final output
├── candide_base.png             # sprite sheet (columns x rows, configurable)
├── candide_base.json            # per-character JSON metadata sidecar
├── candide_grill.png
├── candide_grill.json
├── ...                          # (one .png + .json pair per character/variant)
├── soldier.png
├── soldier.json
├── weapons.png                  # 5 weapons, static sprites
├── items.png                    # 6 luxury items, static sprites
└── tiles.png                    # wall/floor/dot tiles

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
