# Optimism — Architecture

This document describes the technical architecture for Optimism, a Pac-Man-inspired Rust game based on Voltaire's *Candide, ou l'Optimisme*. The game's primary purpose is to serve as a tutorial demonstrating how to integrate [Micromegas](https://github.com/madesroches/micromegas) telemetry into a Rust game built with Bevy.

**Key constraints:**
- Zero art budget — all visuals are procedural/code-generated pixel art
- Audio is AI-generated (Suno/Udio) and committed as assets
- Bevy 2D, grid-based movement, plugin-based architecture
- Micromegas telemetry is a first-class architectural concern, not bolted on

---

## 1. Tech Stack & Dependencies

```toml
[dependencies]
bevy = "0.18"
avian2d = "0.5"
bevy_kira_audio = "0.24"
bevy_asset_loader = "0.25.0-rc.1"  # stable 0.25 not yet published as of Feb 2026
micromegas = "0.20"
pathfinding = "4"
rand = "0.8"
```

### Why Avian2D over bevy_rapier2d

Avian is ECS-native (no dual-world sync), its source is readable (good for a tutorial), and it's the recommended choice for new Bevy projects. We use it lightly — grid-based collision handles gameplay logic; Avian provides wall colliders and sensor triggers as a fallback safety net.

### Why pathfinding crate

Soldier and Slaver enemies use A* pathfinding. The `pathfinding` crate provides a battle-tested `astar()` function that works directly on grid coordinates. Rolling a custom A* is ~100-150 lines and a distraction from the tutorial focus.

### Why bevy_asset_loader

Declarative asset loading with state-driven progress tracking. Keeps the `Loading` state clean and avoids manual `AssetServer` boilerplate.

---

## 2. Project Structure

```
optimism/
├── Cargo.toml
├── docs/
│   ├── concept/OPTIMISM.md              # Game design document
│   └── architecture/ARCHITECTURE.md     # This document
├── assets/
│   ├── audio/
│   │   ├── music/                       # AI-generated harpsichord tracks
│   │   └── sfx/                         # AI-generated sound effects
│   ├── maps/                            # Plain text maze definitions (.txt)
│   └── fonts/                           # Pixel font for Pangloss quotes
├── src/
│   ├── main.rs                          # Entry point, Micromegas init
│   ├── lib.rs                           # OptimismPlugin (aggregates all plugins)
│   ├── app_state.rs                     # AppState + PlayingState enums
│   ├── components.rs                    # All ECS components
│   ├── resources.rs                     # Game-wide resources
│   ├── plugins/
│   │   ├── mod.rs
│   │   ├── maze.rs                      # MazePlugin — maze parsing, wall colliders
│   │   ├── player.rs                    # PlayerPlugin — Candide movement, input
│   │   ├── enemies.rs                   # EnemyPlugin — 4 AI behaviors
│   │   ├── collectibles.rs             # CollectiblePlugin — money, weapons, luxury items
│   │   ├── combat.rs                    # CombatPlugin — weapon usage, kill/death logic
│   │   ├── narration.rs                # NarrationPlugin — Pangloss quotes
│   │   ├── hud.rs                       # HudPlugin — score, lives, level display
│   │   ├── audio.rs                     # AudioPlugin — music, SFX management
│   │   ├── camera.rs                    # CameraPlugin — 2D camera setup
│   │   ├── sprites.rs                   # SpriteGenPlugin — procedural sprite generation
│   │   └── telemetry.rs                # TelemetryPlugin — Micromegas frame instrumentation
│   ├── ai/
│   │   ├── mod.rs
│   │   ├── soldier.rs                   # Direct pursuit AI
│   │   ├── inquisitor.rs               # Exit-cutting AI
│   │   ├── thief.rs                     # Erratic + money-stealing AI
│   │   └── slaver.rs                    # Slow persistent AI
│   └── procgen/
│       ├── mod.rs
│       ├── candide.rs                   # Candide sprite generation + item overlays
│       ├── enemies.rs                   # Enemy sprite generation (colored variants)
│       ├── tiles.rs                     # Wall/floor/dot tile generation
│       ├── weapons.rs                   # Weapon sprite generation
│       └── items.rs                     # Luxury item sprite generation
```

