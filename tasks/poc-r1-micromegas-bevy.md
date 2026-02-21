# PoC R1: Micromegas + Bevy Integration

**Risk**: R1 (Critical) — Architecture doc Section 13
**Goal**: Prove that Micromegas telemetry macros work correctly inside Bevy's parallel ECS systems before writing any game code.

---

## 1. Questions to Answer

1. Can `span_scope!`, `fmetric!`, `imetric!`, `info!` be called from Bevy systems without panics?
2. Do spans work correctly when systems run in parallel across Bevy's thread pool?
3. What happens without per-thread initialization — silent drop or panic?
4. Does telemetry output appear in logs (`LocalEventSink` stdout)?

---

## 2. Research Findings

### Three-channel architecture

Micromegas has three event channels with **different threading models**:

| Channel | Storage | Thread safety | Setup required |
|---------|---------|--------------|----------------|
| **Logs** | Global `Mutex<LogStream>` | Any thread, zero setup | None |
| **Metrics** | Global `Mutex<MetricsStream>` | Any thread, zero setup | None |
| **Spans** | Thread-local `Cell<Option<ThreadStream>>` | Per-thread only | `init_thread_stream()` per thread |

Without `init_thread_stream()`, span events are **silently dropped** — the `on_thread_event` handler checks `if let Some(stream)` and does nothing if `None`. No panic in any case.

**Source**: `micromegas/rust/tracing/src/dispatch.rs` — lines 174-190 (init), 384-397 (silent drop), 553-564 (metrics mutex), 653 (log mutex).

### Bevy thread pool pre-initialization

**Problem**: Bevy's `TaskPoolPlugin` does not expose `on_thread_spawn`/`on_thread_destroy` callbacks.

**Solution**: Call `ComputeTaskPool::get_or_init()` **before** `App::new()`, injecting Micromegas callbacks. `get_or_init` is idempotent — when `TaskPoolPlugin` runs later and calls it again, the pool already exists and the plugin's init closure is skipped.

```rust
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};

ComputeTaskPool::get_or_init(|| {
    TaskPoolBuilder::new()
        .on_thread_spawn(|| {
            micromegas_tracing::dispatch::init_thread_stream();
        })
        .on_thread_destroy(|| {
            micromegas_tracing::dispatch::flush_thread_buffer();
            micromegas_tracing::dispatch::unregister_thread_stream();
        })
        .build()
});
```

**Precedent**: Micromegas uses the same pattern for tokio via `TracingRuntimeExt` in `micromegas/rust/tracing/src/runtime.rs`.

### Test infrastructure

- **`InMemorySink`** (`micromegas-tracing::event::in_memory_sink`) — collects events into mutex-protected vectors. Provides `total_log_events()`, `total_metrics_events()`, `total_thread_events()` counters.
- **`init_in_memory_tracing()`** (`micromegas_tracing::test_utils`) — returns an `InMemoryTracingGuard` that calls `force_uninit()` on drop to reset the global `G_DISPATCH` state.
- **All tests must be `#[serial]`** from `serial_test` crate — there is exactly one global `G_DISPATCH` static shared across the process.
- **`LocalEventSink`** (`micromegas-telemetry-sink::local_event_sink`) — prints colored log messages to stdout. Used in `main.rs` for human-readable console output.

### Version correction

The architecture doc lists `micromegas = "0.14"`. The actual workspace version at `/home/mad/micromegas/` is **0.21.0**. The `micromegas` facade crate pulls the entire stack (analytics, ingestion, auth, datafusion, arrow-flight, axum, sqlx). For the PoC, only `micromegas-tracing` and `micromegas-telemetry-sink` are needed, referenced via path dependencies.

---

## 3. File Structure

```
optimism/
├── Cargo.toml
├── src/
│   ├── main.rs    # Micromegas init + ComputeTaskPool pre-init + Bevy app
│   └── lib.rs     # OptimismPlugin with two parallel PoC systems + tests
```

