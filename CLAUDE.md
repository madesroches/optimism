# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Optimism** is a Pac-Man-inspired Rust game loosely based on Voltaire's *Candide, ou l'Optimisme*. Its primary purpose is to serve as an example/tutorial demonstrating how to use [Micromegas](https://github.com/madesroches/micromegas) (a telemetry/analytics framework) in the context of a Rust game.

- Game design: `docs/concept/OPTIMISM.md`
- Architecture: `docs/architecture/ARCHITECTURE.md`
- Task tracking: `tasks/` (PoC phases R1–R4b)

## Build and Run

```bash
cargo build                                    # build the game
cargo run                                      # run (MinimalPlugins, no window)
cargo test                                     # run all tests
cargo run --example sprite_test -- soldier     # interactive sprite viewer (needs display)
cargo run --example sprite_validate            # headless sprite metadata validation
```

Bevy is configured for x11 only (no wayland) to avoid `libwayland-dev` dependency. Requires Rust edition 2024.

## Sprite Pipeline (Blender + Python)

The art pipeline renders 3D Quaternius models into 2D sprite sheets. Requires Blender 4.x on PATH and Python 3.x.

```bash
# Assemble .blend files from Quaternius assets (art/quaternius/ → art/characters/)
blender -b -P tools/assemble_characters.py

# Render all characters to sprite sheets (art/characters/ → assets/sprites/)
python3 tools/render_all.py

# Render a single character
blender -b art/characters/soldier.blend -P tools/render_sprites.py -- --output assets/sprites/soldier.png
```

Each character produces a PNG sprite sheet + JSON metadata sidecar in `assets/sprites/`. The JSON describes frame layout and animation ranges (walk x4 directions, idle, attack x4, death).

## Architecture

- **Bevy 0.18** ECS with plugin-based design. Each system is a `Plugin`.
- **Micromegas** telemetry is a first-class concern: initialized before Bevy in `main.rs`, `ComputeTaskPool` pre-init with thread callbacks. Systems use `span_scope!`, `fmetric!`, `imetric!`.
- **Grid-based movement** — discrete tile-to-tile, not continuous physics. Avian2d is a safety net, not the primary collision system.
- **Sprite system** (`src/plugins/sprites.rs`): `SpriteSheetLibrary` loads PNG+JSON, builds `TextureAtlasLayout`, drives frame animation via `AnimationState`/`AnimationTimer`/`FacingDirection` components.

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| bevy 0.18 | Game engine (2D rendering, ECS, UI) |
| micromegas 0.20 | Telemetry: spans, metrics, structured logging |
| avian2d 0.5 | 2D physics (wall colliders, sensor triggers) |
| bevy_kira_audio 0.25 | Audio playback |
| bevy_asset_loader 0.25 | Declarative asset loading with state-driven progress |
| pathfinding 4 | A* for enemy AI |

## Conventions

- Prefer Python over shell scripts for tooling and automation
- Binary art assets (Blender files, Quaternius packs) live in `art/` which is gitignored
- Generated sprite sheets in `assets/sprites/` are committed (small PNGs)
- Characters: Candide (player, cream), Soldier (red), Inquisitor (purple), Thief (gold), Brute (green)
