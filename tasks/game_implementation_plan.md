# Game Implementation Plan

## Overview

Build Optimism from its current state (validated foundations: telemetry, sprites, audio, headless testing) into a playable Pac-Man-style game. The game has Candide navigating mazes, collecting money, picking up weapons to fight enemies, and collecting luxury items — all narrated by Pangloss with increasingly unhinged optimism.

The implementation is split into 7 phases. Each phase produces a testable, runnable increment. Phases are ordered so that each one builds on the last — no phase requires forward references to unbuilt systems.

## Current State

**What exists:**
- `main.rs` — Micromegas telemetry init + Bevy app bootstrap (MinimalPlugins, no window)
- `lib.rs` — `OptimismPlugin` with placeholder `system_a`/`system_b` demo systems
- `plugins/sprites.rs` — Full sprite sheet loading, texture atlas, animation state machine
- `assets/sprites/` — 5 character sprite sheets (candide_base, soldier, inquisitor, thief, brute) with JSON metadata
- `assets/audio/` — 2 music tracks + 5 SFX (OGG)
- Tests: telemetry (3), headless ECS patterns (8), audio (2), dep compat (1)
- Tools: `render_sprites.py`, `render_all.py`, `generate_audio.py`

**What doesn't exist yet:**
- Game states (`AppState`, `PlayingState`)
- Components (`GridPosition`, `Player`, `Enemy`, etc.)
- Maze loading and rendering
- Player movement and input
- Enemy AI
- Collectibles, weapons, combat
- Audio playback systems
- HUD, narration, menus
- Level progression

## Implementation Phases

---

### Phase 1: App Skeleton — States, Camera, Window

Replace the placeholder `OptimismPlugin` with the real game structure. Get a window open with a camera and state machine driving transitions.

**Steps:**
1. Create `src/app_state.rs` — `AppState` enum (Loading, MainMenu, InGame, GameOver) + `PlayingState` SubStates enum (LevelIntro, Playing, Paused, PlayerDeath, LevelComplete, LevelTransition)
2. Create `src/components.rs` — Start with `GridPosition { x: i32, y: i32 }`, `Player`, `Enemy`, `Wall`, `Money` marker components. Add more as needed in later phases.
3. Create `src/resources.rs` — `Score(u64)`, `CurrentLevel(u32)`, `Lives(u32)`, `LevelConfig`
4. Update `src/lib.rs` — Replace demo systems with real `OptimismPlugin` that adds states, camera, `SpriteSheetPlugin`, and sub-plugins for each phase
5. Create `src/plugins/camera.rs` — Orthographic 2D camera, centered on maze
6. Update `main.rs` — Switch from `MinimalPlugins` to `DefaultPlugins` (with x11) so we get a window
7. Set up `bevy_asset_loader` — Define an `AssetCollection` for audio files (music + SFX). Use `LoadingStateConfig` to drive `AppState::Loading` → `AppState::MainMenu` transition automatically when all assets are loaded. Sprite sheets are loaded manually via `SpriteSheetLibrary::load` (they need the JSON sidecar), so they don't go through `bevy_asset_loader`.

**Tests:**
- State machine transitions (Loading → MainMenu → InGame → GameOver)
- SubStates activate/deactivate with parent
- Camera entity spawns with correct projection

**Files:**
- `src/app_state.rs` (new)
- `src/components.rs` (new)
- `src/resources.rs` (new)
- `src/plugins/camera.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/lib.rs` (rewrite)
- `src/main.rs` (update)

---

### Phase 2: Maze System — Loading, Rendering, Collision Grid

Parse ASCII map files into ECS entities. Render walls and floors using colored rectangles (placeholder visuals — sprite tiles can come later). Build a walkability grid for pathfinding.

**Steps:**
1. Create `assets/maps/level_01.txt` — First maze using the ASCII format from the architecture doc
2. Create `src/plugins/maze.rs` — `MazePlugin`:
   - `MazeMap` resource: 2D grid storing tile types, walkability lookup, spawn positions
   - `load_maze` system (runs `OnEnter(PlayingState::LevelIntro)`): parse text file, spawn `Wall` entities with `GridPosition` + `SpriteBundle` (colored rectangles), spawn `Money` dot entities, record `PlayerSpawn`/`EnemySpawn`/`WeaponSpawn`/`LuxurySpawn` positions
   - `TILE_SIZE` constant (e.g., 64.0) for grid-to-world coordinate conversion
   - `grid_to_world(GridPosition) -> Vec2` helper
