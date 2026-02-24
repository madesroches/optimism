# Subsystem Span Tracing Plan

**Status: SUPERSEDED** — The subsystem async spans described here are already
implemented (commit 5dab9d0). Schedule-level visibility is now provided by the
Bevy tracing bridge (`src/tracing_bridge.rs`) which automatically captures all
Bevy schedule spans via `bevy/trace`, making the approach below unnecessary for
schedule coverage. See `tasks/tracing_bridge_plan.md` for the current approach.

---

## Overview

Add subsystem-level async spans that group related Bevy systems under named categories (Player, AI, Movement, Combat, Collectibles, Presentation) without constraining parallelism. Individual systems keep their existing `#[span_fn]` sync spans. The Micromegas analysis tool correlates async and sync spans by time overlap, producing a trace hierarchy like:

```
Frame (sync, Main schedule)
├── Player (async span)
│   ├── player_input (sync, worker thread)
│   ├── apply_player_direction (sync, worker thread)
│   └── sync_facing_to_animation (sync, worker thread)
├── AI (async span)
│   ├── enemy_ai (sync, worker thread)
│   ├── enemy_player_collision (sync, worker thread)
│   └── pen_release (sync, worker thread)
├── Movement (async span)
│   └── ...
├── Combat (async span)
│   └── ...
├── Collectibles (async span)
│   └── ...
└── Presentation (async span)
    └── ...
```

## Current State

**Frame span** (`src/plugins/telemetry.rs`): wraps `Main::run_main` with sync begin/end scope calls. Works because `Main` is single-threaded.

**Per-system spans**: all 27 `Update` systems have `#[span_fn]` producing sync thread-local events. These are flat siblings under the Frame span — no intermediate grouping.

**No SystemSets**: systems are registered directly on `Update` with per-system `.after()` ordering. No `SystemSet` types exist yet.

## Design

### Span boundary systems

Each subsystem gets a begin/end system pair. These systems have **no ECS parameters** — they use a `static AtomicU64` for span_id handoff. This means Bevy's scheduler sees zero data access conflicts, preserving all parallelism.

```rust
static_span_desc!(AI_SPAN, "AI");
static AI_SPAN_ID: AtomicU64 = AtomicU64::new(0);

fn begin_ai_span() {
    let id = on_begin_async_scope(&AI_SPAN, 0, 0);
    AI_SPAN_ID.store(id, Ordering::Release);
}

fn end_ai_span() {
    let id = AI_SPAN_ID.load(Ordering::Acquire);
    on_end_async_scope(id, 0, &AI_SPAN, 0);
}
```

The `parent_span_id=0` and `depth=0` are fine — the analysis tool correlates by time overlap, not explicit parent links between sync and async spans.

### SystemSet enum

```rust
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    Player,
    AI,
    Movement,
    Combat,
    Collectibles,
    Presentation,
}
```

Defined in `src/plugins/telemetry.rs` and imported by each plugin.

### Ordering constraints

Within each subsystem, only:
```
begin_X → GameSet::X → end_X
```

**No cross-subsystem ordering.** All six subsystems can overlap freely. Existing intra-system ordering (e.g. `movement_interpolation.after(movement_validation)`) is preserved — `.in_set()` is additive to `.before()`/`.after()`.

### Subsystem groupings

| GameSet | Systems | run_if |
|---------|---------|--------|
| **Player** | player_input, apply_player_direction, sync_facing_to_animation | PlayingState::Playing |
| **AI** | enemy_ai, enemy_player_collision, pen_release | PlayingState::Playing |
| **Movement** | movement_validation, movement_interpolation, sync_transform_to_grid | PlayingState::Playing |
| **Combat** | weapon_pickup, weapon_timer, player_kills_enemy, enemy_respawn | PlayingState::Playing |
| **Collectibles** | money_collection, luxury_collection, luxury_timeout, check_level_complete | PlayingState::Playing |
| **Presentation** | animate_sprites, update_hud, fade_narration, fit_camera_to_maze | unconditional* |

*Presentation systems have mixed run_if conditions. The begin/end span systems for Presentation run unconditionally; individual systems retain their own run_if. An empty span (~40ns) is emitted when no presentation systems actually run — harmless and informative.

**Not grouped** (state-transition UI, low frequency):
- menu_input (MainMenu)
- game_over_input (GameOver)
- level_intro_input (LevelIntro)
- level_complete_delay (LevelComplete)

These are single systems gated to specific states. Their existing `#[span_fn]` is sufficient.

### Boilerplate reduction

A declarative macro generates the static, begin fn, and end fn per subsystem:

