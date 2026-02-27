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

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;
use micromegas::tracing::dispatch::{on_begin_async_scope, on_end_async_scope};
use micromegas::tracing::intern_string::intern_string;
use micromegas::tracing::prelude::*;
use micromegas::tracing::property_set::{Property, PropertySet};

use crate::app_state::{AppState, PlayingState};
use crate::plugins::maze::load_maze;
use crate::resources::LevelConfig;

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
// Game context — carries a PropertySet for per-map metric/log tagging
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct GameContext {
    pub properties: &'static PropertySet,
}

impl GameContext {
    /// Create a GameContext for the given map name (e.g. "level_01").
    pub fn new(map_name: &str) -> Self {
        let name = intern_string(map_name);
        let props = PropertySet::find_or_create(vec![Property::new("map", name)]);
        Self { properties: props }
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn build(&self, app: &mut App) {
        // Game context (per-map property set for metric tagging)
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            update_game_context.after(load_maze),
        );
        app.add_systems(OnExit(AppState::InGame), cleanup_game_context);

        // Frame span (sync, Main schedule)
        app.add_systems(Main, begin_frame_span.before(Main::run_main));
        app.add_systems(Main, end_frame_span.after(Main::run_main));
        app.add_systems(Last, frame_telemetry);

        // Subsystem spans (async, Update schedule)
        let playing = in_state(PlayingState::Playing);
        app.add_systems(
            Update,
            begin_player_span
                .before(GameSet::Player)
                .run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            end_player_span
                .after(GameSet::Player)
                .run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            begin_ai_span.before(GameSet::AI).run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            end_ai_span.after(GameSet::AI).run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            begin_movement_span
                .before(GameSet::Movement)
                .run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            end_movement_span
                .after(GameSet::Movement)
                .run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            begin_combat_span
                .before(GameSet::Combat)
                .run_if(playing.clone()),
        );
        app.add_systems(
            Update,
            end_combat_span.after(GameSet::Combat).run_if(playing),
        );
        app.add_systems(
            Update,
            begin_collectibles_span
                .before(GameSet::Collectibles)
                .run_if(in_state(PlayingState::Playing)),
        );
        app.add_systems(
            Update,
            end_collectibles_span
                .after(GameSet::Collectibles)
                .run_if(in_state(PlayingState::Playing)),
        );
        app.add_systems(
            Update,
            begin_presentation_span.before(GameSet::Presentation),
        );
        app.add_systems(Update, end_presentation_span.after(GameSet::Presentation));
    }
}

// ---------------------------------------------------------------------------
// Frame span (sync)
// ---------------------------------------------------------------------------

static_span_desc!(FRAME_SPAN, "Frame");

fn begin_frame_span() {
    micromegas::tracing::dispatch::on_begin_scope(&FRAME_SPAN);
}

fn end_frame_span() {
    micromegas::tracing::dispatch::on_end_scope(&FRAME_SPAN);
}

#[span_fn]
fn frame_telemetry(time: Res<Time>, game_ctx: Option<Res<GameContext>>) {
    let dt_ms = time.delta_secs_f64() * 1000.0;
    if let Some(ref ctx) = game_ctx {
        fmetric!("frame_time_ms", "ms", ctx.properties, dt_ms);
    } else {
        fmetric!("frame_time_ms", "ms", dt_ms);
    }
}

// ---------------------------------------------------------------------------
// Game context systems
// ---------------------------------------------------------------------------

fn map_name_from_path(path: &str) -> &str {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
}

#[span_fn]
fn update_game_context(mut commands: Commands, config: Res<LevelConfig>) {
    commands.insert_resource(GameContext::new(map_name_from_path(&config.maze_file)));
}

#[span_fn]
fn cleanup_game_context(mut commands: Commands) {
    commands.remove_resource::<GameContext>();
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

subsystem_span!(
    "Player",
    PLAYER_SPAN,
    PLAYER_SPAN_ID,
    begin_player_span,
    end_player_span
);
subsystem_span!("AI", AI_SPAN, AI_SPAN_ID, begin_ai_span, end_ai_span);
subsystem_span!(
    "Movement",
    MOVEMENT_SPAN,
    MOVEMENT_SPAN_ID,
    begin_movement_span,
    end_movement_span
);
subsystem_span!(
    "Combat",
    COMBAT_SPAN,
    COMBAT_SPAN_ID,
    begin_combat_span,
    end_combat_span
);
subsystem_span!(
    "Collectibles",
    COLLECTIBLES_SPAN,
    COLLECTIBLES_SPAN_ID,
    begin_collectibles_span,
    end_collectibles_span
);
subsystem_span!(
    "Presentation",
    PRESENTATION_SPAN,
    PRESENTATION_SPAN_ID,
    begin_presentation_span,
    end_presentation_span
);
