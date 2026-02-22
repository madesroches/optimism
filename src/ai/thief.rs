//! Thief AI: semi-random movement with bias toward the player at close range.

use crate::components::{Direction, GridPosition};
use crate::plugins::maze::MazeMap;
use rand::Rng;

const CHASE_THRESHOLD: u32 = 8;

pub fn choose_direction(
    enemy_pos: GridPosition,
    player_pos: GridPosition,
    _player_dir: Direction,
    maze: &MazeMap,
) -> Option<Direction> {
    let dist = super::manhattan(&enemy_pos, &player_pos);

    // At close range, chase the player more often
    if dist <= CHASE_THRESHOLD {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.7) {
            // 70% chance to chase when close
            return super::next_direction_toward(enemy_pos, player_pos, maze);
        }
    }

    // Otherwise pick a random valid direction
    random_direction(enemy_pos, maze)
}

fn random_direction(pos: GridPosition, maze: &MazeMap) -> Option<Direction> {
    let dirs = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];
    let valid: Vec<_> = dirs
        .iter()
        .filter(|d| {
            let (dx, dy) = d.delta();
            let target = GridPosition {
                x: pos.x + dx,
                y: pos.y + dy,
            };
            maze.is_walkable_for_enemy(target)
        })
        .collect();

    if valid.is_empty() {
        return None;
    }
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..valid.len());
    Some(*valid[idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thief_returns_valid_direction() {
        let maze = MazeMap::parse("#####\n#P  #\n#  G#\n#####").unwrap();
        // Run multiple times since it's random
        for _ in 0..20 {
            let dir = choose_direction(
                GridPosition { x: 3, y: 2 },
                GridPosition { x: 1, y: 1 },
                Direction::Down,
                &maze,
            );
            assert!(dir.is_some());
        }
    }
}
