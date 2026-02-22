# Game Implementation Plan

## Overview

Build Optimism from its current state (validated foundations: telemetry, sprites, audio, headless testing) into a playable Pac-Man-style game. The game has Candide navigating mazes, collecting money, picking up weapons to fight enemies, and collecting luxury items — all narrated by Pangloss with increasingly unhinged optimism.

The implementation is split into 7 phases. Each phase produces a testable, runnable increment. Phases are ordered so that each one builds on the last — no phase requires forward references to unbuilt systems.

## Current State

**Phases 1–6 COMPLETE.** Phase 7 is next.

**What exists (Phases 1–5):**
- `src/main.rs` — Micromegas telemetry init + `ComputeTaskPool` pre-init + `DefaultPlugins` (with x11 window) + `OptimismPlugin`
- `src/lib.rs` — `OptimismPlugin`: registers all states, plugins (Camera, Maze, Movement, Player, Collectible, Enemy, Combat, SpriteSheet, GameAudio, Hud, Narration), resources, asset loading, temporary `skip_to_in_game`
- `src/app_state.rs` — `AppState` (Loading, MainMenu, InGame, GameOver) + `PlayingState` SubStates
- `src/components.rs` — `GridPosition`, `Direction`, `MoveDirection`, `MoveSpeed`, `InputDirection`, `MoveLerp`, `Player`, `Enemy`, `EnemyKind`, `InPen`, `Wall`, `Money`, `SpawnPosition`
- `src/resources.rs` — `Score(u64)`, `CurrentLevel(u32)`, `Lives(u32)`, `LevelConfig`, `AudioAssets`
- `src/plugins/camera.rs` — Camera2d with auto-fit to maze dimensions
- `src/plugins/maze.rs` — `MazeMap` resource, ASCII parser, tile rendering, `grid_to_world`, `auto_start_level` shim, `MazeEntity` marker, `PenGate` component
- `src/plugins/movement.rs` — `MovementPlugin`: grid-based movement validation, smooth lerp interpolation, transform sync
- `src/plugins/player.rs` — `PlayerPlugin`: spawn at `PlayerSpawn`, WASD/arrow input, direction buffering, `FacingDirection` ↔ animation bridge
- `src/plugins/collectibles.rs` — `CollectiblePlugin`: money collection (score += 10), level complete when all collected
- `src/plugins/enemies.rs` — `EnemyPlugin`: spawn 4 enemies with sprite sheets, AI dispatch (soldier/inquisitor/thief/brute), player collision → death, pen release timer, death/respawn position reset
- `src/plugins/combat.rs` — `CombatPlugin`: weapon pickups, `ActiveWeapon`/`WeaponTimer`, `Frightened` mode on all enemies, armed player kills frightened enemies, `Respawning` timer → return to pen
- `src/plugins/sprites.rs` — Full sprite sheet loading, texture atlas, animation state machine
- `src/ai/` — 4 AI modules: soldier (A* direct), inquisitor (A* 4-ahead), thief (random + close-range bias), brute (A* direct, slower speed)
- `assets/maps/level_01.txt` — First maze (28x22, fully enclosed, connected, enemy pen with gate)
- `assets/sprites/` — 5 character sprite sheets with JSON metadata
- `assets/audio/` — 2 music tracks + 5 SFX (OGG)
- `docs/level_design_guidelines.md` — Rules for ASCII maze files
- `src/events.rs` — `MoneyCollected`, `WeaponPickedUp`, `EnemyKilled` event structs for Bevy Observers
- `src/plugins/audio.rs` — `GameAudioPlugin`: typed `MusicChannel`/`SfxChannel`, music loops per AppState, SFX via observers + state hooks
- `src/plugins/hud.rs` — `HudPlugin`: score/lives/level text overlay, spawns on InGame, despawns on exit
- `src/plugins/narration.rs` — `NarrationPlugin`: Candide-themed quote pools per trigger, observer-driven + state-driven, money throttled every 5th, 3s display + 1s alpha fade, garden level (13) suppression
- Tests (57 total, all passing): unit tests in maze (10), movement (4), collectibles (2), enemies (3), combat (4), AI (6), audio (1), hud (3), narration (5), plus integration tests (19)

