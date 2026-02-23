//! Frame-level telemetry: wraps the game loop with Micromegas instrumentation.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, frame_telemetry);
    }
}

#[span_fn]
fn frame_telemetry(time: Res<Time>) {
    let dt_ms = time.delta_secs_f64() * 1000.0;
    fmetric!("frame_time_ms", "ms", dt_ms);
}
