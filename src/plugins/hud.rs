//! HUD overlay: score, lives, and level display.

use bevy::prelude::*;
use micromegas::tracing::prelude::*;

use crate::app_state::AppState;
use crate::plugins::telemetry::GameSet;
use crate::resources::{CurrentLevel, Lives, Score};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), spawn_hud);
        app.add_systems(OnExit(AppState::InGame), despawn_hud);
        app.add_systems(
            Update,
            update_hud
                .in_set(GameSet::Presentation)
                .run_if(in_state(AppState::InGame)),
        );
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct LivesText;

#[derive(Component)]
pub struct LevelText;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

#[span_fn]
fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(12.0)),
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                ScoreText,
                Text::new("Score: 0"),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
            parent.spawn((
                LivesText,
                Text::new("Lives: 3"),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
            parent.spawn((
                LevelText,
                Text::new("Level: 1"),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
        });
}

#[span_fn]
fn despawn_hud(mut commands: Commands, query: Query<Entity, With<HudRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::type_complexity)]
#[span_fn]
fn update_hud(
    score: Res<Score>,
    lives: Res<Lives>,
    level: Res<CurrentLevel>,
    mut score_text: Query<&mut Text, (With<ScoreText>, Without<LivesText>, Without<LevelText>)>,
    mut lives_text: Query<&mut Text, (With<LivesText>, Without<ScoreText>, Without<LevelText>)>,
    mut level_text: Query<&mut Text, (With<LevelText>, Without<ScoreText>, Without<LivesText>)>,
) {
    if let Ok(mut text) = score_text.single_mut() {
        **text = format!("Score: {}", score.0);
    }
    if let Ok(mut text) = lives_text.single_mut() {
        **text = format!("Lives: {}", lives.0);
    }
    if let Ok(mut text) = level_text.single_mut() {
        **text = format!("Level: {}", level.0);
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
        app.add_plugins(HudPlugin);
        app
    }

    fn transition_to_in_game(app: &mut App) {
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        for _ in 0..5 {
            app.update();
        }
    }

    #[test]
    fn hud_spawns_on_in_game() {
        let mut app = setup_app();
        transition_to_in_game(&mut app);

        let hud_count = app
            .world_mut()
            .query::<&HudRoot>()
            .iter(app.world())
            .count();
        assert_eq!(hud_count, 1);
    }

    #[test]
    fn hud_text_updates() {
        let mut app = setup_app();
        transition_to_in_game(&mut app);

        app.world_mut().resource_mut::<Score>().0 = 42;
        app.world_mut().resource_mut::<Lives>().0 = 1;
        app.world_mut().resource_mut::<CurrentLevel>().0 = 5;
        app.update();

        let score_text = app
            .world_mut()
            .query_filtered::<&Text, With<ScoreText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(**score_text, "Score: 42");

        let lives_text = app
            .world_mut()
            .query_filtered::<&Text, With<LivesText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(**lives_text, "Lives: 1");

        let level_text = app
            .world_mut()
            .query_filtered::<&Text, With<LevelText>>()
            .single(app.world())
            .unwrap();
        assert_eq!(**level_text, "Level: 5");
    }

    #[test]
    fn hud_despawns_on_exit() {
        let mut app = setup_app();
        transition_to_in_game(&mut app);

        // Go back to MainMenu
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::MainMenu);
        for _ in 0..5 {
            app.update();
        }

        let hud_count = app
            .world_mut()
            .query::<&HudRoot>()
            .iter(app.world())
            .count();
        assert_eq!(hud_count, 0);
    }
}