```rust
macro_rules! define_subsystem_span {
    ($name:ident, $label:expr) => {
        static_span_desc!(paste::paste!{[<$name _SPAN>]}, $label);
        static paste::paste!{[<$name _SPAN_ID>]}: AtomicU64 = AtomicU64::new(0);

        fn paste::paste!{[<begin_ $name:lower _span>]}() {
            let id = on_begin_async_scope(
                &paste::paste!{[<$name _SPAN>]}, 0, 0);
            paste::paste!{[<$name _SPAN_ID>]}
                .store(id, Ordering::Release);
        }

        fn paste::paste!{[<end_ $name:lower _span>]}() {
            let id = paste::paste!{[<$name _SPAN_ID>]}
                .load(Ordering::Acquire);
            on_end_async_scope(
                id, 0, &paste::paste!{[<$name _SPAN>]}, 0);
        }
    };
}
```

This avoids the `paste` dependency by just writing the 6 triplets out directly — 18 lines each, ~108 lines total. The macro is optional; either approach is fine.

## Implementation Steps

### Step 1: Define GameSet and span infrastructure in telemetry.rs

Add to `src/plugins/telemetry.rs`:
- `GameSet` enum (public, used by all plugins)
- Six static `SpanMetadata` + `AtomicU64` pairs
- Six begin/end function pairs
- Register begin/end systems with `.before(GameSet::X)` / `.after(GameSet::X)` ordering
- Apply `run_if(in_state(PlayingState::Playing))` to begin/end systems for Player, AI, Movement, Combat, Collectibles
- Presentation begin/end systems run unconditionally

### Step 2: Add .in_set() to each plugin

Modify each plugin's `build()` to add `.in_set(GameSet::X)` to its Update systems:

- `src/plugins/player.rs` — `.in_set(GameSet::Player)`
- `src/plugins/enemies.rs` — `.in_set(GameSet::AI)`
- `src/plugins/movement.rs` — `.in_set(GameSet::Movement)`
- `src/plugins/combat.rs` — `.in_set(GameSet::Combat)`
- `src/plugins/collectibles.rs` — `.in_set(GameSet::Collectibles)`
- `src/plugins/sprites.rs` — `.in_set(GameSet::Presentation)` (animate_sprites)
- `src/plugins/hud.rs` — `.in_set(GameSet::Presentation)` (update_hud)
- `src/plugins/narration.rs` — `.in_set(GameSet::Presentation)` (fade_narration)
- `src/plugins/camera.rs` — `.in_set(GameSet::Presentation)` (fit_camera_to_maze)

Existing `.after()` / `.run_if()` chains are unchanged — `.in_set()` is appended.

### Step 3: Verify

- `cargo build` — confirm compilation
- `cargo test` — confirm no ordering ambiguities or panics
- Run with `MICROMEGAS_ENABLE_CPU_TRACING=true` and inspect output for subsystem async span events interleaved with per-system sync span events

## Files to Modify

| File | Change |
|------|--------|
| `src/plugins/telemetry.rs` | GameSet enum, span statics, begin/end systems, configure_sets |
| `src/plugins/player.rs` | `.in_set(GameSet::Player)` |
| `src/plugins/enemies.rs` | `.in_set(GameSet::AI)` |
| `src/plugins/movement.rs` | `.in_set(GameSet::Movement)` |
| `src/plugins/combat.rs` | `.in_set(GameSet::Combat)` |
| `src/plugins/collectibles.rs` | `.in_set(GameSet::Collectibles)` |
| `src/plugins/sprites.rs` | `.in_set(GameSet::Presentation)` |
| `src/plugins/hud.rs` | `.in_set(GameSet::Presentation)` |
| `src/plugins/narration.rs` | `.in_set(GameSet::Presentation)` |
| `src/plugins/camera.rs` | `.in_set(GameSet::Presentation)` |

## Trade-offs

**SystemSets + async spans (chosen)** vs **sub-schedules + exclusive systems**: Sub-schedules would give perfectly nested sync spans but serialize subsystems, losing cross-subsystem parallelism. The async span approach preserves all parallelism at the cost of relying on time-based correlation in the analysis tool — which is how Micromegas already works.

**Grouping the 4 UI systems** vs **leaving them ungrouped**: UI systems (menu_input, game_over_input, level_intro_input, level_complete_delay) run in mutually exclusive states and are low-frequency. A subsystem span adds overhead without insight. Left ungrouped.

**Macro vs manual span definitions**: Manual repetition is ~108 lines but has zero dependencies and is fully transparent. A `paste`-based macro saves lines but adds a proc-macro dependency. Plan writes them manually; can be refactored to a macro later if more subsystems are added.

## Performance

- Each begin/end system: ~40ns (one atomic op + one event write to thread-local buffer)
- 12 boundary systems total (6 begin + 6 end), adding ~480ns per frame
- Zero impact on Bevy's parallelism analysis — boundary systems have no ECS parameters
- No new allocations, no new resources, no new components

## Testing Strategy

- `cargo test` passes (no schedule ambiguities)
- Run game with `MICROMEGAS_ENABLE_CPU_TRACING=true`, verify stdout contains `BeginAsyncSpanEvent` / `EndAsyncSpanEvent` for each subsystem alongside existing `BeginThreadSpanEvent` / `EndThreadSpanEvent` for individual systems
- Spot-check that subsystem async spans temporally contain their child sync spans

## Open Questions

None — approach was validated in discussion before planning.
