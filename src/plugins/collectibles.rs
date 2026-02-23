//! Collectible systems: money dots, luxury items, and level completion.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;

use crate::app_state::PlayingState;
use crate::components::{GridPosition, LuxuryItem, LuxuryTimeout, Money, Player};
use crate::events::{LuxuryCollected, MoneyCollected};
use crate::plugins::maze::{grid_to_world, load_maze, MazeEntity, MazeMap, TILE_SIZE};
use crate::resources::{GameStats, LevelConfig, Score};

pub struct CollectiblePlugin;

impl Plugin for CollectiblePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            spawn_luxury_items.after(load_maze),
        );
        app.add_systems(
            Update,
            (
                money_collection,
                luxury_collection,
                luxury_timeout,
                check_level_complete.after(money_collection),
            )
                .run_if(in_state(PlayingState::Playing)),
        );
    }
}

/// Despawn money dots when player walks over them.
#[span_fn]
fn money_collection(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut stats: ResMut<GameStats>,
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
            stats.money_collected += 10;
            micromegas_tracing::prelude::info!("money_collected: score={}", score.0);
            imetric!("score", "points", score.0);
            commands.trigger(MoneyCollected);
        }
    }
}

/// Spawn luxury items at L positions from the maze.
#[span_fn]
fn spawn_luxury_items(
    mut commands: Commands,
    maze: Res<MazeMap>,
    config: Res<LevelConfig>,
) {
    if config.is_garden {
        return;
    }

    let luxury_color = Color::srgb(0.9, 0.6, 1.0);

    for spawn_pos in &maze.luxury_spawns {
        let world_pos = grid_to_world(*spawn_pos, maze.width, maze.height);
        commands.spawn((
            LuxuryItem(config.luxury_type),
            LuxuryTimeout(Timer::from_seconds(15.0, TimerMode::Once)),
            *spawn_pos,
            MazeEntity,
            Sprite::from_color(luxury_color, Vec2::splat(TILE_SIZE * 0.7)),
            Transform::from_xyz(world_pos.x, world_pos.y, 2.0),
        ));
    }
}

/// Player collects luxury items on contact.
#[span_fn]
fn luxury_collection(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut stats: ResMut<GameStats>,
    player_query: Query<&GridPosition, With<Player>>,
    luxury_query: Query<(Entity, &GridPosition, &LuxuryItem)>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };
    for (entity, luxury_pos, luxury_item) in &luxury_query {
        if player_pos == luxury_pos {
            commands.entity(entity).despawn();
            score.0 += 500;
            stats.luxuries_collected.push(luxury_item.0);
            commands.trigger(LuxuryCollected);
        }
    }
}

/// Tick luxury timeouts and despawn expired items.
#[span_fn]
fn luxury_timeout(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut LuxuryTimeout)>,
) {
    for (entity, mut timeout) in &mut query {
        timeout.tick(time.delta());
        if timeout.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// When all money is collected, transition to LevelComplete.
/// Only checks after at least one dot has been collected (score > 0)
/// to avoid false triggers before the maze is loaded.
#[span_fn]
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
        app.init_resource::<GameStats>();
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
    fn money_collection_tracks_stats() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };
        app.world_mut().spawn((Player, pos));
        app.world_mut().spawn((Money, pos));

        app.update();

        assert_eq!(app.world().resource::<GameStats>().money_collected, 10);
    }

    #[test]
    fn all_money_collected_triggers_level_complete() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::app_state::AppState>();
        app.add_sub_state::<PlayingState>();
        app.insert_resource(Score(0));
        app.init_resource::<GameStats>();
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

    #[test]
    fn luxury_timeout_despawns() {
        use crate::components::LuxuryType;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, luxury_timeout);

        // Warm up so Time has a non-zero delta on subsequent updates
        app.update();

        // Create a timer pre-ticked to just under its duration. Any positive
        // frame delta from the Time resource will push it past the threshold
        // and trigger just_finished().
        let mut timer = Timer::from_seconds(0.001, TimerMode::Once);
        timer.tick(Duration::from_nanos(999_000));
        let luxury = app
            .world_mut()
            .spawn((
                LuxuryItem(LuxuryType::GoldGrill),
                LuxuryTimeout(timer),
                GridPosition { x: 1, y: 1 },
            ))
            .id();

        // Run several updates to guarantee at least one has a non-zero delta
        for _ in 0..10 {
            app.update();
            if app.world().get_entity(luxury).is_err() {
                return; // Despawned as expected
            }
        }

        panic!("Luxury item should be despawned after timeout");
    }
}