---

## 4. Dependencies

```toml
[package]
name = "optimism"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = { version = "0.18", default-features = false, features = ["core"] }
micromegas-tracing = { path = "../micromegas/rust/tracing", default-features = false }
micromegas-telemetry-sink = { path = "../micromegas/rust/telemetry-sink" }

[dev-dependencies]
serial_test = "3.2"
```

**Why path deps**: Avoids the 5+ minute compile of the full `micromegas` facade. Only two crates are needed: `micromegas-tracing` (macros, dispatch, test utils) and `micromegas-telemetry-sink` (`TelemetryGuardBuilder`, `LocalEventSink`).

**Why `features = ["core"]`**: Bevy's `core` feature includes `TaskPoolPlugin`, `TimePlugin`, `ScheduleRunnerPlugin`, `FrameCountPlugin` — everything needed for headless execution.

---

## 5. Code Outlines

### `src/main.rs`

```rust
use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
use micromegas_telemetry_sink::TelemetryGuardBuilder;
use micromegas_tracing::dispatch::init_thread_stream;
use micromegas_tracing::prelude::*;

fn main() {
    // 1. Initialize telemetry (creates LocalEventSink for stdout)
    let _telemetry_guard = TelemetryGuardBuilder::default()
        .build()
        .expect("failed to initialize telemetry");

    info!("Optimism PoC starting");

    // 2. Pre-init ComputeTaskPool with Micromegas thread callbacks
    //    Must happen BEFORE App::new() so TaskPoolPlugin finds the pool
    //    already initialized and skips its own init.
    ComputeTaskPool::get_or_init(|| {
        TaskPoolBuilder::new()
            .on_thread_spawn(|| {
                init_thread_stream();
            })
            .on_thread_destroy(|| {
                micromegas_tracing::dispatch::flush_thread_buffer();
                micromegas_tracing::dispatch::unregister_thread_stream();
            })
            .build()
    });

    // 3. Run Bevy app with MinimalPlugins (no window)
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(optimism::OptimismPlugin)
        .run();
}
```

### `src/lib.rs`