**What doesn't exist yet:**
- Level progression (multiple mazes, level transitions, menus, game over screen)
- Micromegas telemetry instrumentation in game systems

## Implementation Phases

---

### Phase 1: App Skeleton — States, Camera, Window ✓ COMPLETE

Replace the placeholder `OptimismPlugin` with the real game structure. Get a window open with a camera and state machine driving transitions.

**Steps:**
1. Create `src/app_state.rs` — `AppState` enum (Loading, MainMenu, InGame, GameOver) + `PlayingState` SubStates enum with `#[default] LevelIntro` (LevelIntro, Playing, Paused, PlayerDeath, LevelComplete, LevelTransition)
2. Create `src/components.rs` — Start with `GridPosition { x: i32, y: i32 }`, `Player`, `Enemy`, `Wall`, `Money` marker components. Add more as needed in later phases.
3. Create `src/resources.rs` — `Score(u64)`, `CurrentLevel(u32)`, `Lives(u32)`, `LevelConfig`
4. Update `src/lib.rs` — Replace demo systems with real `OptimismPlugin` that adds states, camera, `SpriteSheetPlugin`, and sub-plugins for each phase
5. Create `src/plugins/camera.rs` — Orthographic 2D camera, centered on maze
6. Update `main.rs` — Switch from `MinimalPlugins` to `DefaultPlugins` (with x11) so we get a window
7. Set up `bevy_asset_loader` — Define an `AssetCollection` (derive macro) for audio files (music + SFX). Register via `app.add_loading_state(LoadingState::new(AppState::Loading).continue_to_state(AppState::MainMenu).load_collection::<AudioAssets>())` — this drives the `AppState::Loading` → `AppState::MainMenu` transition automatically when all assets are loaded. Sprite sheets are loaded manually via `SpriteSheetLibrary::load` (they need the JSON sidecar), so they don't go through `bevy_asset_loader`.

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

### Phase 2: Maze System — Loading, Rendering, Collision Grid ✓ COMPLETE

Parse ASCII map files into ECS entities. Render walls and floors using colored rectangles (placeholder visuals — sprite tiles can come later). Build a walkability grid for pathfinding.

**Steps:**
1. Create `assets/maps/level_01.txt` — First maze using the ASCII format from the architecture doc
2. Create `src/plugins/maze.rs` — `MazePlugin`:
   - `MazeMap` resource: 2D grid storing tile types, walkability lookup, spawn positions
   - `load_maze` system (runs `OnEnter(PlayingState::LevelIntro)`): parse text file, spawn `Wall` entities with `GridPosition` + `Sprite::from_color()` (colored rectangles — Bevy 0.18 uses required components, not bundles), spawn `Money` dot entities, record `PlayerSpawn`/`EnemySpawn`/`WeaponSpawn`/`LuxurySpawn` positions. Parse `-` tiles as `PenGate` entities — walkable for enemies but not for the player (see Phase 4 for pen release mechanics). Render gates as a distinct color (e.g., dark pink) to visually distinguish them from walls and floors.
   - `TILE_SIZE` constant (e.g., 64.0) for grid-to-world coordinate conversion
   - `grid_to_world(GridPosition) -> Vec2` helper
3. Wire into camera: auto-center and scale camera to fit the maze dimensions
4. Add a temporary `auto_start_level` system (runs `OnEnter(PlayingState::LevelIntro)`): immediately transitions to `PlayingState::Playing` after maze load completes. This is a development shim so Phases 2–6 are playable with `cargo run`. Phase 7 replaces it with the real level intro screen (show "Level X" + Pangloss quote, then transition on timer/input).
5. **System ordering**: All `OnEnter(PlayingState::LevelIntro)` systems across phases must be explicitly ordered. `load_maze` runs first (it populates `MazeMap`), then `spawn_player` and `spawn_enemies` (added in later phases) run `.after(load_maze)`, then `auto_start_level` runs last via `.after(spawn_player)`. Register ordering in each plugin's `build()` using `.after()`/`.chain()` constraints.

