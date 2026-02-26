# Game Context PropertySet Plan

## Overview

Add a `GameContext` Bevy resource that holds a Micromegas `&'static PropertySet` reflecting the current map. All metrics and tagged logs emitted during gameplay will carry this property set, enabling per-map filtering and aggregation in analytics.

## Current State

Three metrics and four logs are emitted, all untagged:

| File | Line | Call |
|------|------|------|
| `src/plugins/telemetry.rs` | 129 | `fmetric!("frame_time_ms", "ms", dt_ms)` |
| `src/plugins/collectibles.rs` | 53 | `imetric!("score", "points", score.0)` |
| `src/plugins/combat.rs` | 193 | `imetric!("kills", "count", total_kills as u64)` |
| `src/plugins/collectibles.rs` | 52 | `info!("money_collected: score={}", ...)` |
| `src/plugins/combat.rs` | 188 | `info!("enemy_killed: weapon={:?} score={}", ...)` |
| `src/plugins/maze.rs` | 372 | `info!("maze loaded: {} ({}x{})", ...)` |
| `src/main.rs` | 23 | `info!("Optimism PoC starting")` — startup, no map yet |

Maps are loaded per-level from `LevelConfig.maze_file` (5 distinct values: `level_01` through `level_04` plus `garden`). The map name is derived from the file path in `level_config()` in `src/resources.rs`.

## Design

### GameContext resource

```rust
#[derive(Resource)]
pub struct GameContext {
    pub properties: &'static PropertySet,
}
```

Holds a single interned `PropertySet`. Inserted/updated on each level load, read by every system that emits metrics or logs.

### Building the PropertySet

Use `intern_string` from `micromegas_tracing` to convert the dynamic map name to `&'static str`, then create the property set:

```rust
use micromegas_tracing::intern_string::intern_string;
use micromegas_tracing::property_set::{Property, PropertySet};

fn map_name_from_path(path: &str) -> &str {
    // "assets/maps/level_01.txt" → "level_01"
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
}

let name = intern_string(map_name_from_path(&config.maze_file));
let props = PropertySet::find_or_create(vec![
    Property::new("map", name),
]);
GameContext { properties: props }
```

With only 5 distinct map names, interning cardinality is trivially bounded.

### Consuming the context

Systems that emit metrics add `Option<Res<GameContext>>` and pass `ctx.properties` when available:

```rust
if let Some(ref ctx) = game_ctx {
    imetric!("score", "points", ctx.properties, score.0);
} else {
    imetric!("score", "points", score.0);
}
```

For logs, use the `properties:` keyword:

```rust
micromegas_tracing::prelude::info!(
    properties: ctx.properties,
    "money_collected: score={}", score.0
);
```

Wait — the `info!` macro doesn't have a `properties:` arm. It forwards `$($arg:tt)+` to `log!($crate::levels::Level::Info, $($arg)+)` which does match `($lvl:expr, properties: $properties:expr, $($arg:tt)+)`. So `info!(properties: ctx.properties, "msg")` works correctly.

### Lifecycle

- **Insert**: `OnEnter(PlayingState::LevelIntro)`, after `load_maze` (maze and config are available).
- **Available during**: `PlayingState::Playing` — all gameplay systems can read it.
- **Cleanup**: Removed with other game resources on `OnExit(AppState::InGame)`.
- `frame_telemetry` runs every frame including during `LevelIntro`/`LevelTransition`. It should handle `GameContext` being absent via `Option<Res<GameContext>>` and skip tagging when unavailable.

### Where to put it

Add the resource definition and the `update_game_context` system to `src/plugins/telemetry.rs` since it's a telemetry concern, not a gameplay one. Register it in `TelemetryPlugin::build`.

## Implementation Steps

### Step 1: Add `GameContext` resource and update system to `src/plugins/telemetry.rs`

