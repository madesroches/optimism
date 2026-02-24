use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::tasks::{ComputeTaskPool, TaskPoolBuilder};
use micromegas_telemetry_sink::TelemetryGuardBuilder;
use micromegas_telemetry_sink::tracing_interop::TracingCaptureLayer;
use micromegas_tracing::dispatch::init_thread_stream;
use micromegas_tracing::levels::LevelFilter;
use micromegas_tracing::prelude::info;
use optimism::tracing_bridge::MicromegasBridgeLayer;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;

fn main() {
    // 1. Initialize telemetry (creates LocalEventSink for stdout)
    //    Note: spans require MICROMEGAS_ENABLE_CPU_TRACING=true (env var).
    //    Without it, init_thread_stream() is a no-op and spans are silently dropped.
    //    Logs and metrics always work regardless.
    let _telemetry_guard = TelemetryGuardBuilder::default()
        .with_install_tracing_capture(false)
        .build()
        .expect("failed to initialize telemetry");

    info!("Optimism PoC starting");

    // 2. Install tracing subscriber that bridges Bevy's schedule spans into
    //    Micromegas.  Must be set before Bevy starts (bevy/trace emits spans
    //    via the `tracing` crate's global subscriber).
    let log_layer = TracingCaptureLayer {
        max_level: LevelFilter::Info,
    };
    let subscriber = Registry::default()
        .with(MicromegasBridgeLayer)
        .with(log_layer);
    tracing::subscriber::set_global_default(subscriber).expect("failed to set tracing subscriber");

    // 3. Pre-init ComputeTaskPool with Micromegas thread callbacks.
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

    // 4. Run Bevy app
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins(optimism::OptimismPlugin)
        .run();
}