**Tests:**
- Parse a small test maze string → correct entity counts (walls, dots, spawns)
- `MazeMap` walkability: walls are not walkable, dots/empty are, pen gates are walkable for enemies only
- `grid_to_world` round-trips correctly
- Malformed maps produce errors, not panics

**Files:**
- `assets/maps/level_01.txt` (new)
- `src/plugins/maze.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/plugins/camera.rs` (update — fit to maze)

---

### Phase 3: Player Movement — Input, Grid Logic, Sprite Animation ✓ COMPLETE

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
   - `spawn_player` system (runs `OnEnter(PlayingState::LevelIntro)`, ordered `.after(load_maze)`): spawn Candide at `PlayerSpawn` position with sprite sheet, `Player` marker, `GridPosition`, `MoveSpeed`, `FacingDirection`. Spawns once per level alongside the maze — the death/respawn flow resets positions without re-spawning (see Phase 4).
   - `player_input` system: read `ButtonInput<KeyCode>`, set `InputDirection` on player entity
   - `player_input_to_direction` system: convert buffered `InputDirection` into `MoveDirection` when the player arrives at a tile (no active lerp)
3. Wire sprite animation: add `impl From<Direction> for FacingDirection` to bridge the movement `Direction` enum (in `components.rs`) to the animation `FacingDirection` enum (in `sprites.rs`). Add a `sync_facing_to_animation` system that detects when an entity's `FacingDirection` changes and updates `AnimationState.current` accordingly using `resolve_animation_key`/`set_animation` from `sprites.rs`. The existing `animate_sprites` system only advances frames within the current animation range — it does not read `FacingDirection`, so this bridge system is required to switch between directional animation keys (e.g., `walk_down` → `walk_left`).

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

### Phase 4: Collectibles and Enemies — Money, AI, Death ✓ COMPLETE

Add the core Pac-Man loop: collect all money to win, enemies chase and kill you.

**Steps:**
1. Create `src/plugins/collectibles.rs` — `CollectiblePlugin`:
   - `money_collection` system: when player's `GridPosition` matches a `Money` entity, despawn it, increment `Score`, write `MoneyCollected` message
   - `check_level_complete` system: when no `Money` entities remain, transition to `PlayingState::LevelComplete`
2. Create `src/ai/mod.rs`, `src/ai/soldier.rs`, `src/ai/inquisitor.rs`, `src/ai/thief.rs`, `src/ai/brute.rs`:
   - Each module exposes `fn choose_target(enemy_pos: GridPosition, player_pos: GridPosition, player_dir: Direction, maze: &MazeMap) -> GridPosition`
   - Soldier: A* directly toward player (fastest)
   - Inquisitor: A* toward a tile 4 squares ahead of player's facing direction
   - Thief: random with bias toward player at close range
   - Brute: A* toward player (slowest)
   - All use `pathfinding::prelude::astar` on `MazeMap`'s walkability grid
3. Create `src/plugins/enemies.rs` — `EnemyPlugin`:
   - `spawn_enemies` system (runs `OnEnter(PlayingState::LevelIntro)`, ordered `.after(load_maze)`): spawn 4 enemies at `EnemySpawn` positions with sprite sheets, AI type, `MoveSpeed`. All enemies start inside the pen with an `InPen` marker component. Spawns once per level alongside the maze — not on `OnEnter(Playing)`, to avoid duplicates on death/respawn.
   - `enemy_ai` system (runs during `PlayingState::Playing`): for each enemy, call its AI module to get target, set `MoveDirection` toward next step on path
   - `enemy_player_collision` system: when enemy's `GridPosition` matches player's → trigger `PlayerDeath` state, decrement `Lives`
   - `handle_player_death` system: if `Lives > 0`, reset player and enemy `GridPosition` to their spawn positions (do NOT re-enter `Playing` via state transition — use `OnEnter(PlayerDeath)` to reset, then transition to `Playing`). If `Lives == 0`, transition to `AppState::GameOver`. Note: because entities are spawned on `OnEnter(LevelIntro)`, re-entering `Playing` after death does not trigger re-spawning.
   - Enemy pen release: a `PenReleaseTimer` resource ticks during `PlayingState::Playing`. Each tick, the next enemy with an `InPen` marker has it removed, allowing it to move through the `PenGate` tiles and into the maze. Release interval is configurable per level (faster at higher levels). `PenGate` tiles are walkable for entities with `Enemy` but not for `Player` — the `movement_validation` system checks this. Once an enemy exits the pen area, normal AI takes over. Enemies killed during frightened mode respawn back into the pen with `InPen` restored (see Phase 5).