- Define `GameContext` struct with `&'static PropertySet` field.
- Add `update_game_context` system (reads `Res<LevelConfig>`, inserts `GameContext`).
- Add `cleanup_game_context` system for `OnExit(AppState::InGame)`.
- Register in `TelemetryPlugin::build` on `OnEnter(PlayingState::LevelIntro)` after `load_maze`.

### Step 2: Tag `frame_telemetry` in `src/plugins/telemetry.rs`

- Change parameter to `Option<Res<GameContext>>`.
- Use tagged `fmetric!` when context is available, untagged when not.

### Step 3: Tag metrics and logs in `src/plugins/collectibles.rs`

- Add `Option<Res<GameContext>>` parameter to `money_collection`.
- Tag the `imetric!("score", ...)` and `info!("money_collected", ...)` calls when context is available, untagged when not.

### Step 4: Tag metrics and logs in `src/plugins/combat.rs`

- Add `Option<Res<GameContext>>` parameter to `player_kills_enemy`.
- Tag the `imetric!("kills", ...)` and `info!("enemy_killed", ...)` calls when context is available.

### Step 5: Tag the maze-loaded log in `src/plugins/maze.rs`

- Add `Res<LevelConfig>` is already available in `load_maze`. Build a temporary `PropertySet` inline (GameContext isn't inserted yet at this point — this system runs before `update_game_context`). Alternatively, leave this log untagged since it fires once per level load and the map name is already in the message text.

### Step 6: Build and test

- `cargo build` — verify compilation.
- `cargo test` — all tests pass. Unit tests in `collectibles.rs` and `combat.rs` don't insert `GameContext`, so `Option<Res<GameContext>>` resolves to `None` and metrics/logs are emitted untagged. This is correct — tagging is a telemetry concern, not a gameplay one.

## Files to Modify

| File | Change |
|------|--------|
| `src/plugins/telemetry.rs` | Add `GameContext` resource, `update_game_context` system, `cleanup_game_context`, tag `frame_telemetry` |
| `src/plugins/collectibles.rs` | Add `Option<Res<GameContext>>` param to `money_collection`, tag metric + log |
| `src/plugins/combat.rs` | Add `Option<Res<GameContext>>` param to `player_kills_enemy`, tag metric + log |

## Trade-offs

**Resource with `&'static PropertySet` (chosen)** vs **rebuilding PropertySet at each call site**: The resource approach builds the PropertySet once per level transition. Every call site just reads the static reference — zero allocation per metric. Rebuilding at each call site would call `find_or_create` every frame/event, which does a hash lookup. Negligible cost either way, but the resource is cleaner.

**Tagging logs too (chosen)** vs **metrics only**: Logs already contain the map name in their text, so tagging is redundant for human reading. But for programmatic filtering in analytics (e.g. "show all events for level_03"), tagged logs are useful. Minimal cost to include.

**`Option<Res<GameContext>>` for all consumers** vs **mandatory `Res<GameContext>`**: All metric/log-emitting systems use `Option<Res<GameContext>>` rather than mandatory `Res<GameContext>`. This is necessary because (a) `frame_telemetry` runs unconditionally in `Update` where `GameContext` may not exist, and (b) unit tests in `collectibles.rs` and `combat.rs` reach `PlayingState::Playing` without inserting `GameContext` — a mandatory `Res` would cause Bevy to silently skip the system, breaking test assertions. Using `Option` everywhere is the safe, uniform pattern.

**Leaving `maze loaded` log untagged**: The `load_maze` system runs before `update_game_context` in the same `OnEnter` schedule, so `GameContext` doesn't exist yet. We could build a PropertySet inline, but the map path is already in the log message text. Not worth the complexity.

## Testing Strategy

- `cargo build` — compiles with new imports and tagged macro variants.
- `cargo test` — all existing tests pass. All consumers use `Option<Res<GameContext>>`, so systems run normally without a `GameContext` resource (metrics/logs emitted untagged).
- Manual: run with Micromegas sink enabled, verify tagged metric events contain `map=level_01` etc. in the telemetry output.

## Open Questions

None — the approach is straightforward given the bounded set of map names and existing Micromegas API.