### Module responsibilities

- **`plugins/`** — Each plugin is a self-contained Bevy `Plugin` that registers its own systems, events, and resources. Plugins communicate through ECS components, resources, and events — never direct function calls between plugins.
- **`ai/`** — Enemy AI logic, separated from the ECS wiring in `plugins/enemies.rs`. Each AI module exposes a function that takes the current game state and returns a target `GridPosition`.
- **`procgen/`** — Procedural sprite generation. Runs once during the `Loading` state. Produces `Handle<Image>` and `TextureAtlasLayout` resources consumed by other plugins.

---

## 3. Procedural Art Pipeline

All game sprites are generated in code. No external image files for game art.

### Approach

At startup, `SpriteGenPlugin` runs in `OnEnter(AppState::Loading)` and generates `Image` assets by writing pixels directly into RGBA buffers. It then creates `TextureAtlas` layouts from them and inserts the handles as resources. Because these assets are created in-memory (not loaded from disk), their handles are immediately valid — no interaction with `bevy_asset_loader`'s async loading. `bevy_asset_loader` manages only the disk-based assets (audio files, fonts, map files); procedural sprites are a separate concern that completes synchronously within `OnEnter`.

This gives us:
- Full control over the pixel art style
- Deterministic, reproducible visuals
- Zero external art dependencies
- Easy to tweak and iterate

### Sprite categories

**Candide** — Pre-composed sprite variants generated at startup, all the same tile-sized dimensions. The base sprite plus one variant per luxury item state (7 total: bare, grill, chain, Rolex, goblet, fur coat, gold toilet). Each variant bakes the luxury item overlay into the sprite at generation time. When Candide collects an item, the game swaps his sprite handle to the corresponding pre-composed variant. This avoids runtime pixel-buffer compositing and keeps collision/layout uniform.

**Enemies** — Color-tinted variants of a base ghost/figure shape. Red (Soldier), Purple (Inquisitor), Yellow (Thief), Green (Slaver). A "frightened" variant uses a shared blue/white palette.

**Tiles** — Wall tiles with solid, border, and corner variants. Floor tiles. Money dot sprites. Generated with simple patterns.

**Weapons** — One sprite per weapon type (brass knuckles, bat, knife, axe, chainsaw). Shown near Candide when active.

**Luxury items** — One sprite per item type (gold grill, chain, Rolex, goblet, fur coat, gold toilet). Shown in the maze at the central spawn point.

---

## 4. Game States

```rust
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
enum AppState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    GameOver,
}

#[derive(SubStates, Clone, Eq, PartialEq, Debug, Hash)]
#[source(AppState = AppState::InGame)]
enum PlayingState {
    #[default]
    LevelIntro,      // "Level X" screen with Pangloss quote
    Playing,         // Active gameplay
    Paused,          // Pause menu
    PlayerDeath,     // Death animation, life lost
    LevelComplete,   // All money collected
    LevelTransition, // Loading next level
}
```

`SubStates` ensure `PlayingState` only exists when `AppState::InGame` is active. This prevents gameplay systems from running during menus or loading.

### State transitions

```
Loading → MainMenu → InGame → GameOver → MainMenu

InGame substates (PlayingState):
  LevelIntro → Playing ⇄ Paused
                  ↓
              PlayerDeath → Playing (lives > 0)
                  ↓ (lives == 0)
              [exits InGame → AppState::GameOver]

  Playing → LevelComplete → LevelTransition → LevelIntro
```

---

## 5. Core ECS Design

### Components (`components.rs`)