**Tests:**
- Money collection increments score and despawns entity
- All money collected → LevelComplete transition
- Each AI module: given a small maze, returns correct target direction
- Soldier takes shortest path, Inquisitor targets ahead of player
- Enemy collision triggers PlayerDeath
- Lives decrement on death; 0 lives → GameOver
- Pen release timer releases enemies one at a time
- PenGate blocks player movement but allows enemy movement

**Files:**
- `src/plugins/collectibles.rs` (new)
- `src/plugins/enemies.rs` (new)
- `src/ai/mod.rs` (new)
- `src/ai/soldier.rs` (new)
- `src/ai/inquisitor.rs` (new)
- `src/ai/thief.rs` (new)
- `src/ai/brute.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/plugins/movement.rs` (update — PenGate walkability check for Player vs Enemy)
- `src/components.rs` (update — Enemy, EnemyKind, AiState, InPen, PenGate, PenReleaseTimer)

---

### Phase 5: Weapons and Combat — Power Pellets, Frightened Mode ✓ COMPLETE

Add the weapon pickup → frightened mode → enemy kill loop.

**Steps:**
1. Create `src/plugins/combat.rs` — `CombatPlugin`:
   - `WeaponType` enum (BrassKnuckles, Bat, Knife, Axe, Chainsaw)
   - `ActiveWeapon(Option<WeaponType>)`, `WeaponTimer(Timer)` components on player
   - `spawn_weapons` system: place weapon pickups at `WeaponSpawn` positions
   - `weapon_pickup` system: player touches weapon → set `ActiveWeapon`, start `WeaponTimer`, add `Frightened` marker to all enemies, write `WeaponPickup` message
   - `weapon_timer` system: tick timer, on expiry remove `ActiveWeapon` and all `Frightened` markers
   - `player_kills_enemy` system: if player has `ActiveWeapon` and `GridPosition` matches a `Frightened` enemy → hide enemy (remove `Sprite` visibility, disable AI and movement), add `Respawning(Timer)` component, write `EnemyKilled` message. The entity is NOT despawned — it stays alive to hold the respawn timer.
   - `enemy_respawn` system: tick `Respawning` timer on hidden enemies. On expiry, remove `Respawning`, reset `GridPosition` to pen spawn, restore visibility, add `InPen` marker so pen release logic governs re-entry.
   - `Frightened` AI override: when frightened, enemies flee (move away from player) instead of chasing

**Tests:**
- Weapon pickup sets ActiveWeapon and adds Frightened to enemies
- Timer expiry removes weapon and Frightened
- Player kills frightened enemy (hidden + respawn timer ticks + returns to pen)
- Player without weapon touching enemy = death (not kill)
- Respawning enemy returns to pen

**Files:**
- `src/plugins/combat.rs` (new)
- `src/plugins/mod.rs` (update)
- `src/components.rs` (update — weapon/combat components)

---

### Phase 6: Audio, HUD, and Narration ✓ COMPLETE

Layer on the feedback systems. These are read-only consumers of game state — they don't affect gameplay logic.

**Steps:**
1. Create `src/events.rs` — Define `MoneyCollected`, `WeaponPickedUp`, `EnemyKilled` as `#[derive(Event)]` marker structs. Add `commands.trigger()` calls in `collectibles.rs::money_collection`, `combat.rs::weapon_pickup`, and `combat.rs::player_kills_enemy`.
2. Create `src/plugins/audio.rs` — `GameAudioPlugin`:
   - Two typed channels: `MusicChannel`, `SfxChannel` (via `bevy_kira_audio::AudioApp::add_audio_channel`)
   - Music: `OnEnter(AppState::MainMenu)` → loop menu_theme, `OnExit` → stop, `OnEnter(AppState::InGame)` → loop gameplay, `OnExit` → stop
   - SFX via Observers: `MoneyCollected` → dot_pickup, `WeaponPickedUp` → power_pellet, `EnemyKilled` → ghost_eaten
   - SFX via state hooks: `OnEnter(PlayerDeath)` → death, `OnEnter(LevelComplete)` → level_complete
   - Observer signature uses Bevy 0.18 `On<E>` (not `Trigger<E>`)
