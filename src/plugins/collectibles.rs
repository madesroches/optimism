//! Collectible systems: money dots and level completion.

use bevy::prelude::*;

use crate::app_state::PlayingState;
use crate::components::{GridPosition, Money, Player};
use crate::resources::Score;

pub struct CollectiblePlugin;

impl Plugin for CollectiblePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                money_collection,
                check_level_complete.after(money_collection),
            )
                .run_if(in_state(PlayingState::Playing)),
        );
    }
}

/// Despawn money dots when player walks over them.
fn money_collection(
    mut commands: Commands,
    mut score: ResMut<Score>,
    player_query: Query<&GridPosition, With<Player>>,
    money_query: Query<(Entity, &GridPosition), With<Money>>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };
    for (entity, money_pos) in &money_query {
        if player_pos == money_pos {
            commands.entity(entity).despawn();
            score.0 += 10;
        }
    }
}

/// When all money is collected, transition to LevelComplete.
/// Only checks after at least one dot has been collected (score > 0)
/// to avoid false triggers before the maze is loaded.
fn check_level_complete(
    money_query: Query<(), With<Money>>,
    score: Res<Score>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if score.0 > 0 && money_query.iter().count() == 0 {
        next_state.set(PlayingState::LevelComplete);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::app_state::AppState>();
        app.add_sub_state::<PlayingState>();
        app.insert_resource(Score(0));
        // Only register money_collection for simple tests — check_level_complete
        // is tested separately to avoid it firing before entities are spawned.
        app.add_systems(
            Update,
            money_collection.run_if(in_state(PlayingState::Playing)),
        );

        // Transition to Playing
        app.world_mut()
            .resource_mut::<NextState<crate::app_state::AppState>>()
            .set(crate::app_state::AppState::InGame);
        for _ in 0..5 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<NextState<PlayingState>>()
            .set(PlayingState::Playing);
        for _ in 0..5 {
            app.update();
        }
        app
    }

    #[test]
    fn money_collection_increments_score() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };
        app.world_mut().spawn((Player, pos));
        let money = app.world_mut().spawn((Money, pos)).id();

        app.update();

        assert_eq!(app.world().resource::<Score>().0, 10);
        assert!(app.world().get_entity(money).is_err());
    }

    #[test]
    fn all_money_collected_triggers_level_complete() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::app_state::AppState>();
        app.add_sub_state::<PlayingState>();
        app.insert_resource(Score(0));
        app.add_systems(
            Update,
            (
                money_collection,
                check_level_complete.after(money_collection),
            )
                .run_if(in_state(PlayingState::Playing)),
        );

        // Spawn entities BEFORE transitioning to Playing
        let pos = GridPosition { x: 1, y: 1 };
        app.world_mut().spawn((Player, pos));
        app.world_mut().spawn((Money, pos));

        // Now transition to Playing
        app.world_mut()
            .resource_mut::<NextState<crate::app_state::AppState>>()
            .set(crate::app_state::AppState::InGame);
        for _ in 0..5 {
            app.update();
        }
        app.world_mut()
            .resource_mut::<NextState<PlayingState>>()
            .set(PlayingState::Playing);
        for _ in 0..5 {
            app.update();
        }

        // Money should be collected and level complete triggered
        for _ in 0..5 {
            app.update();
        }

        let state = app.world().resource::<State<PlayingState>>();
        assert_eq!(*state.get(), PlayingState::LevelComplete);
    }
}