3. Wire into camera: auto-center and scale camera to fit the maze dimensions
4. Add a temporary `auto_start_level` system (runs `OnEnter(PlayingState::LevelIntro)`): immediately transitions to `PlayingState::Playing` after maze load completes. This is a development shim so Phases 2–6 are playable with `cargo run`. Phase 7 replaces it with the real level intro screen (show "Level X" + Pangloss quote, then transition on timer/input).

**Tests:**
- Parse a small test maze string → correct entity counts (walls, dots, spawns)
- `MazeMap` walkability: walls are not walkable, dots/empty are
- `grid_to_world` round-trips correctly
- Malformed maps produce errors, not panics

**Files:**
- `assets/maps/level_01.txt` (new)
- `src/plugins/maze.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/plugins/camera.rs` (update — fit to maze)

---

### Phase 3: Player Movement — Input, Grid Logic, Sprite Animation

Get Candide moving through the maze with arrow keys. This is the core movement system that enemies will also use.

**Steps:**
1. Create `src/plugins/movement.rs` — `MovementPlugin`:
   - `MoveLerp { from: Vec2, to: Vec2, t: f32 }` component for smooth visual interpolation
   - `MoveDirection(Direction)` component — current facing
   - `MoveSpeed(f32)` component — tiles per second
   - `InputDirection(Option<Direction>)` component — buffered player input
   - `movement_validation` system: if entity has no active lerp and has a `MoveDirection`, check `MazeMap` walkability for target tile. If valid, update `GridPosition` and create `MoveLerp`.
   - `movement_interpolation` system: advance `MoveLerp::t`, update `Transform`. When `t >= 1.0`, snap to target and remove `MoveLerp`.
   - Pac-Man-style cornering: buffer one input ahead so direction changes feel responsive
2. Create `src/plugins/player.rs` — `PlayerPlugin`:
   - `spawn_player` system (runs `OnEnter(PlayingState::Playing)`): spawn Candide at `PlayerSpawn` position with sprite sheet, `Player` marker, `GridPosition`, `MoveSpeed`, `FacingDirection`
   - `player_input` system: read `ButtonInput<KeyCode>`, set `InputDirection` on player entity
   - `player_input_to_direction` system: convert buffered `InputDirection` into `MoveDirection` when the player arrives at a tile (no active lerp)
3. Wire sprite animation: when `MoveDirection` changes, update the entity's `FacingDirection` (from `sprites.rs`) to match, which drives the walk animation direction in the existing `animate_sprites` system. Add `impl From<Direction> for FacingDirection` to bridge the movement `Direction` enum (in `components.rs`) to the animation `FacingDirection` enum (in `sprites.rs`).

**Tests:**
- Player can move to an empty tile (GridPosition updates)
- Player cannot move into a wall (GridPosition unchanged)
- Input buffering: press direction while moving → applied on arrival at next tile
- Sprite animation matches facing direction
- `Direction` → `FacingDirection` conversion is correct for all 4 variants

**Files:**
- `src/plugins/movement.rs` (new)
- `src/plugins/player.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/plugins/sprites.rs` (update — add `From<Direction> for FacingDirection`)
- `src/components.rs` (update — Direction enum, movement components)

---

### Phase 4: Collectibles and Enemies — Money, AI, Death

Add the core Pac-Man loop: collect all money to win, enemies chase and kill you.

**Steps:**
1. Create `src/plugins/collectibles.rs` — `CollectiblePlugin`:
   - `money_collection` system: when player's `GridPosition` matches a `Money` entity, despawn it, increment `Score`, play `dot_pickup` SFX
   - `check_level_complete` system: when no `Money` entities remain, transition to `PlayingState::LevelComplete`
2. Create `src/ai/mod.rs`, `src/ai/soldier.rs`, `src/ai/inquisitor.rs`, `src/ai/thief.rs`, `src/ai/brute.rs`:
   - Each module exposes `fn choose_target(enemy_pos: GridPosition, player_pos: GridPosition, player_dir: Direction, maze: &MazeMap) -> GridPosition`
   - Soldier: A* directly toward player (fastest)
   - Inquisitor: A* toward a tile 4 squares ahead of player's facing direction
   - Thief: random with bias toward player at close range
   - Brute: A* toward player (slowest)
   - All use `pathfinding::prelude::astar` on `MazeMap`'s walkability grid