```rust
// --- Grid movement ---
struct GridPosition { x: i32, y: i32 }
struct MoveLerp { from: Vec2, to: Vec2, t: f32 }
struct MoveDirection(Direction)
struct MoveSpeed(f32)

// --- Player ---
struct Player;
struct Lives(u32);
struct ActiveWeapon(Option<WeaponType>);
struct CollectedLuxuries(Vec<LuxuryType>);

// --- Enemies ---
struct Enemy;
struct EnemyType(EnemyKind);
struct AiState { /* per-enemy AI working memory */ }
struct Frightened;            // Marker: player has weapon active
struct Respawning(Timer);

// --- Collectibles ---
struct Money;                 // Dots
struct Weapon(WeaponType);    // Power pellets
struct LuxuryItem(LuxuryType);
struct WeaponTimer(Timer);    // Active weapon duration

// --- Maze ---
struct Wall;
struct EnemySpawn;
struct PlayerSpawn;
```

### Resources (`resources.rs`)

```rust
struct Score(u64);
struct CurrentLevel(u32);

struct LevelConfig {
    maze_path: String,
    weapon: WeaponType,
    luxury: LuxuryType,
    enemy_speed_multiplier: f32,
    weapon_duration_secs: f32,
}

struct GameStats {
    kills_by_weapon: HashMap<WeaponType, u32>,
    deaths: u32,
    money_collected: u64,
    luxuries_collected: Vec<LuxuryType>,
}
```

### Enums

```rust
enum Direction { Up, Down, Left, Right }

enum EnemyKind { Soldier, Inquisitor, Thief, Slaver }

enum WeaponType { BrassKnuckles, Bat, Knife, Axe, Chainsaw }

enum LuxuryType { GoldGrill, Chain, Rolex, Goblet, FurCoat, GoldToilet }
```

---

## 6. Grid-Based Movement

No continuous physics for character movement. Characters move tile-to-tile on a discrete grid.

### Movement pipeline (per frame)

1. **Input** — Read keyboard input, set `MoveDirection` on the player entity.
2. **AI decision** — Each enemy AI sets its own `MoveDirection` based on its behavior.
3. **Validation** — Movement system checks if the target tile (current `GridPosition` + `MoveDirection`) is a wall. If clear, update `GridPosition` and create a `MoveLerp` for visual interpolation.
4. **Interpolation** — A separate system advances `MoveLerp::t` each frame and updates `Transform` to smoothly slide the sprite between tiles.
5. **Collision** — `GridPosition` equality checks handle all gameplay collisions: player vs enemy, player vs collectible, player vs weapon pickup, etc.

### Avian2D's role

Avian is a safety net, not the primary collision system:
- **Wall colliders** — Static `Collider` components on wall tiles prevent entities from visually clipping through walls if a bug in grid logic allows it.
- **Sensor triggers** — Collectible tiles use `Sensor` colliders to fire events on overlap. This provides an alternative to pure grid-position checks.

---

## 7. Enemy AI

Each enemy type has a dedicated module in `src/ai/` that implements its pathfinding/targeting logic. The `EnemyPlugin` in `plugins/enemies.rs` wires these into Bevy systems that run during `PlayingState::Playing`.

### Behaviors

**Soldier** (`ai/soldier.rs`) — A* pathfinding directly toward the player's `GridPosition`. Fastest `MoveSpeed`. The straightforward threat.

**Inquisitor** (`ai/inquisitor.rs`) — Targets a position several tiles ahead of the player's current `MoveDirection`. Tries to cut off escape routes rather than chase directly.

**Thief** (`ai/thief.rs`) — Random movement with increasing bias toward the player as distance decreases. On contact: steals money (reduces `Score`) before killing. Collecting a luxury item increases the Thief's `MoveSpeed` for the rest of the level.

**Slaver** (`ai/slaver.rs`) — A* toward player but slowest `MoveSpeed`. On contact: doesn't kill — teleports the player to the maze center and costs a turn. A fate worse than death in arcade terms.

### Frightened mode

When the player picks up a weapon (`ActiveWeapon` becomes `Some`):
- All enemies gain the `Frightened` marker component
- AI reverses: enemies flee from the player
- Contact with an enemy = kill → enemy gets `Respawning` timer and returns to center spawn
- When `WeaponTimer` expires, `Frightened` is removed and normal AI resumes

