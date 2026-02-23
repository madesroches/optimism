//! Main menu UI: title screen with start prompt.

use bevy::prelude::*;

use crate::app_state::AppState;

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
) {
    if keyboard.just_pressed(KeyCode::Enter) {
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
    fn enter_starts_game() {
        let mut app = setup_app();
        transition_to_main_menu(&mut app);

        // Simulate Enter press
        let mut input = ButtonInput::<KeyCode>::default();
        input.press(KeyCode::Enter);
        app.insert_resource(input);
        app.update();

        // Should transition to InGame
        for _ in 0..5 {
            app.update();
        }
        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::InGame);
    }
}