3. Create `src/plugins/hud.rs` — `HudPlugin`:
   - Full-screen `Node` row (`JustifyContent::SpaceBetween`) with 3 `Text` children
   - Marker components: `HudRoot`, `ScoreText`, `LivesText`, `LevelText`
   - Spawns on `OnEnter(AppState::InGame)`, despawns on `OnExit(AppState::InGame)`
   - Update systems read `Score`, `Lives`, `CurrentLevel` resources
   - NO `MazeEntity` marker — persists across levels
4. Create `src/plugins/narration.rs` — `NarrationPlugin`:
   - Quote pools (const `&[&str]` arrays) per trigger: money, weapon, death, kill, level_start
   - Observer-driven + state-driven triggers (same pattern as audio)
   - Money narration throttled: only every 5th collection (uses `Local<u32>` counter)
   - Fade: 3s display + 1s alpha fade → despawn. New narration replaces old (despawn previous)
   - Garden level (CurrentLevel == 13): all narrations suppressed
   - `NarrationState` resource tracks `last_quote` to avoid consecutive duplicates
5. Fix `src/plugins/camera.rs` — Remove `maze.is_changed()` guard so camera re-fits on window resize

**Tests:**
- Audio plugin initializes, channel resources exist
- HUD spawns on InGame, text updates when Score/Lives/CurrentLevel change, despawns on exit
- Narration `pick_quote` returns from pool, no consecutive duplicates, garden level suppresses, text entity spawns, old narration replaced by new

**Files:**
- `src/events.rs` (new)
- `src/plugins/audio.rs` (new)
- `src/plugins/hud.rs` (new)
- `src/plugins/narration.rs` (new)
- `src/plugins/collectibles.rs` (update — trigger MoneyCollected)
- `src/plugins/combat.rs` (update — trigger WeaponPickedUp, EnemyKilled)
- `src/plugins/camera.rs` (update — remove is_changed guard)
- `src/plugins/mod.rs` (update)
- `src/lib.rs` (update — register events module and 3 new plugins)

---

### Phase 7: Level Progression, Menus, and Polish

Wire up the full game loop from menu to game over, with level escalation.

**Steps:**
1. Create additional maze files (`level_02.txt` through `level_04.txt`, plus `garden.txt`)
2. Update `src/plugins/maze.rs` — `LevelConfig` mapping: level number → maze file, weapon type, luxury type, enemy speed multiplier, weapon duration
3. Create `src/plugins/collectibles.rs` additions — `LuxuryItem` spawning at `LuxurySpawn`, temporary with timeout, Thief speed boost on collection. No visual change on Candide's sprite (deferred — see Open Questions).
4. Level transitions: `LevelComplete` → `LevelTransition` (cleanup entities) → `LevelIntro` (show "Level X" + Pangloss quote) → `Playing` (load next maze)
5. Main menu: simple "Press Enter to Start" screen
6. Game over: stats display (score, deaths, kills by weapon type, luxuries collected) — *"The best of possible games... considering..."*
7. The Garden (level 13): no enemies, no weapons, no narration, small simple maze
8. Micromegas telemetry: add `span_scope!`, `fmetric!`, `imetric!`, `info!` calls to each plugin per the architecture doc's instrumentation table. Specific additions:
   - `player.rs`: log user input direction at debug level via `debug!("player_input: {:?}", direction)` whenever `InputDirection` changes
   - `player.rs`: record player grid position as metrics via `imetric!("player_x", "tile", pos.x)` and `imetric!("player_y", "tile", pos.y)` each frame during `PlayingState::Playing`
