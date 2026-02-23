//! Grid-based movement with smooth visual interpolation.
//!
//! Entities move tile-to-tile using `MoveDirection`. Movement is validated
//! against the `MazeMap` walkability grid. Visual position is interpolated
//! via `MoveLerp` for smooth animation between tiles.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;

use crate::app_state::PlayingState;
use crate::components::*;
use crate::plugins::maze::{grid_to_world, MazeMap};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                movement_validation,
                movement_interpolation.after(movement_validation),
                sync_transform_to_grid.after(movement_interpolation),
            )
                .run_if(in_state(PlayingState::Playing)),
        );
    }
}

/// Check if a move is valid and start a lerp if so.
#[allow(clippy::type_complexity)]
#[span_fn]
fn movement_validation(
    maze: Res<MazeMap>,
    mut query: Query<(
        Entity,
        &mut GridPosition,
        &MoveDirection,
        &MoveSpeed,
        Option<&MoveLerp>,
        Has<Player>,
    )>,
    mut commands: Commands,
) {
    for (entity, mut pos, dir, _speed, lerp, is_player) in &mut query {
        // Don't start a new move while one is in progress
        if lerp.is_some() {
            continue;
        }

        let (dx, dy) = dir.0.delta();
        let target = GridPosition {
            x: pos.x + dx,
            y: pos.y + dy,
        };

        // Player cannot walk through pen gates; enemies can. InPen enemies don't
        // reach here because enemy_ai excludes them from receiving MoveDirection.
        let walkable = if is_player {
            maze.is_walkable_for_player(target)
        } else {
            maze.is_walkable_for_enemy(target)
        };

        if !walkable {
            // Can't move there — remove direction so we stop
            commands.entity(entity).remove::<MoveDirection>();
            continue;
        }

        let from = grid_to_world(*pos, maze.width, maze.height);
        let to = grid_to_world(target, maze.width, maze.height);

        // Record previous position for cross-through collision detection
        commands.entity(entity).insert(PreviousGridPosition(*pos));

        // Update grid position immediately (gameplay logic uses GridPosition)
        *pos = target;

        // Start visual interpolation
        commands.entity(entity).insert(MoveLerp { from, to, t: 0.0 });
    }
}

/// Advance movement interpolation and snap when complete.
#[span_fn]
fn movement_interpolation(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut MoveLerp, &MoveSpeed)>,
    mut commands: Commands,
) {
    for (entity, mut transform, mut lerp, speed) in &mut query {
        // t advances based on speed (tiles per second) and tile size
        let dt = time.delta_secs() * speed.0;
        let new_t = (lerp.t + dt).min(1.0);
        lerp.t = new_t;

        let pos = lerp.from.lerp(lerp.to, new_t);
        transform.translation.x = pos.x;
        transform.translation.y = pos.y;

        if new_t >= 1.0 {
            // Snap to target
            transform.translation.x = lerp.to.x;
            transform.translation.y = lerp.to.y;
            commands.entity(entity).remove::<MoveLerp>();
            commands.entity(entity).remove::<MoveDirection>();
            commands.entity(entity).remove::<PreviousGridPosition>();
        }
    }
}

/// For entities with a GridPosition but no MoveLerp, keep Transform synced.
/// This handles initial placement and teleports (e.g., death respawn).
#[allow(clippy::type_complexity)]
#[span_fn]
fn sync_transform_to_grid(
    maze: Option<Res<MazeMap>>,
    mut query: Query<(&GridPosition, &mut Transform), (Changed<GridPosition>, Without<MoveLerp>)>,
) {
    let Some(maze) = maze else { return };
    for (pos, mut transform) in &mut query {
        let world = grid_to_world(*pos, maze.width, maze.height);
        transform.translation.x = world.x;
        transform.translation.y = world.y;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::maze::MazeMap;
    use bevy::state::app::StatesPlugin;

    fn test_maze() -> MazeMap {
        MazeMap::parse("####\n#P.#\n#..#\n####").unwrap()
    }

    fn setup_app(maze: MazeMap) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::app_state::AppState>();
        app.add_sub_state::<PlayingState>();
        app.insert_resource(maze);
        app.add_plugins(MovementPlugin);

        // Transition to Playing state
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
    fn player_moves_to_empty_tile() {
        let maze = test_maze();
        let mut app = setup_app(maze);

        // Spawn player at (1,1)
        let player = app
            .world_mut()
            .spawn((
                Player,
                GridPosition { x: 1, y: 1 },
                MoveDirection(Direction::Right),
                MoveSpeed(10.0),
                Transform::default(),
            ))
            .id();

        app.update();

        // GridPosition should have moved to (2,1)
        let pos = app.world().entity(player).get::<GridPosition>().unwrap();
        assert_eq!(*pos, GridPosition { x: 2, y: 1 });
    }

    #[test]
    fn player_cannot_move_into_wall() {
        let maze = test_maze();
        let mut app = setup_app(maze);

        // Spawn player at (1,1), try to move up into wall
        let player = app
            .world_mut()
            .spawn((
                Player,
                GridPosition { x: 1, y: 1 },
                MoveDirection(Direction::Up),
                MoveSpeed(10.0),
                Transform::default(),
            ))
            .id();

        app.update();

        // GridPosition should be unchanged
        let pos = app.world().entity(player).get::<GridPosition>().unwrap();
        assert_eq!(*pos, GridPosition { x: 1, y: 1 });
        // MoveDirection should be removed
        assert!(app.world().entity(player).get::<MoveDirection>().is_none());
    }

    #[test]
    fn lerp_created_on_valid_move() {
        let maze = test_maze();
        let mut app = setup_app(maze);

        let player = app
            .world_mut()
            .spawn((
                Player,
                GridPosition { x: 1, y: 1 },
                MoveDirection(Direction::Right),
                MoveSpeed(10.0),
                Transform::default(),
            ))
            .id();

        // Validation creates MoveLerp for valid move
        app.update();
        assert!(app.world().entity(player).get::<MoveLerp>().is_some());
        // GridPosition is updated immediately
        let pos = app.world().entity(player).get::<GridPosition>().unwrap();
        assert_eq!(*pos, GridPosition { x: 2, y: 1 });
    }

    #[test]
    fn no_new_move_while_lerping() {
        let maze = test_maze();
        let mut app = setup_app(maze);

        let player = app
            .world_mut()
            .spawn((
                Player,
                GridPosition { x: 1, y: 1 },
                MoveDirection(Direction::Right),
                MoveSpeed(10.0),
                Transform::default(),
            ))
            .id();

        // First update: starts lerp to (2,1)
        app.update();
        assert_eq!(
            *app.world().entity(player).get::<GridPosition>().unwrap(),
            GridPosition { x: 2, y: 1 }
        );

        // Add another MoveDirection while lerping
        app.world_mut()
            .entity_mut(player)
            .insert(MoveDirection(Direction::Down));
        app.update();

        // Should NOT have moved again — still lerping
        assert_eq!(
            *app.world().entity(player).get::<GridPosition>().unwrap(),
            GridPosition { x: 2, y: 1 }
        );
    }
}
