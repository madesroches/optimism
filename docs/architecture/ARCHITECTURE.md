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
micromegas = "0.14"
micromegas-tracing = "0.14"
rand = "0.8"
```

### Why Avian2D over bevy_rapier2d

Avian is ECS-native (no dual-world sync), its source is readable (good for a tutorial), and it's the recommended choice for new Bevy projects. We use it lightly — grid-based collision handles gameplay logic; Avian provides wall colliders and sensor triggers as a fallback safety net.

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

At startup, `SpriteGenPlugin` runs in `OnEnter(AppState::Loading)` and generates `Image` assets by writing pixels directly into RGBA buffers. It then creates `TextureAtlas` layouts from them and inserts the handles as resources.

This gives us:
- Full control over the pixel art style
- Deterministic, reproducible visuals
- Zero external art dependencies
- Easy to tweak and iterate

### Sprite categories

**Candide** — A base sprite with overlay layers for each collected luxury item. When Candide picks up a gold grill, chain, Rolex, etc., the corresponding overlay is composited onto his sprite at runtime. The fur coat doubles sprite width.

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
#.####.#####.##.#####.####.#
#..........................#
#.####.##.########.##.####.#
#......##....##....##......#
######.##### ## #####.######
     #.#   G    G #.#
######.# ###--### #.######
      .  #        #  .
######.# ########## #.######
     #.#            #.#
######.# ########## #.######
#............##............#
#.####.#####.##.#####.####.#
#...##.......P .......##...#
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
| 11+ | Chainsaw | Gold toilet |

As levels progress, `enemy_speed_multiplier` increases and `weapon_duration_secs` decreases.

### The Garden (final level)

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
// TelemetryGuardBuilder setup with HTTP sink
// Configurable via environment variables:
//   MICROMEGAS_URL — ingestion endpoint
//   MICROMEGAS_ENABLE_CPU_TRACING — enable span traces
```

The telemetry guard is initialized before the Bevy app starts, ensuring all systems are instrumented from the first frame.

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

## 12. Implementation Order

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

Telemetry instrumentation (step 12) is listed as a discrete step but in practice should be added incrementally as each system is built — this makes the tutorial narrative more natural.

---

## 13. Verification Checklist

- `cargo build` compiles cleanly at each implementation step
- `cargo run` launches the game window with a visible maze and player
- Player can navigate the maze with arrow keys
- Enemies move according to their AI type
- Money collection increments score; collecting all money completes the level
- Weapons activate frightened mode and allow enemy kills
- Luxury items modify Candide's sprite and increase Thief speed
- Pangloss quotes appear on game events
- Game over screen shows stats
- Micromegas telemetry appears in logs
- `MICROMEGAS_ENABLE_CPU_TRACING=1 cargo run` produces span traces
