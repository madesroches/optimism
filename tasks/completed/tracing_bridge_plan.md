# Bevy Tracing Bridge Plan

**Status: DONE** — Implemented in the `tracing` branch. All 82 tests pass.

## Overview

Fill the "empty space" in traces by bridging Bevy's built-in `tracing` crate instrumentation into Micromegas. Bevy already emits `info_span!("schedule", name = ?self.label)` in `Schedule::run()` for every schedule (behind the `trace` feature flag). By installing a custom `tracing_subscriber::Layer` that translates these span events into Micromegas `on_begin_named_scope` / `on_end_named_scope` calls, we get per-schedule visibility (First, PreUpdate, Update, PostUpdate, etc.) without replacing `run_main` or modifying any Bevy internals.

## Prior State (before this work)

- `src/plugins/telemetry.rs`: Frame sync span wrapping `Main::run_main`, GameSet async subsystem spans within Update
- `#[span_fn]` on all 27+ game systems for per-system sync spans
- `src/main.rs`: Micromegas initialized before Bevy, `LogPlugin` disabled (no global tracing subscriber installed), ComputeTaskPool pre-initialized with Micromegas thread callbacks
- `Cargo.toml`: `bevy/trace` NOT enabled, `tracing`/`tracing-subscriber` not explicit deps (available transitively via Bevy)

The trace timeline showed the Frame span with subsystem spans inside Update, but time spent in First, PreUpdate, PostUpdate, Last, and Bevy's internal systems was invisible.

## Design

### Tracing bridge layer

A `tracing_subscriber::Layer` implementation that:

1. **Filters** to only `"schedule"` spans (metadata name check). Our systems already have `#[span_fn]` — no duplication.
2. **On `on_new_span`**: extracts the `name` field (schedule label debug string) from span attributes via a `Visit` implementation, interns it with `micromegas_tracing::intern_string::intern_string`, stores the interned `&'static str` in span extensions.
3. **On `on_enter`**: looks up the stored name, calls `on_begin_named_scope`.
4. **On `on_exit`**: looks up the stored name, calls `on_end_named_scope`.

All schedule spans enter/exit on the main thread (schedules run sequentially in `run_main`), so sync named spans are correct and efficient.

### Span hierarchy (after implementation)

```
Frame (sync span, existing)
├── schedule:First (bridge, NEW)
├── schedule:PreUpdate (bridge, NEW)
├── schedule:StateTransition (bridge, NEW)
├── schedule:RunFixedMainLoop (bridge, NEW)
├── schedule:Update (bridge, NEW)
│   ├── Player (async subsystem span, existing)
│   │   ├── player_input (#[span_fn], existing)
│   │   └── ...
│   ├── AI (async subsystem span, existing)
│   │   └── ...
│   ├── Movement, Combat, Collectibles, Presentation (existing)
│   └── ...
├── schedule:SpawnScene (bridge, NEW)
├── schedule:PostUpdate (bridge, NEW)
└── schedule:Last (bridge, NEW)
```

Micromegas correlates sync schedule spans with sync system spans and async subsystem spans by time overlap.

## Implementation Steps (all done)

### Step 1: Add dependencies — DONE

In `Cargo.toml`: added `"trace"` to bevy features, `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["registry"] }`.

### Step 2: Create src/tracing_bridge.rs — DONE

New module containing `MicromegasBridgeLayer`, `ScheduleSpanData`, `NameVisitor`, and a shared `static SpanLocation` via `static_span_location!`.

### Step 3: Register module in src/lib.rs — DONE

Added `pub mod tracing_bridge;`.

### Step 4: Install subscriber in src/main.rs — DONE

Installed `Registry::default().with(MicromegasBridgeLayer)` as global tracing subscriber after Micromegas init, before Bevy app creation.

### Step 5: Build and test — DONE

- `cargo build` — compiles cleanly
- `cargo test` — all 82 tests pass (59 unit + 23 integration)

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `"trace"` feature, `tracing`, `tracing-subscriber` deps |
| `src/tracing_bridge.rs` | New — bridge layer implementation |
| `src/lib.rs` | Add `pub mod tracing_bridge` |
| `src/main.rs` | Install tracing subscriber |

## Trade-offs

**Tracing bridge (chosen)** vs **run_main replacement**: The bridge is strictly better — it covers all schedules automatically (including future ones from third-party plugins), requires no Bevy system removal gymnastics, and works with Bevy's existing instrumentation. Removing `Main::run_main` is awkward (`Schedule::remove_systems_in_set` needs `&mut World` inside a `&mut Schedule` closure; `configure_sets` with `run_if(|| false)` doesn't compile on `SystemTypeSet`).

**Schedule spans only (chosen)** vs **schedule + system spans**: Our systems already have `#[span_fn]` which is more efficient (static `SpanMetadata`) than the bridge's dynamic name lookup. Capturing Bevy's "system" spans too would duplicate our `#[span_fn]` spans. We filter to "schedule" only. Can expand to "system" later if we want Bevy-internal system visibility.

**Sync named spans (chosen)** vs **async spans**: Schedule spans enter/exit on the main thread sequentially. Sync spans are more efficient and appropriate. The intern_string call for dynamic schedule label names is a one-time cost per unique label.

## Testing Strategy

- `cargo build` — compiles with trace feature enabled (**verified**)
- `cargo test` — all 82 tests pass; bridge is passive, only active when tracing subscriber is installed (**verified**)
- Run with `MICROMEGAS_ENABLE_CPU_TRACING=true cargo run` — verify schedule span events appear in output alongside existing Frame/subsystem/system spans (TODO: manual verification with display)

## Design Decisions Validated During Implementation

- **Sync spans confirmed correct**: Bevy's `Schedule::run()` uses `info_span!(...).entered()` which is a `!Send` guard — enter and exit always happen on the same thread (main thread for all standard schedules). Sync `on_begin_named_scope` / `on_end_named_scope` is the right choice.
- **`intern_string` is a submodule**: The import is `micromegas_tracing::intern_string::intern_string`, not `micromegas_tracing::intern_string` directly.
