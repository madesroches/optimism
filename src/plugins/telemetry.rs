//! Frame-level telemetry: wraps the game loop with Micromegas instrumentation.
//!
//! A "Frame" span covers the entire Bevy frame by hooking into the `Main`
//! schedule before and after `Main::run_main`.  The `Main` schedule uses a
//! single-threaded executor, so begin/end are guaranteed to run on the same
//! thread.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Main,
            begin_frame_span.before(Main::run_main),
        );
        app.add_systems(
            Main,
            end_frame_span.after(Main::run_main),
        );
        app.add_systems(Last, frame_telemetry);
    }
}

static_span_desc!(FRAME_SPAN, "Frame");

fn begin_frame_span() {
    micromegas_tracing::dispatch::on_begin_scope(&FRAME_SPAN);
}

fn end_frame_span() {
    micromegas_tracing::dispatch::on_end_scope(&FRAME_SPAN);
}

#[span_fn]
fn frame_telemetry(time: Res<Time>) {
    let dt_ms = time.delta_secs_f64() * 1000.0;
    fmetric!("frame_time_ms", "ms", dt_ms);
}
