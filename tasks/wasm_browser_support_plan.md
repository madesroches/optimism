# WASM Browser Support Plan

## Overview

Add the ability to compile and run Optimism in the browser via WebAssembly. The game logic is already platform-agnostic ECS code — the work is removing platform assumptions (x11, multi-threading, filesystem I/O, micromegas telemetry) and adding a web build pipeline.

## Current State

The game compiles exclusively for desktop Linux (x11). Three areas are incompatible with `wasm32-unknown-unknown`:

1. **Cargo.toml**: hardcodes `x11` and `multi_threaded` Bevy features
2. **`std::fs` usage**: `sprites.rs:73-75` and `maze.rs:299` read files directly from disk at runtime
3. **Micromegas telemetry**: `main.rs` initializes thread-local telemetry streams and a stdout sink; `telemetry.rs`, `maze.rs`, `enemies.rs`, `combat.rs`, `collectibles.rs` use `span_scope!`, `fmetric!`, `imetric!`, `info!` macros

Good news: `bevy_kira_audio` supports WASM with OGG format (which is what we use), and `avian2d`, `pathfinding`, `rand`, `serde` are all pure-Rust / WASM-compatible.

### Files using `std::fs`

| File | Lines | Usage |
|------|-------|-------|
| `src/plugins/sprites.rs` | 73-75 | Read JSON sidecar for sprite sheet metadata |
| `src/plugins/maze.rs` | 299 | Read maze map file at level load |
| `src/plugins/maze.rs` | 606, 617 | Tests only (read map files in test assertions) |

### Files using micromegas

| File | Macros used |
|------|-------------|
| `src/main.rs` | `TelemetryGuardBuilder`, `init_thread_stream`, `info!` |
| `src/plugins/telemetry.rs` | `span_scope!`, `fmetric!` |
| `src/plugins/maze.rs` | `span_scope!`, `info!` |
| `src/plugins/enemies.rs` | `span_scope!` |
| `src/plugins/combat.rs` | `imetric!`, `info!` |
| `src/plugins/collectibles.rs` | `imetric!`, `info!` |

## Design

### Strategy: conditional compilation via `cfg(target_arch)`

Use `#[cfg(target_arch = "wasm32")]` / `#[cfg(not(...))]` to split native vs web behavior. No Cargo feature flags needed — the target arch is sufficient and avoids combinatorial complexity.

### Cargo.toml changes

Split Bevy features by target:

```toml
[dependencies]
bevy = { version = "0.18", default-features = false, features = [
    "default_app", "bevy_winit", "std",
    "bevy_render", "bevy_core_pipeline", "bevy_sprite",
    "bevy_sprite_render", "bevy_gizmos_render", "bevy_post_process",
    "ui_api", "ui_bevy_render", "scene", "picking", "default_font",
] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
micromegas = "0.20"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies.bevy]
# can't add features this way — use a crate feature instead
```

Since Cargo doesn't allow per-target features on the same dependency cleanly, the approach is:
- Keep the common Bevy features in `[dependencies]` (remove `x11` and `multi_threaded`)
- Add a crate-level `native` feature that adds `x11` and `multi_threaded`
- Make `micromegas` optional, gated behind `native`
- Default features include `native` so `cargo build` / `cargo run` work unchanged

```toml
[features]
default = ["native"]
native = ["dep:micromegas", "bevy/x11", "bevy/multi_threaded"]
```

### File I/O → Bevy AssetServer

Replace `std::fs::read_to_string` with Bevy's asset loading. Two approaches:

**Sprite JSON metadata** (`sprites.rs`): Load the JSON as a Bevy `Asset<String>` or use `include_str!` at compile time. Since the JSON files are small and static, the simplest approach is to make `SpriteSheetLibrary::load()` accept the JSON string as a parameter instead of reading it from disk, and have the caller provide it (either via asset server or embedded).

**Maze files** (`maze.rs`): Similar — `load_maze` currently calls `std::fs::read_to_string(path)`. Convert `LevelConfig::maze_file` from a filesystem path to an asset path, load via `AssetServer`, and parse once loaded. This requires making maze loading async-friendly (load in one system, parse in another when the asset is ready), or embedding the maps at compile time.

**Recommended: embed at compile time.** The maps and JSON files are tiny (< 5KB each) and committed to the repo. Use `include_str!` to embed them. This avoids async asset loading complexity and works identically on native and WASM.

### Micromegas cfg-gating

Create a thin `telemetry` wrapper module that either calls micromegas (native) or is a no-op (WASM):

```rust
// In telemetry.rs or a new src/telemetry_shim.rs
#[cfg(feature = "native")]
pub use micromegas::tracing::prelude::{span_scope, fmetric, imetric, info};

#[cfg(not(feature = "native"))]
mod stubs {
    macro_rules! span_scope { ($($t:tt)*) => {} }
    macro_rules! fmetric { ($($t:tt)*) => {} }
    macro_rules! imetric { ($($t:tt)*) => {} }
    macro_rules! info { ($($t:tt)*) => {} }
    pub(crate) use {span_scope, fmetric, imetric, info};
}
#[cfg(not(feature = "native"))]
pub use stubs::*;
```

Then change all `use micromegas::tracing::prelude::*` imports to `use crate::telemetry_shim::*`.

### Entry point (`main.rs`)

Gate the telemetry init and thread pool setup behind `#[cfg(feature = "native")]`. The Bevy `App::new()` + `DefaultPlugins` + `OptimismPlugin` code stays the same.

