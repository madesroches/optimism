pub mod ai;
pub mod app_state;
pub mod components;
pub mod events;
pub mod plugins;
pub mod resources;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioPlugin;

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

        // Resources
        app.insert_resource(Score(0));
        app.insert_resource(CurrentLevel(1));
        app.insert_resource(Lives(3));
        app.init_resource::<GameStats>();

        // Asset loading
        app.add_loading_state(
            LoadingState::new(AppState::Loading)
                .continue_to_state(AppState::MainMenu)
                .load_collection::<AudioAssets>(),
        );
    }
}
