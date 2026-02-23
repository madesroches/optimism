//! Soldier AI: A* directly toward the player. The fastest and most aggressive enemy.

use micromegas_tracing::prelude::*;

use crate::components::{Direction, GridPosition};
use crate::plugins::maze::MazeMap;

#[span_fn]
pub fn choose_direction(
    enemy_pos: GridPosition,
    player_pos: GridPosition,
    _player_dir: Direction,
    maze: &MazeMap,
) -> Option<Direction> {
    super::next_direction_toward(enemy_pos, player_pos, maze)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soldier_heads_toward_player() {
        let maze = MazeMap::parse("#####\n#P  #\n#   #\n#  G#\n#####").unwrap();
        let dir = choose_direction(
            GridPosition { x: 3, y: 3 },
            GridPosition { x: 1, y: 1 },
            Direction::Down,
            &maze,
        );
        // Should move toward player (left or up)
        assert!(matches!(dir, Some(Direction::Left) | Some(Direction::Up)));
    }
}