### Build tooling

Use **`wasm-server-runner`** for development (simplest path — just `cargo run --target wasm32-unknown-unknown`). Add a `Trunk.toml` + `web/index.html` for production builds later if needed.

## Implementation Steps

### Phase 1: Cargo.toml + feature gates

1. Add `native` feature to `Cargo.toml` with `default = ["native"]`
2. Make `micromegas` dependency optional, gated behind `native`
3. Remove `x11` and `multi_threaded` from base Bevy features, add them to `native` feature
4. Verify `cargo build` (default features) still works
5. Verify `cargo build --target wasm32-unknown-unknown --no-default-features` compiles (or at least gets past dependency resolution)

### Phase 2: Telemetry shim

1. Create `src/telemetry_shim.rs` with cfg-gated re-exports and no-op stub macros
2. Update imports in `maze.rs`, `enemies.rs`, `combat.rs`, `collectibles.rs`, `telemetry.rs` to use `crate::telemetry_shim` instead of `micromegas::tracing::prelude`
3. Gate `TelemetryPlugin`'s frame telemetry system behind `native` (or have it use the shim — if the macros are no-ops, the system just runs and does nothing, which is fine)
4. Gate `main.rs` telemetry init + thread pool setup behind `#[cfg(feature = "native")]`

### Phase 3: Remove `std::fs` usage

1. **Sprite JSON**: embed metadata via `include_str!` in a lookup function, or change `SpriteSheetLibrary::load()` to accept `&str` JSON content instead of reading from disk
2. **Maze files**: embed map text via `include_str!` in a lookup table keyed by level name, change `LevelConfig::maze_file` from a path to a level identifier, look up embedded content in `load_maze`
3. Update tests in `maze.rs` that use `std::fs` — they can keep using `std::fs` since tests run on native only (`#[cfg(test)]` won't run in WASM)

### Phase 4: WASM build pipeline

1. Install target: `rustup target add wasm32-unknown-unknown`
2. Install runner: `cargo install wasm-server-runner`
3. Create `.cargo/config.toml` with WASM runner config
4. Create `web/index.html` with a canvas element and basic styling
5. Verify the game compiles: `cargo build --target wasm32-unknown-unknown --no-default-features`
6. Verify the game runs in browser: `cargo run --target wasm32-unknown-unknown --no-default-features`

### Phase 5: Browser polish

1. Add canvas resize handling (Bevy's `WindowPlugin` with `fit_canvas_to_parent: true`)
2. Test audio works (Chrome autoplay policy — may need a "click to start" overlay)
3. Add loading screen / progress indicator for WASM asset loading
4. Test all 4 levels + garden in browser

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `native` feature, gate `micromegas`, split Bevy features |
| `src/lib.rs` | Add `pub mod telemetry_shim;` |
| `src/telemetry_shim.rs` | **New** — cfg-gated re-exports + no-op stubs |
| `src/main.rs` | Gate telemetry init + thread pool behind `native` |
| `src/plugins/telemetry.rs` | Use `crate::telemetry_shim` imports |
| `src/plugins/maze.rs` | Use shim imports; embed map files; remove `std::fs` from runtime code |
| `src/plugins/enemies.rs` | Use shim imports |
| `src/plugins/combat.rs` | Use shim imports |
| `src/plugins/collectibles.rs` | Use shim imports |
| `src/plugins/sprites.rs` | Embed JSON metadata; remove `std::fs` |
| `src/resources.rs` | Change `LevelConfig::maze_file` from path to level key |
| `.cargo/config.toml` | **New** — WASM runner config |
| `web/index.html` | **New** — minimal HTML shell for WASM |

## Trade-offs

**Embed vs async asset loading for maps/JSON:**
- Embed (`include_str!`): simpler, no async complexity, works identically everywhere, adds ~20KB to binary. Chosen because the files are small, static, and already committed.
- Async asset loading: more "correct" for Bevy, but requires restructuring the maze loading flow into a multi-system async pipeline. Not worth the complexity for a handful of tiny text files.

**Feature flag (`native`) vs `cfg(target_arch)`:**
- Feature flag: explicit opt-in, works with `--no-default-features`. Chosen because it also lets you test the WASM code path on a native target for debugging.
- `cfg(target_arch = "wasm32")`: automatic but can't easily test WASM codepath on native.

**`wasm-server-runner` vs Trunk:**
- `wasm-server-runner`: zero config, just works with `cargo run --target`. Good for dev.
- Trunk: more control (asset copying, HTML template, WASM optimization). Can add later for production builds. Starting simple.

## Testing Strategy

1. **Native regression**: `cargo build && cargo test` must pass unchanged (default features = native)
2. **WASM compilation**: `cargo build --target wasm32-unknown-unknown --no-default-features` must succeed
3. **Browser smoke test**: game loads, renders the main menu, starts a level, enemies move, audio plays, collectibles work
4. **Cross-browser**: test Chrome (audio autoplay) and Firefox (performance)

## Open Questions

1. **WebGPU vs WebGL2?** Bevy 0.18 defaults to WebGPU with WebGL2 fallback. WebGL2 has wider browser support. Should we force WebGL2 via Bevy's `WgpuSettings`?
2. **Hosting**: is there a target hosting platform (GitHub Pages, itch.io, self-hosted)? This affects the production build pipeline but not the core port.
