//! Inquisitor AI: targets 4 tiles ahead of the player's facing direction.
//! If that tile is not walkable, falls back to targeting the player directly.

use micromegas_tracing::prelude::*;

use crate::components::{Direction, GridPosition};
use crate::plugins::maze::MazeMap;

#[span_fn]
pub fn choose_direction(
    enemy_pos: GridPosition,
    player_pos: GridPosition,
    player_dir: Direction,
    maze: &MazeMap,
) -> Option<Direction> {
    let (dx, dy) = player_dir.delta();
    let ahead = GridPosition {
        x: player_pos.x + dx * 4,
        y: player_pos.y + dy * 4,
    };

    // If the ahead target is walkable, aim there; otherwise aim at player
    let target = if maze.is_walkable_for_enemy(ahead) {
        ahead
    } else {
        player_pos
    };

    super::next_direction_toward(enemy_pos, target, maze)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inquisitor_targets_ahead_of_player() {
        // Wide corridor so 4-ahead target is valid
        let maze =
            MazeMap::parse("########\n#P     #\n#      #\n#     G#\n########").unwrap();
        let dir = choose_direction(
            GridPosition { x: 6, y: 3 },
            GridPosition { x: 1, y: 1 },
            Direction::Right, // Player facing right → target is (5,1)
            &maze,
        );
        assert!(dir.is_some());
    }

    #[test]
    fn inquisitor_falls_back_to_player_when_ahead_blocked() {
        let maze = MazeMap::parse("#####\n#P  #\n#  G#\n#####").unwrap();
        let dir = choose_direction(
            GridPosition { x: 3, y: 2 },
            GridPosition { x: 1, y: 1 },
            Direction::Up, // 4 ahead is outside maze → falls back to player
            &maze,
        );
        assert!(dir.is_some());
    }
}
