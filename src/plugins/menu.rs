//! Main menu UI: title screen with start prompt.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::resources::{CurrentLevel, GameStats, Lives, Score};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_menu);
        app.add_systems(OnExit(AppState::MainMenu), despawn_menu);
        app.add_systems(
            Update,
            menu_input.run_if(in_state(AppState::MainMenu)),
        );
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct MenuRoot;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.02, 0.02, 0.06)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("OPTIMISM"),
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("A Pac-Man game inspired by Voltaire's Candide"),
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("Press Enter to Start"),
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
        });
}

fn despawn_menu(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut score: ResMut<Score>,
    mut lives: ResMut<Lives>,
    mut level: ResMut<CurrentLevel>,
    mut stats: ResMut<GameStats>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        // Reset game state for a new game
        score.0 = 0;
        lives.0 = 3;
        level.0 = 1;
        *stats = GameStats::default();
        next_state.set(AppState::InGame);
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
        app.insert_resource(Score(0));
        app.insert_resource(Lives(3));
        app.insert_resource(CurrentLevel(1));
        app.init_resource::<GameStats>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_plugins(MenuPlugin);
        app
    }

    fn transition_to_main_menu(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::MainMenu);
        for _ in 0..5 {
            app.update();
        }
    }

    #[test]
    fn menu_spawns_on_main_menu() {
        let mut app = setup_app();
        transition_to_main_menu(&mut app);

        let count = app
            .world_mut()
            .query::<&MenuRoot>()
            .iter(app.world())
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn menu_despawns_on_exit() {
        let mut app = setup_app();
        transition_to_main_menu(&mut app);

        // Transition away
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        for _ in 0..5 {
            app.update();
        }

        let count = app
            .world_mut()
            .query::<&MenuRoot>()
            .iter(app.world())
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn enter_resets_and_starts_game() {
        let mut app = setup_app();
        transition_to_main_menu(&mut app);

        // Set dirty state
        app.world_mut().resource_mut::<Score>().0 = 999;
        app.world_mut().resource_mut::<Lives>().0 = 0;
        app.world_mut().resource_mut::<CurrentLevel>().0 = 5;

        // Simulate Enter press
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Enter);
        app.insert_resource(input);
        app.update();

        // State should have been reset
        assert_eq!(app.world().resource::<Score>().0, 0);
        assert_eq!(app.world().resource::<Lives>().0, 3);
        assert_eq!(app.world().resource::<CurrentLevel>().0, 1);

        // Should transition to InGame
        for _ in 0..5 {
            app.update();
        }
        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::InGame);
    }
}
