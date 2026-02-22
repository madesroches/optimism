# PoC R3: Procedural Pixel Art Quality

**Risk**: R3 (High) — Architecture doc Section 13
**Goal**: Prove that code-generated 16x16 pixel art produces visually recognizable game sprites before committing to a zero-art-budget approach.
**Status**: FAILED — AI-agent-generated procedural sprites lack the quality needed for recognizable game art at 16x16. The rendering pipeline works, but the sprites themselves are not good enough.

---

## 1. Questions to Answer

1. Can an AI coding agent produce recognizable pixel art by writing RGBA buffers in Rust?
2. Are 16x16 sprites large enough to distinguish Candide, enemies, walls, floor, and dots?
3. Does the Bevy `Image` → `Sprite` pipeline render pixel art cleanly when scaled up?
4. Does `ImageSampler::nearest()` preserve sharp pixel edges at 4x scale?

---

## 2. What to Generate

Five sprite categories, each 16x16 pixels:

### Candide (player)
A small humanoid figure. Recognizable features at 16x16:
- Blonde hair (2-3 rows of yellow/gold pixels at the top of the head)
- Skin-colored face with 2 dark eyes
- Blue tunic/coat body
- Dark legs/boots
- Should read as "a person" at a glance

### Enemy — Soldier (red)
A menacing ghost/figure shape, red palette:
- Dome-shaped top (classic arcade ghost silhouette)
- White eyes with dark pupils
- Red body, wavy/jagged bottom edge
- Should read as "threat" and be visually distinct from Candide

### Wall tile
Dark blue/indigo blocks with lighter border:
- Outer 1-pixel border in a lighter blue
- Inner fill in darker blue/indigo
- Should look like solid, impassable terrain

