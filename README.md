# Optimism

A Pac-Man-inspired game loosely based on Voltaire's *Candide, ou l'Optimisme*, built with [Bevy](https://bevyengine.org/) and instrumented with [Micromegas](https://github.com/madesroches/micromegas) telemetry.

Play as **Candide**, navigating mazes, collecting money, and fighting off the Misfortunes — the Soldier, the Inquisitor, the Thief, and the Brute — while **Pangloss** narrates with increasingly unhinged optimism.

Weapons escalate from brass knuckles to chainsaw. Luxury items make you look ridiculous. The final level is a quiet garden with no enemies, no weapons, and no Pangloss.

*"The best of possible games. Or was it?"*

![Optimism gameplay](docs/screenshot.png)

## Purpose

This project serves as a tutorial demonstrating how to integrate [Micromegas](https://github.com/madesroches/micromegas) telemetry into a Rust game. Spans, metrics, and structured logging are woven into every system — not bolted on as an afterthought.

## Tech Stack

| Crate | Purpose |
|-------|---------|
| [Bevy 0.18](https://bevyengine.org/) | Game engine (2D rendering, ECS, UI) |
| [Micromegas 0.20](https://github.com/madesroches/micromegas) | Telemetry: spans, metrics, structured logging |
| [Avian2D 0.5](https://github.com/Jondolf/avian) | 2D physics (wall colliders, sensor triggers) |
| [bevy_kira_audio 0.25](https://github.com/NiklasEi/bevy_kira_audio) | Audio playback |
| [bevy_asset_loader 0.25](https://github.com/NiklasEi/bevy_asset_loader) | Declarative asset loading |
| [pathfinding 4](https://github.com/evenfurther/pathfinding) | A* for enemy AI |

## Build and Run

Requires Rust (edition 2024) and `libasound2-dev` on Linux.

```bash
cargo build
cargo run
cargo test
```

## Art Pipeline

Sprites are pre-rendered from 3D [Quaternius](https://quaternius.com/) models via Blender. Requires Blender 4.x and Python 3.x.

```bash
# Assemble .blend files from Quaternius assets
blender -b -P tools/assemble_characters.py

# Render all characters to sprite sheets
python3 tools/render_all.py
```

## Audio Pipeline

Music (harpsichord via MIDI + FluidSynth) and SFX (numpy waveform synthesis) are procedurally generated. Requires `fluidsynth`, `fluid-soundfont-gm`, and `ffmpeg`.

```bash
pip install -r requirements.txt
python3 tools/generate_audio.py
```

## Documentation

- [Game Design](docs/concept/OPTIMISM.md)
- [Architecture](docs/architecture/ARCHITECTURE.md)
- [Level Design Guidelines](docs/level_design_guidelines.md)

## License

[Apache 2.0](LICENSE)
