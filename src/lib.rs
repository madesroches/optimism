use bevy::prelude::*;
use micromegas::tracing::prelude::*;
use micromegas::tracing::prelude::info;

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
