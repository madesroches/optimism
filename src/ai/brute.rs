//! Brute AI: A* directly toward the player, same as Soldier but slower.
//! Speed difference is handled by `MoveSpeed`, not the AI module.

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