3. Create `src/plugins/enemies.rs` — `EnemyPlugin`:
   - `spawn_enemies` system: spawn 4 enemies at `EnemySpawn` positions with sprite sheets, AI type, `MoveSpeed`
   - `enemy_ai` system (runs during `PlayingState::Playing`): for each enemy, call its AI module to get target, set `MoveDirection` toward next step on path
   - `enemy_player_collision` system: when enemy's `GridPosition` matches player's → trigger `PlayerDeath` state, play `death` SFX, decrement `Lives`
   - `handle_player_death` system: if `Lives > 0`, reset positions, transition back to `Playing`. If `Lives == 0`, transition to `AppState::GameOver`.
   - Enemy pen: enemies start in the central pen, released one at a time on a timer

**Tests:**
- Money collection increments score and despawns entity
- All money collected → LevelComplete transition
- Each AI module: given a small maze, returns correct target direction
- Soldier takes shortest path, Inquisitor targets ahead of player
- Enemy collision triggers PlayerDeath
- Lives decrement on death; 0 lives → GameOver

**Files:**
- `src/plugins/collectibles.rs` (new)
- `src/plugins/enemies.rs` (new)
- `src/ai/mod.rs` (new)
- `src/ai/soldier.rs` (new)
- `src/ai/inquisitor.rs` (new)
- `src/ai/thief.rs` (new)
- `src/ai/brute.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/components.rs` (update — Enemy, EnemyKind, AiState)

---

### Phase 5: Weapons and Combat — Power Pellets, Frightened Mode

Add the weapon pickup → frightened mode → enemy kill loop.

**Steps:**
1. Create `src/plugins/combat.rs` — `CombatPlugin`:
   - `WeaponType` enum (BrassKnuckles, Bat, Knife, Axe, Chainsaw)
   - `ActiveWeapon(Option<WeaponType>)`, `WeaponTimer(Timer)` components on player
   - `spawn_weapons` system: place weapon pickups at `WeaponSpawn` positions
   - `weapon_pickup` system: player touches weapon → set `ActiveWeapon`, start `WeaponTimer`, add `Frightened` marker to all enemies, play `power_pellet` SFX
   - `weapon_timer` system: tick timer, on expiry remove `ActiveWeapon` and all `Frightened` markers
   - `player_kills_enemy` system: if player has `ActiveWeapon` and `GridPosition` matches a `Frightened` enemy → despawn enemy, start `Respawning` timer, play `ghost_eaten` SFX
   - `enemy_respawn` system: tick `Respawning` timer, on expiry re-spawn enemy at pen
   - `Frightened` AI override: when frightened, enemies flee (move away from player) instead of chasing

**Tests:**
- Weapon pickup sets ActiveWeapon and adds Frightened to enemies
- Timer expiry removes weapon and Frightened
- Player kills frightened enemy (despawn + respawn timer)
- Player without weapon touching enemy = death (not kill)
- Respawning enemy returns to pen

**Files:**
- `src/plugins/combat.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/components.rs` (update — weapon/combat components)

---

### Phase 6: Audio, HUD, and Narration

Layer on the feedback systems. These are read-only consumers of game state — they don't affect gameplay logic.

**Steps:**
1. Create `src/plugins/audio.rs` — `GameAudioPlugin`:
   - Music channel: loop `menu_theme` on MainMenu, crossfade to `gameplay` on InGame
   - SFX channel: listen for game events and play corresponding sounds
   - Uses `bevy_kira_audio` channels
2. Create `src/plugins/hud.rs` — `HudPlugin`:
   - Bevy UI overlay: score (top-left), lives (top-right), level number (top-center)
   - Updates reactively from `Score`, `Lives`, `CurrentLevel` resources
3. Create `src/plugins/narration.rs` — `NarrationPlugin`:
   - Quote pool per trigger type (money, weapon, death, kill, luxury, level start)
   - Text overlay that fades after a few seconds
   - No repeats of the same quote consecutively
   - Suppressed entirely on the Garden level

**Tests:**
- HUD elements update when resources change
- Narration triggers display text and despawn after timeout
- No consecutive duplicate quotes
- Garden level suppresses narration

**Files:**
- `src/plugins/audio.rs` (new)
- `src/plugins/hud.rs` (new)
- `src/plugins/narration.rs` (new)
- `src/plugins/mod.rs` (update)

---

### Phase 7: Level Progression, Menus, and Polish

Wire up the full game loop from menu to game over, with level escalation.

