use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
use micromegas::tracing::dispatch::{
    flush_log_buffer, flush_metrics_buffer, flush_thread_buffer, init_thread_stream,
};
use micromegas::tracing::levels::{self, LevelFilter};
use micromegas::tracing::prelude::*;
use micromegas::tracing::prelude::info;
use micromegas::tracing::test_utils::init_in_memory_tracing;
use serial_test::serial;
use std::time::Duration;

fn telemetry_system_a(time: Res<Time>) {
    span_scope!("telemetry_system_a");
    let dt_ms = time.delta_secs_f64() * 1000.0;
    fmetric!("system_a_dt", "ms", dt_ms);
    imetric!("system_a_tick", "count", 1);
    info!("system_a: dt={:.2}ms", dt_ms);
}

fn telemetry_system_b(time: Res<Time>) {
    span_scope!("telemetry_system_b");
    let dt_ms = time.delta_secs_f64() * 1000.0;
    fmetric!("system_b_dt", "ms", dt_ms);
    imetric!("system_b_tick", "count", 1);
    info!("system_b: dt={:.2}ms", dt_ms);
}

/// Test 1: Micromegas macros don't panic when called from Bevy systems
/// running on the parallel executor, and span events are actually collected.
#[test]
#[serial]
fn micromegas_macros_dont_panic_in_bevy_systems() {
    let guard = init_in_memory_tracing();
    levels::set_max_level(LevelFilter::Trace);

    // Init thread stream on the calling thread — Bevy's multi-threaded
    // executor uses the calling thread as a worker alongside pool threads.
    init_thread_stream();

    // Pre-init thread pool with Micromegas callbacks
    ComputeTaskPool::get_or_init(|| {
        TaskPoolBuilder::new()
            .on_thread_spawn(|| {
                init_thread_stream();
            })
            .on_thread_destroy(|| {
                flush_thread_buffer();
                micromegas::tracing::dispatch::unregister_thread_stream();
            })
            .build()
    });

    // Run for ~5 frames then exit
    let mut frame_count = 0u32;
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
        )
        .add_systems(Update, (telemetry_system_a, telemetry_system_b))
        .add_systems(Update, move |mut exit: MessageWriter<AppExit>| {
            frame_count += 1;
            if frame_count >= 5 {
                exit.write(AppExit::Success);
            }
        })
        .run();

    // Flush the calling thread's span buffer — Bevy's multi-threaded
    // executor runs some systems on the calling thread.
    flush_thread_buffer();

    // Flush span buffers from pool threads — events stay in thread-local
    // buffers until explicitly flushed. flush_thread_buffer() flushes the
    // CURRENT thread's local buffer, so we must run it on each worker.
    // Spawn 2x tasks to increase chance each thread gets at least one.
    let pool = ComputeTaskPool::get();
    pool.scope(|s| {
        for _ in 0..pool.thread_num() * 2 {
            s.spawn(async { flush_thread_buffer(); });
        }
    });

    // Flush log and metrics buffers
    flush_log_buffer();
    flush_metrics_buffer();

    let sink = &guard.sink;
    // 2 systems x 5 frames = at least 10 log events
    assert!(
        sink.total_log_events() >= 10,
        "expected >= 10 log events, got {}",
        sink.total_log_events()
    );
    // 2 systems x 2 metrics x 5 frames = at least 20
    assert!(
        sink.total_metrics_events() >= 20,
        "expected >= 20 metrics events, got {}",
        sink.total_metrics_events()
    );
    // Spans: Bevy distributes systems across a large thread pool. Not all
    // pool threads are used for only 2 systems, so span collection is partial.
    // Assert that at least SOME spans were collected, proving the pattern works.
    assert!(
        sink.total_thread_events() > 0,
        "expected > 0 span events, got {}",
        sink.total_thread_events()
    );
}

/// Test 2: Spans are silently dropped without per-thread init.
/// This confirms the safety guarantee — no panics, just silent no-ops.
#[test]
#[serial]
fn spans_silently_dropped_without_thread_init() {
    let guard = init_in_memory_tracing();
    levels::set_max_level(LevelFilter::Trace);

    // Do NOT call init_thread_stream()
    {
        span_scope!("test_span");
        info!("this log should work");
    }

    flush_log_buffer();

    let sink = &guard.sink;
    assert!(
        sink.total_log_events() >= 1,
        "logs should work without thread init"
    );
    assert_eq!(
        sink.total_thread_events(),
        0,
        "spans should be silently dropped without thread init"
    );
}

/// Test 3: Logs and metrics work from any thread without init_thread_stream().
/// Only spans require per-thread setup.
#[test]
#[serial]
fn logs_and_metrics_work_from_any_thread() {
    let guard = init_in_memory_tracing();
    levels::set_max_level(LevelFilter::Trace);

    let handles: Vec<_> = (0..4)
        .map(|i| {
            std::thread::spawn(move || {
                // No init_thread_stream() — these should still work
                info!("thread {} reporting", i);
                imetric!("thread_tick", "count", 1);
                fmetric!("thread_value", "units", i as f64);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    flush_log_buffer();
    flush_metrics_buffer();

    let sink = &guard.sink;
    assert!(
        sink.total_log_events() >= 4,
        "expected >= 4 log events from threads, got {}",
        sink.total_log_events()
    );
    assert!(
        sink.total_metrics_events() >= 8,
        "expected >= 8 metrics events from threads, got {}",
        sink.total_metrics_events()
    );
}
