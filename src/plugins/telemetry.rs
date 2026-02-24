//! Telemetry: frame-level and subsystem-level span instrumentation.
//!
//! A "Frame" span covers the entire Bevy frame by hooking into the `Main`
//! schedule before and after `Main::run_main`.  The `Main` schedule uses a
//! single-threaded executor, so begin/end are guaranteed to run on the same
//! thread.
//!
//! Subsystem spans use async span events (explicit ID tracking) to group
//! related systems under named categories without constraining Bevy's
//! multi-threaded parallelism.  The Micromegas analysis tool correlates
//! async subsystem spans with sync per-system spans by time overlap.

use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;
use micromegas_tracing::dispatch::{on_begin_async_scope, on_end_async_scope};
use micromegas_tracing::prelude::*;

use crate::app_state::PlayingState;

// ---------------------------------------------------------------------------
// Subsystem sets — used by individual plugins via `.in_set(GameSet::X)`
// ---------------------------------------------------------------------------

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSet {
    Player,
    AI,
    Movement,
    Combat,
    Collectibles,
    Presentation,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        // Frame span (sync, Main schedule)
        app.add_systems(
            Main,
            begin_frame_span.before(Main::run_main),
        );
        app.add_systems(
            Main,
            end_frame_span.after(Main::run_main),
        );
        app.add_systems(Last, frame_telemetry);

        // Subsystem spans (async, Update schedule)
        let playing = in_state(PlayingState::Playing);
        app.add_systems(Update, begin_player_span.before(GameSet::Player).run_if(playing.clone()));
        app.add_systems(Update, end_player_span.after(GameSet::Player).run_if(playing.clone()));
        app.add_systems(Update, begin_ai_span.before(GameSet::AI).run_if(playing.clone()));
        app.add_systems(Update, end_ai_span.after(GameSet::AI).run_if(playing.clone()));
        app.add_systems(Update, begin_movement_span.before(GameSet::Movement).run_if(playing.clone()));
        app.add_systems(Update, end_movement_span.after(GameSet::Movement).run_if(playing.clone()));
        app.add_systems(Update, begin_combat_span.before(GameSet::Combat).run_if(playing.clone()));
        app.add_systems(Update, end_combat_span.after(GameSet::Combat).run_if(playing));
        app.add_systems(Update, begin_collectibles_span.before(GameSet::Collectibles).run_if(in_state(PlayingState::Playing)));
        app.add_systems(Update, end_collectibles_span.after(GameSet::Collectibles).run_if(in_state(PlayingState::Playing)));
        app.add_systems(Update, begin_presentation_span.before(GameSet::Presentation));
        app.add_systems(Update, end_presentation_span.after(GameSet::Presentation));
    }
}

// ---------------------------------------------------------------------------
// Frame span (sync)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Subsystem spans (async — explicit ID tracking, cross-thread safe)
// ---------------------------------------------------------------------------

macro_rules! subsystem_span {
    ($label:expr, $span:ident, $id:ident, $begin:ident, $end:ident) => {
        static_span_desc!($span, $label);
        static $id: AtomicU64 = AtomicU64::new(0);

        fn $begin() {
            let id = on_begin_async_scope(&$span, 0, 0);
            $id.store(id, Ordering::Release);
        }

        fn $end() {
            let id = $id.load(Ordering::Acquire);
            on_end_async_scope(id, 0, &$span, 0);
        }
    };
}

subsystem_span!("Player",       PLAYER_SPAN,       PLAYER_SPAN_ID,       begin_player_span,       end_player_span);
subsystem_span!("AI",           AI_SPAN,           AI_SPAN_ID,           begin_ai_span,           end_ai_span);
subsystem_span!("Movement",     MOVEMENT_SPAN,     MOVEMENT_SPAN_ID,     begin_movement_span,     end_movement_span);
subsystem_span!("Combat",       COMBAT_SPAN,       COMBAT_SPAN_ID,       begin_combat_span,       end_combat_span);
subsystem_span!("Collectibles", COLLECTIBLES_SPAN, COLLECTIBLES_SPAN_ID, begin_collectibles_span, end_collectibles_span);
subsystem_span!("Presentation", PRESENTATION_SPAN, PRESENTATION_SPAN_ID, begin_presentation_span, end_presentation_span);
