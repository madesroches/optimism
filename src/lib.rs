pub mod ai;
pub mod app_state;
pub mod components;
pub mod events;
pub mod plugins;
pub mod resources;
pub mod tracing_bridge;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioPlugin;
use micromegas_tracing::prelude::*;

use app_state::{AppState, PlayingState};
use plugins::audio::GameAudioPlugin;
use plugins::camera::CameraPlugin;
use plugins::collectibles::CollectiblePlugin;
use plugins::combat::CombatPlugin;
use plugins::enemies::EnemyPlugin;
use plugins::game_over::GameOverPlugin;
use plugins::hud::HudPlugin;
use plugins::maze::MazePlugin;
use plugins::menu::MenuPlugin;
use plugins::movement::MovementPlugin;
use plugins::narration::NarrationPlugin;
use plugins::narration::NarrationState;
use plugins::player::PlayerPlugin;
use plugins::sprites::SpriteSheetPlugin;
use plugins::telemetry::TelemetryPlugin;
use resources::{AudioAssets, CurrentLevel, GameStats, Lives, Score};

pub struct OptimismPlugin;

impl Plugin for OptimismPlugin {
    fn build(&self, app: &mut App) {
        // State machine (StatesPlugin comes from DefaultPlugins)
        app.init_state::<AppState>();
        app.add_sub_state::<PlayingState>();

        // Audio
        app.add_plugins(AudioPlugin);

        // Game plugins
        app.add_plugins(SpriteSheetPlugin);
        app.add_plugins(CameraPlugin);
        app.add_plugins(MazePlugin);
        app.add_plugins(MovementPlugin);
        app.add_plugins(PlayerPlugin);
        app.add_plugins(CollectiblePlugin);
        app.add_plugins(EnemyPlugin);
        app.add_plugins(CombatPlugin);
        app.add_plugins(GameAudioPlugin);
        app.add_plugins(HudPlugin);
        app.add_plugins(NarrationPlugin);
        app.add_plugins(MenuPlugin);
        app.add_plugins(GameOverPlugin);
        app.add_plugins(TelemetryPlugin);

        // Per-game-session resources: inserted fresh on each game start,
        // persist through GameOver for stats display, cleaned up on exit.
        app.add_systems(OnEnter(AppState::InGame), init_game_session);
        app.add_systems(OnExit(AppState::GameOver), cleanup_game_session);

        // Asset loading
        app.add_loading_state(
            LoadingState::new(AppState::Loading)
                .continue_to_state(AppState::MainMenu)
                .load_collection::<AudioAssets>(),
        );
    }
}

/// Insert per-game-session resources with fresh defaults.
/// Runs on each `OnEnter(AppState::InGame)`, so a new game always starts clean.
/// These resources persist through `GameOver` for stats display and are
/// cleaned up on `OnExit(GameOver)`.
#[span_fn]
fn init_game_session(mut commands: Commands) {
    commands.insert_resource(Score(0));
    commands.insert_resource(CurrentLevel(1));
    commands.insert_resource(Lives(3));
    commands.insert_resource(GameStats::default());
    commands.insert_resource(NarrationState::default());
}

/// Remove per-game-session resources when leaving GameOver.
#[span_fn]
fn cleanup_game_session(mut commands: Commands) {
    commands.remove_resource::<Score>();
    commands.remove_resource::<CurrentLevel>();
    commands.remove_resource::<Lives>();
    commands.remove_resource::<GameStats>();
    commands.remove_resource::<NarrationState>();
}