### Floor tile
Near-black background:
- Solid dark color (#111 or similar)
- Visually recessive — walls and characters must pop against it

### Money dot
Small bright circle centered on a floor-colored background:
- 4x4 bright yellow circle centered in the 16x16 tile
- High contrast against the dark floor
- Should read as "thing to collect"

---

## 3. Technical Approach

### Image creation

Use Bevy 0.18's `Image` API to create RGBA8 buffers:

```rust
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::asset::RenderAssetUsages;

fn create_sprite(pixel_data: &[[u8; 4]; 256]) -> Image {
    let size = Extent3d { width: 16, height: 16, depth_or_array_layers: 1 };
    let data: Vec<u8> = pixel_data.iter().flatten().copied().collect();
    let mut image = Image::new(
        size,
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest(); // pixel-perfect scaling
    image
}
```

Setting `ImageSampler::nearest()` is critical — without it, Bevy defaults to linear filtering which blurs pixel art into mush when scaled.

### Sprite definition style

Define each sprite as a 16x16 grid of color indices mapped to a palette. This is more readable and editable than raw RGBA values:

```rust
const CANDIDE: [[u8; 16]; 16] = [
    [0,0,0,0,0,5,5,5,5,5,5,0,0,0,0,0], // row 0: hair top
    [0,0,0,0,5,5,5,5,5,5,5,5,0,0,0,0], // row 1: hair
    // ... 14 more rows
];

const CANDIDE_PALETTE: &[[u8; 4]] = &[
    [0x11, 0x11, 0x11, 0xFF], // 0: background (transparent or dark)
    [0xE8, 0xC1, 0x90, 0xFF], // 1: skin
    [0x22, 0x22, 0x22, 0xFF], // 2: eyes
    [0x44, 0x66, 0xAA, 0xFF], // 3: blue tunic
    [0x33, 0x33, 0x33, 0xFF], // 4: dark boots
    [0xDD, 0xBB, 0x44, 0xFF], // 5: blonde hair
];
```

This pattern:
- Makes sprite shapes easy to see and tweak in code
- Keeps color decisions separate from shape
- Matches the architecture's "deterministic, reproducible visuals" goal
- Is the same approach the full `procgen/` modules will use

### Test grid layout

Render a 9x7 mini-maze to show all tile types in context:

```
#########
#.......#
#.##.##.#
#.#...#.#
#...C.E.#
#.#...#.#
#.......#
#########
```

Where `#` = wall, `.` = money dot on floor, `C` = Candide, `E` = enemy (Soldier).

Each tile is 16x16 pixels, scaled 4x = 64x64 screen pixels. Grid of 9x7 = 576x448 screen pixels. Camera centered on the grid.

### Rendering setup

A single `Startup` system that:
1. Spawns `Camera2d`
2. Generates all 5 sprite images, adds them to `Assets<Image>`
3. Iterates the grid layout string, spawning a `Sprite` entity per tile at the correct `Transform` position
4. For Candide and enemy tiles, spawns two entities: floor underneath + character on top (higher z-order)

Sprite positioning formula:
```rust
let x = col as f32 * TILE_SCREEN_SIZE - grid_width / 2.0 + TILE_SCREEN_SIZE / 2.0;
let y = -(row as f32 * TILE_SCREEN_SIZE - grid_height / 2.0 + TILE_SCREEN_SIZE / 2.0);
```

Scaling: `Transform::from_xyz(x, y, z).with_scale(Vec3::splat(SCALE))`

### No game logic

This PoC has zero gameplay. No input handling, no movement, no states. Just a static scene with sprites rendered in a window. The app runs until closed.

---

## 4. File Structure

```
optimism/
├── src/
│   ├── main.rs          # Modified: windowed Bevy app for visual test
│   └── lib.rs           # Modified: add sprite gen + render alongside existing systems
```

The PoC adds sprite generation and rendering to `lib.rs` while keeping the existing `system_a` and `system_b` systems. Those systems are needed by the telemetry integration tests in `tests/telemetry_integration.rs`, which assert specific log/metric counts produced by them. Removing them would break 1 of the 4 existing tests. They'll be removed later when the game has real systems emitting telemetry.

### What NOT to build

- No `procgen/` module hierarchy yet — that's implementation step 2
- No `TextureAtlas` — individual `Image` handles per sprite type are sufficient for this test
- No `AppState` or loading screens — single `Startup` system
- No `SpriteGenPlugin` — just functions called from setup

---

## 5. Implementation Steps

### Step 1: Update `main.rs` for windowed rendering

Replace `MinimalPlugins` with `DefaultPlugins`. Keep the telemetry guard and `ComputeTaskPool` pre-init from PoC R1 — they're still needed. The window will open with default size.

Add `ImagePlugin::default_nearest()` to `DefaultPlugins` to set nearest-neighbor filtering globally (avoids setting it per-image):

```rust
App::new()
    .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
    .add_plugins(OptimismPlugin)
    .run();
```

### Step 2: Write sprite data in `lib.rs`

Define the five sprites as `const` 16x16 arrays with palette lookups. Each sprite is a function that returns an `Image`:

- `fn create_candide_sprite() -> Image`
- `fn create_soldier_sprite() -> Image`
- `fn create_wall_sprite() -> Image`
- `fn create_floor_sprite() -> Image`
- `fn create_dot_sprite() -> Image`

Helper function to convert palette-indexed grid to `Image`:

```rust
fn sprite_from_grid(grid: &[[u8; 16]; 16], palette: &[[u8; 4]]) -> Image {
    let mut data = Vec::with_capacity(16 * 16 * 4);
    for row in grid {
        for &idx in row {
            data.extend_from_slice(&palette[idx as usize]);
        }
    }
    Image::new(
        Extent3d { width: 16, height: 16, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}
```

### Step 3: Write the grid renderer

A `Startup` system that:
1. Calls each sprite creation function
2. Adds images to `Assets<Image>`, storing handles
3. Parses the 9x7 grid layout string
4. Spawns `Camera2d`
5. Spawns sprite entities at grid positions with 4x scale

### Step 4: Run and visually evaluate

`cargo run` — a window opens showing the mini-maze. Visual pass/fail:

- Can you tell Candide is a person?
- Can you tell the enemy is a different, threatening thing?
- Can you distinguish walls from floor?
- Can you see the money dots?
- Do the sprites look like pixel art (sharp edges) rather than blurred blobs?

### Step 5: Iterate if needed

If sprites don't pass visual inspection, adjust pixel patterns and colors. This is the whole point of the PoC — find out early whether the approach works before building 30+ sprite variants.

### Step 6: Record results

Update this file with:
- Screenshot description or visual assessment
- Any pixel art techniques that worked / didn't work
- Bevy rendering findings (sampler, scaling, z-ordering issues)
- Whether 16x16 is sufficient or if a different size is needed

---

## 6. Dependencies

No new crate dependencies. Everything uses Bevy's built-in `Image`, `Sprite`, and rendering pipeline already in `Cargo.toml`.

System dependencies: the existing `x11` feature requires an X11 display server at runtime. In WSL2, this requires an X server (WSLg provides one by default on Windows 11).

---

## 7. Success Criteria

| Criterion | Method | Expected |
|-----------|--------|----------|
| Window opens with rendered sprites | `cargo run` | Bevy window shows 9x7 grid |
| Candide is recognizable as a character | Visual inspection | Humanoid figure, distinct from enemies |
| Soldier is recognizable as a threat | Visual inspection | Different shape/color from Candide |
| Walls vs floor clearly distinguished | Visual inspection | Walls are solid, floor is dark |
| Money dots visible | Visual inspection | Bright dots on dark floor |
| Pixel art is sharp (not blurred) | Visual inspection | Crisp edges at 4x scale |
| Existing PoC R1 tests still pass | `cargo test -- --test-threads=1` | 4 passed, 0 failed |

---

## 8. Risks Within This PoC

**WSL2 display**: WSLg should provide X11 support, but if the window doesn't open, the PoC can't be evaluated. Fallback: run on a native Linux machine or add screenshot-to-file functionality via Bevy's `screenshot` feature.

**AI sprite quality**: The entire point of this PoC is testing whether an AI agent can design recognizable pixel art. If the first attempt fails, iteration is expected — the plan allows for it in Step 5.

---

## 9. What This Proves

If all criteria pass:
- Procedural pixel art at 16x16 is viable for the full game
- The palette-indexed grid approach works for sprite definition
- Bevy's `Image` → `Sprite` pipeline renders pixel art cleanly
- The architecture's zero-art-budget constraint is feasible

If visual quality fails after iteration:
- Consider increasing sprite size to 24x24 or 32x32
- Consider a hybrid approach (hand-drawn reference sprites stored as assets, code-composed variants)
- Worst case: the zero-art-budget constraint is dropped and external pixel art tools are used

---

## 10. Architecture Doc Updates (post-PoC)

If successful:
1. Section 3: Confirm the `ImageSampler::nearest()` requirement and `ImagePlugin::default_nearest()` approach
2. Section 13 R3: Mark as resolved with findings
3. Section 3: Note the palette-indexed grid pattern if it proves effective