---

## 8. Level Progression

### Level definitions

Levels are plain text files stored in `assets/maps/`. Each file is a grid of ASCII characters:

```
############################
#............##............#
#.####.#####.##.#####.####.#
#W####.#####.##.#####.####W#
#..........................#
#.####.##.########.##.####.#
#......##....##....##......#
######.##### ## #####.######
     #.#   G    G #.#
######.# ###--### #.######
      .  #  G  G  #  .
######.# ########## #.######
     #.#      L     #.#
######.# ########## #.######
#............##............#
#.####.#####.##.#####.####.#
#W..##.......P .......##..W#
###.##.##.########.##.##.###
#......##....##....##......#
#.##########.##.##########.#
#..........................#
############################
```

Character legend:
- `#` — wall
- `.` — money dot
- ` ` — empty floor
- `P` — player spawn
- `G` — enemy spawn (one per enemy)
- `W` — weapon spawn
- `L` — luxury item spawn
- `-` — enemy pen gate

The `MazePlugin` parses these files at level load, spawning ECS entities for each tile.

Luxury items appear twice per level at the `L` spawn point. Each appearance is temporary — the item despawns after a configurable timeout if not collected.

### Level config mapping

The `CurrentLevel` resource maps to a `LevelConfig` that determines:

| Level | Weapon | Luxury Item |
|-------|--------|-------------|
| 1-2 | Brass knuckles | Gold grill |
| 3-4 | Bat | Chain necklace |
| 5-6 | Knife | Rolex |
| 7-8 | Axe | Goblet |
| 9-10 | Chainsaw | Fur coat |
| 11-12 | Chainsaw | Gold toilet |

As levels progress, `enemy_speed_multiplier` increases and `weapon_duration_secs` decreases.

### The Garden (level 13)

Level 13 triggers the Garden — a special `LevelConfig` with no weapon, no luxury item, and `enemy_speed_multiplier: 0.0` (which the `EnemyPlugin` interprets as "no enemies spawned"). The `NarrationPlugin` checks `CurrentLevel` and suppresses all quotes for this level. The maze file is a small, simple layout with only `#`, `.`, `P`, and ` ` characters — no `G`, `W`, or `L` markers.

After the chainsaw levels — silence. A small, simple maze. No enemies. No weapons. No Pangloss. Just money to collect quietly.

---

## 9. Narration System

`NarrationPlugin` listens for game events and displays Pangloss quotes as text overlays that fade after a few seconds.

### Triggers

```rust
enum NarrationTrigger {
    MoneyCollected,
    WeaponPickup,
    PlayerDeath,
    EnemyKill,
    LuxuryPickup(LuxuryType),
    LevelStart(u32),
    GameOver,
}
```

Each trigger has a pool of quotes (sourced from the game design doc). Quotes are selected semi-randomly, with later levels drawing from increasingly unhinged variants. The narration system never repeats the same quote twice in a row.

---

## 10. Micromegas Telemetry Integration

This is the tutorial's core value. Telemetry is woven into the game code deliberately, not as an afterthought, to demonstrate different instrumentation patterns.

### Initialization (`main.rs`)

```rust
// 1. TelemetryGuardBuilder::default().build() — creates LocalEventSink for stdout
// 2. ComputeTaskPool::get_or_init() with on_thread_spawn → init_thread_stream()
//    Must happen BEFORE App::new() so TaskPoolPlugin uses our pre-initialized pool.
// 3. App::new().add_plugins(MinimalPlugins).add_plugins(OptimismPlugin).run()
//
// Configurable via environment variables:
//   MICROMEGAS_URL — ingestion endpoint
//   MICROMEGAS_ENABLE_CPU_TRACING — enable span traces
```

The telemetry guard and thread pool are initialized before the Bevy app starts. The `ComputeTaskPool` pre-initialization pattern ensures span collection works on all Bevy worker threads. This sequence is validated by the PoC R1 tests.

### Frame-level instrumentation (`plugins/telemetry.rs`)

