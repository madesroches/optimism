use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
use micromegas::telemetry_sink::TelemetryGuardBuilder;
use micromegas::tracing::dispatch::init_thread_stream;
use micromegas::tracing::prelude::info;

fn main() {
    // 1. Initialize telemetry (creates LocalEventSink for stdout)
    //    Note: spans require MICROMEGAS_ENABLE_CPU_TRACING=true (env var).
    //    Without it, init_thread_stream() is a no-op and spans are silently dropped.
    //    Logs and metrics always work regardless.
    let _telemetry_guard = TelemetryGuardBuilder::default()
        .build()
        .expect("failed to initialize telemetry");

    info!("Optimism PoC starting");

    // 2. Pre-init ComputeTaskPool with Micromegas thread callbacks.
    //    Must happen BEFORE App::new() so TaskPoolPlugin finds the pool
    //    already initialized and skips its own init.
    ComputeTaskPool::get_or_init(|| {
        TaskPoolBuilder::new()
            .on_thread_spawn(|| {
                init_thread_stream();
            })
            .on_thread_destroy(|| {
                micromegas::tracing::dispatch::flush_thread_buffer();
                micromegas::tracing::dispatch::unregister_thread_stream();
            })
            .build()
    });

    // 3. Run Bevy app
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(optimism::OptimismPlugin)
        .run();
}
