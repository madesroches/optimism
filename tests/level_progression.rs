//! Integration tests for level progression, game over, and menu transitions.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use optimism::app_state::{AppState, PlayingState};
use optimism::resources::{CurrentLevel, GameStats, Lives, Score, level_config};

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.add_sub_state::<PlayingState>();
    app.insert_resource(Score(0));
    app.insert_resource(CurrentLevel(1));
    app.insert_resource(Lives(3));
    app.init_resource::<GameStats>();
    app
}

fn transition_to(app: &mut App, state: AppState) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(state);
    for _ in 0..5 {
        app.update();
    }
}

#[test]
fn game_over_to_main_menu_transition() {
    let mut app = setup_app();

    // Go to GameOver
    transition_to(&mut app, AppState::GameOver);
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(*state.get(), AppState::GameOver);

    // Go back to MainMenu
    transition_to(&mut app, AppState::MainMenu);
    let state = app.world().resource::<State<AppState>>();
    assert_eq!(*state.get(), AppState::MainMenu);
}

#[test]
fn garden_level_config_has_no_enemies() {
    let cfg = level_config(13);
    assert_eq!(cfg.enemy_speed_multiplier, 0.0);
    assert!(cfg.maze_file.contains("garden"));
}

#[test]
fn stats_accumulate() {
    let mut stats = GameStats::default();
    stats.deaths += 1;
    stats.money_collected += 100;
    stats.deaths += 1;
    stats.money_collected += 50;
    assert_eq!(stats.deaths, 2);
    assert_eq!(stats.money_collected, 150);
}

#[test]
fn level_configs_form_valid_progression() {
    for level in 1..=13 {
        let cfg = level_config(level);

        // All maze files should exist
        assert!(
            std::fs::metadata(&cfg.maze_file).is_ok(),
            "Maze file {} does not exist for level {}",
            cfg.maze_file,
            level
        );

        // Speed multiplier should be non-negative
        assert!(cfg.enemy_speed_multiplier >= 0.0);

        // Duration should be at least 3.0 (our minimum)
        assert!(cfg.weapon_duration_secs >= 3.0);

        // Pen release should be at least 1.0
        assert!(cfg.pen_release_interval_secs >= 1.0);
    }
}