```rust
use bevy::prelude::*;
use micromegas_tracing::prelude::*;

pub struct OptimismPlugin;

impl Plugin for OptimismPlugin {
    fn build(&self, app: &mut App) {
        // Two independent systems — Bevy may run them in parallel
        app.add_systems(Update, (system_a, system_b));
    }
}

fn system_a(time: Res<Time>) {
    span_scope!("system_a");
    let dt_ms = time.delta_secs_f64() * 1000.0;
    fmetric!("system_a_dt", "ms", dt_ms);
    imetric!("system_a_tick", "count", 1);
    info!("system_a: dt={:.2}ms", dt_ms);
}

fn system_b(time: Res<Time>) {
    span_scope!("system_b");
    let dt_ms = time.delta_secs_f64() * 1000.0;
    fmetric!("system_b_dt", "ms", dt_ms);
    imetric!("system_b_tick", "count", 1);
    info!("system_b: dt={:.2}ms", dt_ms);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::ScheduleRunnerPlugin;
    use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
    use micromegas_tracing::dispatch::init_thread_stream;
    use micromegas_tracing::test_utils::init_in_memory_tracing;
    use serial_test::serial;
    use std::time::Duration;

    /// Test 1: Micromegas macros don't panic when called from Bevy systems
    /// running on the parallel executor.
    #[test]
    #[serial]
    fn micromegas_macros_dont_panic_in_bevy_systems() {
        let guard = init_in_memory_tracing();

        // Pre-init thread pool with Micromegas callbacks
        ComputeTaskPool::get_or_init(|| {
            TaskPoolBuilder::new()
                .on_thread_spawn(|| { init_thread_stream(); })
                .on_thread_destroy(|| {
                    micromegas_tracing::dispatch::flush_thread_buffer();
                    micromegas_tracing::dispatch::unregister_thread_stream();
                })
                .build()
        });

        // Run for ~5 frames then exit
        let mut frame_count = 0u32;
        App::new()
            .add_plugins(MinimalPlugins.set(
                ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))
            ))
            .add_plugins(OptimismPlugin)
            .add_systems(Update, move |mut exit: EventWriter<AppExit>| {
                frame_count += 1;
                if frame_count >= 5 {
                    exit.send(AppExit::Success);
                }
            })
            .run();

        let sink = guard.sink();
        // 2 systems x 5 frames = at least 10 log events
        assert!(sink.total_log_events() >= 10,
            "expected >= 10 log events, got {}", sink.total_log_events());
        // 2 systems x 2 metrics x 5 frames = at least 20
        assert!(sink.total_metrics_events() >= 20,
            "expected >= 20 metrics events, got {}", sink.total_metrics_events());
    }

    /// Test 2: Spans are silently dropped without per-thread init.
    /// This confirms the safety guarantee — no panics, just silent no-ops.
    #[test]
    #[serial]
    fn spans_silently_dropped_without_thread_init() {
        let guard = init_in_memory_tracing();

        // Do NOT call init_thread_stream()
        {
            span_scope!("test_span");
            info!("this log should work");
        }

        let sink = guard.sink();
        assert!(sink.total_log_events() >= 1,
            "logs should work without thread init");
        assert_eq!(sink.total_thread_events(), 0,
            "spans should be silently dropped without thread init");
    }

    /// Test 3: Logs and metrics work from any thread without init_thread_stream().
    /// Only spans require per-thread setup.
    #[test]
    #[serial]
    fn logs_and_metrics_work_from_any_thread() {
        let guard = init_in_memory_tracing();

        let handles: Vec<_> = (0..4).map(|i| {
            std::thread::spawn(move || {
                // No init_thread_stream() — these should still work
                info!("thread {} reporting", i);
                imetric!("thread_tick", "count", 1);
                fmetric!("thread_value", "units", i as f64);
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }

        let sink = guard.sink();
        assert!(sink.total_log_events() >= 4,
            "expected >= 4 log events from threads, got {}", sink.total_log_events());
        assert!(sink.total_metrics_events() >= 8,
            "expected >= 8 metrics events from threads, got {}", sink.total_metrics_events());
    }
}
```

### Test implementation notes

- Use `>=` not `==` in assertions: `ComputeTaskPool::get_or_init` is idempotent across tests, but thread streams from the first test may be stale in subsequent tests since `on_thread_spawn` only fires once per thread lifetime.
- The frame counter in test 1 may need adjustment — Bevy's `Update` schedule runs once per `ScheduleRunnerPlugin` tick, but the exact count depends on timing. Using `>=` handles this.
- `InMemoryTracingGuard` drops at end of each test, calling `force_uninit()` to reset global dispatch state for the next test.

---

## 6. Success Criteria

| Criterion | Command | Expected |
|-----------|---------|----------|
| All 3 tests pass | `cargo test -- --test-threads=1` | 3 passed, 0 failed |
| Console output from both systems | `cargo run` | `system_a` and `system_b` log lines on stdout |
| No crashes with CPU tracing | `MICROMEGAS_ENABLE_CPU_TRACING=true cargo run` | Same output, no panics |

---

## 7. What This Proves

If all criteria pass:
- Micromegas logs and metrics work from any Bevy worker thread (global mutex channels)
- Micromegas spans work from Bevy worker threads when `ComputeTaskPool` is pre-initialized with thread callbacks
- Spans fail gracefully (silent drop) without thread init — no defensive coding needed in systems
- The `TelemetryGuardBuilder` + `ComputeTaskPool` pre-init pattern is the correct initialization sequence for the full game

If any criterion fails, the project's core value proposition (Micromegas tutorial in a Bevy game) needs to be reconsidered before writing game code.