```rust
span_scope!("frame");
fmetric!("frame_time_ms", "ms", dt.as_secs_f64() * 1000.0);
imetric!("entity_count", "count", world.entities().len());
```

This plugin demonstrates the baseline: wrapping the game loop and emitting per-frame performance metrics.

### Per-plugin instrumentation

Each plugin includes telemetry calls that demonstrate a specific instrumentation pattern. Every instrumented file includes a comment block explaining what the instrumentation measures, how to query it in the Micromegas analytics UI, and an example SQL query.

| Plugin | Instrumentation | What It Teaches |
|--------|----------------|-----------------|
| `player.rs` | `span_scope!("player_movement")`, `info!("player_moved: {:?}", direction)` | Tracing system execution, structured logging |
| `enemies.rs` | `span_scope!("enemy_ai")` per enemy, `imetric!("ai_path_length", "tiles", path.len())` | Per-entity spans, performance metrics |
| `collectibles.rs` | `info!("money_collected: {}", score)`, `imetric!("score", "points", score)` | Event logging, gameplay metrics |
| `combat.rs` | `info!("enemy_killed: {:?} by {:?}", enemy_type, weapon)`, `imetric!("kills", "count", 1)` | Structured event data |
| `narration.rs` | `span_scope!("narration_display")` | UI system tracing |
| `maze.rs` | `span_scope!("maze_load")`, `fmetric!("maze_load_ms", "ms", elapsed.as_secs_f64() * 1000.0)` | Asset loading performance |
| `app_state.rs` | `info!("state_change: {:?} -> {:?}", from, to)` | Lifecycle events |

### Running with telemetry

```bash
# Logs only (no ingestion service needed)
cargo run

# With CPU tracing spans
MICROMEGAS_ENABLE_CPU_TRACING=1 cargo run

# With full ingestion (requires Micromegas service running)
MICROMEGAS_URL=http://localhost:8080 cargo run
```

---

## 11. Audio

AI-generated assets committed to `assets/audio/`.

### Assets

- `music/` — Harpsichord tracks per level range, increasingly frantic. Chainsaw levels: the buzzing drowns the music.
- `sfx/` — Weapon hits, money pickup, death, enemy kill, chainsaw rev.

### Implementation

`AudioPlugin` uses `bevy_kira_audio` with two channels:
- **Music channel** — Loops the current track. Crossfades between tracks on level transitions.
- **SFX channel** — One-shot sounds triggered by game events (money collection, weapon pickup, combat, death).

Music selection is driven by `CurrentLevel`. SFX playback listens for the same events that the narration system uses.

---

## 12. Testing Strategy

This game is AI-generated. Every system gets automated tests. No exceptions.

### Principle

Each implementation step produces tests alongside code. Tests run in CI and are the primary verification mechanism — manual playtesting is secondary. If a system can't be tested automatically, redesign it until it can.

### Test infrastructure

```toml
[dev-dependencies]
# Bevy's built-in test utilities (MinimalPlugins, etc.)
# No additional test framework needed beyond cargo test
```

Bevy systems are testable by constructing a minimal `World`/`App`, inserting components and resources, running the system, and asserting on the resulting state. No window or renderer needed.

### What to test per system

