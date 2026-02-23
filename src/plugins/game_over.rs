//! Game Over screen: display stats and return to main menu.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::resources::{CurrentLevel, GameStats, Score};

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::GameOver), spawn_game_over);
        app.add_systems(OnExit(AppState::GameOver), despawn_game_over);
        app.add_systems(
            Update,
            game_over_input.run_if(in_state(AppState::GameOver)),
        );
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct GameOverRoot;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn spawn_game_over(
    mut commands: Commands,
    score: Res<Score>,
    level: Res<CurrentLevel>,
    stats: Res<GameStats>,
) {
    let mut lines = vec![
        format!("Final Score: {}", score.0),
        format!("Level Reached: {}", level.0),
        format!("Deaths: {}", stats.deaths),
    ];

    // Kills by weapon
    if !stats.kills_by_weapon.is_empty() {
        let mut kill_parts: Vec<String> = stats
            .kills_by_weapon
            .iter()
            .map(|(w, c)| format!("{:?}: {}", w, c))
            .collect();
        kill_parts.sort();
        lines.push(format!("Kills: {}", kill_parts.join(", ")));
    }

    // Luxuries collected
    if !stats.luxuries_collected.is_empty() {
        let luxury_names: Vec<String> = stats
            .luxuries_collected
            .iter()
            .map(|l| format!("{:?}", l))
            .collect();
        lines.push(format!("Luxuries: {}", luxury_names.join(", ")));
    }

    commands
        .spawn((
            GameOverRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.02, 0.02)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("The best of possible games..."),
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("...considering."),
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
            ));

            for line in &lines {
                parent.spawn((
                    Text::new(line.clone()),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                ));
            }

            parent.spawn((
                Text::new("Press Enter to Return to Menu"),
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
            ));
        });
}

fn despawn_game_over(mut commands: Commands, query: Query<Entity, With<GameOverRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::MainMenu);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<AppState>();
        app.add_sub_state::<crate::app_state::PlayingState>();
        app.insert_resource(Score(100));
        app.insert_resource(CurrentLevel(3));
        app.init_resource::<GameStats>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_plugins(GameOverPlugin);
        app
    }

    fn transition_to_game_over(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::GameOver);
        for _ in 0..5 {
            app.update();
        }
    }

    #[test]
    fn game_over_spawns() {
        let mut app = setup_app();
        transition_to_game_over(&mut app);

        let count = app
            .world_mut()
            .query::<&GameOverRoot>()
            .iter(app.world())
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn game_over_despawns_on_exit() {
        let mut app = setup_app();
        transition_to_game_over(&mut app);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::MainMenu);
        for _ in 0..5 {
            app.update();
        }

        let count = app
            .world_mut()
            .query::<&GameOverRoot>()
            .iter(app.world())
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn enter_returns_to_menu() {
        let mut app = setup_app();
        transition_to_game_over(&mut app);

        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Enter);
        app.insert_resource(input);
        app.update();

        for _ in 0..5 {
            app.update();
        }

        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::MainMenu);
    }
}