9. Update `docs/architecture/ARCHITECTURE.md`: remove the `procgen/` module from the project structure (Section 2), fix the `sprites.rs` description in Section 2 from `SpriteGenPlugin — procedural sprite generation` to `SpriteSheetPlugin — sprite sheet loading and animation`, replace Section 3 (Procedural Art Pipeline) with a description of the Quaternius 3D-to-sprite-sheet pipeline (`tools/render_sprites.py`, `tools/render_all.py`, JSON metadata sidecar format), update the test strategy table in Section 12 to replace the procgen "Sprite generation" row with sprite sheet loading/animation tests, remove "Procedural sprite generation" from Section 14's implementation order, and update Risk R3 to note the procgen approach was abandoned in favor of pre-rendered sprites

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
- `docs/architecture/ARCHITECTURE.md` (update — remove procgen, document Quaternius pipeline, update Risk R3)

---

## Files to Modify (Summary)

**Created (Phases 1–5):**
- `src/app_state.rs` ✓
- `src/components.rs` ✓
- `src/resources.rs` ✓ (will be updated in Phase 7)
- `src/main.rs` ✓
- `src/lib.rs` ✓
- `src/plugins/mod.rs` ✓
- `src/plugins/camera.rs` ✓
- `src/plugins/maze.rs` ✓
- `src/plugins/movement.rs` ✓
- `src/plugins/player.rs` ✓
- `src/plugins/collectibles.rs` ✓ (Phase 7 adds luxury items)
- `src/plugins/enemies.rs` ✓
- `src/plugins/combat.rs` ✓
- `src/plugins/sprites.rs` ✓
- `src/ai/mod.rs` ✓
- `src/ai/soldier.rs` ✓
- `src/ai/inquisitor.rs` ✓
- `src/ai/thief.rs` ✓
- `src/ai/brute.rs` ✓
- `assets/maps/level_01.txt` ✓
- `docs/level_design_guidelines.md` ✓

**Created (Phase 6):**
- `src/events.rs` ✓
- `src/plugins/audio.rs` ✓
- `src/plugins/hud.rs` ✓
- `src/plugins/narration.rs` ✓

**Still to create:**
- `src/plugins/telemetry.rs` (Phase 7)
- `assets/maps/level_02.txt` through `level_04.txt`, `garden.txt` (Phase 7)

## Implementation Notes (Phases 1–6)

Deviations from the original plan, discovered during implementation:

1. **Bevy 0.18 removed `EventWriter`/`EventReader`/`add_event`** — The old event system is gone. Bevy 0.18 uses Observers (`commands.trigger()` / `app.add_observer()`). Phase 6 added the events and trigger calls that Phases 4–5 deferred. Observer system parameter is `On<E>`, not `Trigger<E>`.
2. **`OrthographicProjection` is not a Component** — Bevy 0.18 wraps it in `Projection` enum (which IS a Component). Camera fitting queries `&mut Projection` and matches `Projection::Orthographic(ref mut ortho)`.
3. **`TILE_SIZE` is 32.0**, not 64.0 as originally suggested.
4. **`check_level_complete` guards on `score > 0`** — Prevents false level-complete triggers when the system runs before the maze spawns any money entities.
5. **`From<Direction> for FacingDirection`** lives in `player.rs`, not `sprites.rs`.
6. **AI functions return `Option<Direction>`**, not `GridPosition` as the plan spec'd. The pathfinding returns the direction of the first step, which is what the movement system needs.
7. **`ActiveWeapon(WeaponType)`**, not `ActiveWeapon(Option<WeaponType>)`. Presence/absence of the component replaces the Option.
8. **rand 0.8 API** — Uses `rand::thread_rng()` and `.gen_range()`, not the 0.9+ `rand::rng()` API.
9. **Bevy 0.18 removed `despawn_recursive()`** — `despawn()` now recursively despawns children by default via the relationship system.
10. **WSL2 audio requires PulseAudio** — `bevy_kira_audio` silently drops all audio when no device is available (`AudioManager` initializes as `None`). Install `pulseaudio` package and ensure WSLg socket at `/mnt/wslg/PulseServer` is accessible. See `docs/wsl2-setup.md`.

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