| System | Test coverage | Approach |
|--------|--------------|----------|
| **Maze parsing** | Every tile type parsed correctly; malformed maps produce errors; entity counts match expected | Load map string → run `MazePlugin` setup → assert `GridPosition`, `Wall`, `Money`, spawn marker entities |
| **Grid movement** | Cannot move into walls; moves update `GridPosition`; `MoveLerp` created correctly | Insert player + walls into `World` → run movement system → assert position |
| **Player input** | Direction mapping; queued input buffering | Simulate `ButtonInput<KeyCode>` → run input system → assert `MoveDirection` |
| **Collectibles** | Money increments score; all money collected triggers `LevelComplete`; weapon pickup sets `ActiveWeapon` + `WeaponTimer` | Insert player + collectibles at same `GridPosition` → run collection system → assert `Score`, state changes |
| **Enemy AI** | Soldier A* finds shortest path; Inquisitor targets ahead of player; Thief biases toward player at close range; Slaver pathfinds correctly | Construct small test mazes → run AI function → assert target `GridPosition` / `MoveDirection` |
| **Combat** | Weapon active + enemy contact = enemy killed + `Respawning`; no weapon + enemy contact = player death; Slaver teleports instead of killing | Set up combat scenarios → run combat system → assert outcomes |
| **Frightened mode** | Weapon pickup adds `Frightened` to all enemies; timer expiry removes it; frightened enemies flee from player | Insert entities with/without `Frightened` → run AI → assert flee direction |
| **Narration** | Correct trigger → quote displayed; no repeat of last quote; Garden level suppresses quotes | Fire `NarrationTrigger` event → run narration system → assert UI entity spawned with expected text |
| **Level progression** | Level config maps correctly; speed multiplier increases; weapon duration decreases; Garden level has no enemies | Assert `LevelConfig` values for each level range |
| **Sprite generation** | Buffers have correct dimensions; no panics on generation; color values in expected ranges | Run procgen functions → assert `Image` dimensions and sample pixel values |
| **State transitions** | `Loading` → `MainMenu` → `InGame` → `GameOver` flow; `PlayingState` substates transition correctly | Drive state machine → assert current state after each transition |
| **Telemetry** | Micromegas macros don't panic; metrics emit expected values | Run instrumented systems → assert no panics (telemetry output verified by log inspection) |

### Test organization

```
src/
├── plugins/
│   ├── maze.rs          # #[cfg(test)] mod tests at bottom
│   ├── player.rs        # #[cfg(test)] mod tests at bottom
│   ├── enemies.rs       # #[cfg(test)] mod tests at bottom
│   └── ...
├── ai/
│   ├── soldier.rs       # #[cfg(test)] mod tests at bottom
│   └── ...
tests/
├── integration/
│   ├── level_flow.rs    # Full level lifecycle: load → play → complete
│   └── combat.rs        # Multi-entity combat scenarios
```

Unit tests live in each module (`#[cfg(test)] mod tests`). Integration tests in `tests/` exercise cross-plugin interactions.

### CI gate

`cargo test` must pass before any implementation step is considered complete. `cargo clippy -- -D warnings` enforces code quality.

---

## 13. Risks

Ordered by severity. Risks marked **PoC** need a proof of concept before full implementation begins.

### Critical — could invalidate the project

**R1. Micromegas + Bevy integration** — **PoC DONE** (see `tasks/poc-r1-micromegas-bevy.md`)

The entire tutorial premise depends on `micromegas-tracing` macros (`span_scope!`, `fmetric!`, `imetric!`) working correctly inside Bevy systems. Bevy runs systems in parallel across threads. If Micromegas uses thread-local span stacks that conflict with Bevy's scheduling, or if the telemetry guard doesn't survive across Bevy's app lifecycle, the project's core value proposition fails. No amount of game code matters if the telemetry doesn't work.

*Result: All three channels (logs, metrics, spans) work from Bevy worker threads. Spans require `ComputeTaskPool` pre-initialization with `on_thread_spawn` → `init_thread_stream()`. Without it, spans are silently dropped (no panics). The calling thread also needs `init_thread_stream()` since Bevy uses it as a worker. Bevy `default_app` feature must be avoided (pulls in winit which fails on Rust 1.93); use `multi_threaded` only for headless.*

**R2. Dependency compatibility — bevy_kira_audio 0.24 vs Bevy 0.18** — **PoC**

`bevy_kira_audio = "0.24"` was built for an earlier Bevy version. If it doesn't compile against Bevy 0.18, audio is blocked. Fallback: Bevy's built-in `bevy_audio`, but it lacks the two-channel (music + SFX) architecture described in Section 11. The same concern applies to `bevy_asset_loader = "0.25.0-rc.1"` (release candidate — API could shift).