**Steps:**
1. Create additional maze files (`level_02.txt` through `level_04.txt`, plus `garden.txt`)
2. Update `src/plugins/maze.rs` — `LevelConfig` mapping: level number → maze file, weapon type, luxury type, enemy speed multiplier, weapon duration
3. Create `src/plugins/collectibles.rs` additions — `LuxuryItem` spawning at `LuxurySpawn`, temporary with timeout, visual change on Candide's sprite (swap sprite sheet handle to variant), Thief speed boost on collection
4. Level transitions: `LevelComplete` → `LevelTransition` (cleanup entities) → `LevelIntro` (show "Level X" + Pangloss quote) → `Playing` (load next maze)
5. Main menu: simple "Press Enter to Start" screen
6. Game over: stats display (score, deaths, kills by weapon type, luxuries collected) — *"The best of possible games... considering..."*
7. The Garden (level 13): no enemies, no weapons, no narration, small simple maze
8. Micromegas telemetry: add `span_scope!`, `fmetric!`, `imetric!`, `info!` calls to each plugin per the architecture doc's instrumentation table

**Tests:**
- LevelConfig returns correct weapon/luxury/speed for each level range
- Level transition cleans up old entities and loads new maze
- Garden level spawns no enemies and suppresses narration
- Game over displays correct stats

**Files:**
- `assets/maps/level_02.txt` through `level_04.txt`, `garden.txt` (new)
- `src/plugins/maze.rs` (update — level config)
- `src/plugins/collectibles.rs` (update — luxury items)
- `src/plugins/telemetry.rs` (new — frame instrumentation)
- `src/plugins/mod.rs` (update)
- `src/resources.rs` (update — GameStats)

---

## Files to Modify (Summary)

**New files:**
- `src/app_state.rs`
- `src/components.rs`
- `src/resources.rs`
- `src/plugins/camera.rs`
- `src/plugins/maze.rs`
- `src/plugins/movement.rs`
- `src/plugins/player.rs`
- `src/plugins/collectibles.rs`
- `src/plugins/enemies.rs`
- `src/plugins/combat.rs`
- `src/plugins/audio.rs`
- `src/plugins/hud.rs`
- `src/plugins/narration.rs`
- `src/plugins/telemetry.rs`
- `src/ai/mod.rs`
- `src/ai/soldier.rs`
- `src/ai/inquisitor.rs`
- `src/ai/thief.rs`
- `src/ai/brute.rs`
- `assets/maps/level_01.txt` (+ additional levels)

**Modified files:**
- `src/main.rs`
- `src/lib.rs`
- `src/plugins/mod.rs`

## Trade-offs

**Grid-first collision vs Avian2D-first**: The plan uses `GridPosition` equality as the source of truth for all gameplay collisions. Avian2D is available but not load-bearing — it can be added later as a visual safety net for wall clipping, or dropped entirely if it causes interaction issues with grid logic. This keeps the collision model simple and fully testable headlessly.

**Placeholder tile visuals in Phase 2**: Walls and floors will be colored rectangles initially, not sprite tiles. This lets us validate the maze system without blocking on tile art. Sprite tiles can be swapped in later without changing any logic.

**All 4 enemy AIs in one phase**: Could be split (one AI per phase) but they share the same movement system and spawn logic. Building them together avoids repeated refactoring of `EnemyPlugin`. Each AI module is independently testable regardless.

**Narration/HUD/Audio in one phase**: These are all read-only feedback systems with no gameplay impact. Grouping them keeps the gameplay-critical phases (3-5) focused.

## Testing Strategy

Every phase produces headless tests alongside code, following the patterns validated in PoC R4:

- **Unit tests** (`#[cfg(test)] mod tests`) in each module for pure logic (AI targeting, maze parsing, grid math)
- **Integration tests** (`tests/`) for cross-plugin interactions (player collects money → score updates → level complete triggers)
- `cargo test` must pass before a phase is considered done
- `cargo clippy -- -D warnings` for code quality
- Manual playtesting with `cargo run` after each phase for visual verification

## Open Questions

1. **Luxury item visual effect on Candide** — deferred. The original design called for sprite overlays (grill, chain, Rolex, etc.) but those don't render well with the Quaternius character model. For now, luxury items are collectible for score/Thief-speed mechanics only, with no visual change on Candide. A future design pass will revisit this — possible directions: clothes, armor, or a vehicle instead of kétaine accessories.
2. **Pause menu** — the architecture lists `Paused` as a PlayingState but no design for the pause UI. Minimal approach: dim overlay + "Paused" text + resume on Escape.