*PoC: `cargo check` a minimal Cargo.toml with all dependencies listed in Section 1. If anything fails to resolve or compile, identify alternatives before writing game code.*

### High — would require significant rework

**R3. Procedural pixel art quality** — **PoC**

All game visuals are generated by writing pixels into RGBA buffers in Rust code. There's no precedent for an AI coding agent producing recognizable pixel art this way. If sprites look like colored noise, the game is unplayable regardless of how correct the logic is. The Candide variants (7 states), enemy shapes (4 colors + frightened), and tile patterns all need to be visually distinguishable at small sizes.

*PoC: Generate a 16x16 Candide sprite, one enemy sprite, and wall/floor/dot tiles. Render them in a Bevy window on a small test grid. Visual pass/fail: can you tell what things are?*

**R4. Bevy headless testing** — **PoC**

The testing strategy (Section 12) assumes Bevy systems can be unit-tested by constructing a `World`/`App` with `MinimalPlugins`. If system ordering, state transitions, or event propagation behave differently in test vs. real app, tests give false confidence and bugs surface only at runtime. This is especially risky for the state machine (`AppState`/`PlayingState` transitions) and event-driven systems (collectibles, combat).

*PoC: Write a headless test that inserts entities, runs a system, and asserts on ECS state. Confirm state transitions and events propagate correctly without a window.*

### Medium — would require workarounds

**R5. Grid movement + Avian2D interaction**

Two collision systems coexist: grid-based (`GridPosition` checks) and physics-based (Avian `Collider` + `Sensor`). If they disagree — grid logic allows a move but Avian blocks it, or Avian fires a sensor event the grid system already handled — the result is duplicate events, stuck entities, or desync between visual position and grid position. The contract between the two systems needs to be precise.

*Mitigated by: testing grid logic independently first (implementation step 3), adding Avian as a layer on top (not load-bearing). If interaction proves problematic, Avian can be removed entirely — grid logic is the source of truth.*

**R6. Scope creep during late-stage integration**

Steps 1–8 build mostly independent systems. Steps 9–14 (narration, HUD, audio, level progression, polish) integrate across all of them. Late-stage integration could reveal design mismatches — e.g., the narration system needs events that the combat system doesn't emit, or the HUD layout conflicts with the camera setup. Each integration point is a potential rework trigger.

*Mitigated by: automated tests at each step catching regressions, and the plugin architecture limiting blast radius of changes.*

### Low — manageable if encountered

**R7. AI-generated audio asset quality**

The architecture assumes Suno/Udio can produce usable harpsichord tracks and SFX. If output quality is poor or doesn't match the tone, assets need regeneration or the audio system needs simplifying (e.g., single music track, fewer SFX).

*Mitigated by: audio is the last integration step (step 11). The game works silently without it. Audio assets can be iterated independently of code.*

---

## 14. Implementation Order

1. Scaffold Cargo project + Bevy app with states and camera
2. Procedural sprite generation (basic shapes)
3. Grid movement system + maze loading from plain text files
4. Player input and movement
5. Money collection + score
6. Enemy spawning + AI (one type at a time)
7. Weapon system + combat
8. Luxury items + sprite overlays
9. Narration system
10. HUD
11. Audio integration
12. Micromegas telemetry instrumentation (layered in as each system is built)
13. Level progression + game over flow
14. Polish: animations, screen shake, visual effects

Every step produces tests alongside code. Telemetry instrumentation (step 12) is listed as a discrete step but in practice should be added incrementally as each system is built.

---

## 14. Manual Verification Checklist

These supplement automated tests — they cover visual/audio correctness that unit tests cannot:

- `cargo run` launches the game window with a visible maze and player
- Player can navigate the maze with arrow keys
- Enemies move according to their AI type
- Weapons activate frightened mode and allow enemy kills
- Luxury items modify Candide's sprite
- Pangloss quotes appear on game events
- Audio plays correctly
- Micromegas telemetry appears in logs
- `MICROMEGAS_ENABLE_CPU_TRACING=1 cargo run` produces span traces
