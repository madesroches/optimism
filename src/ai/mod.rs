//! Enemy AI modules. Each enemy type has a different targeting strategy.

pub mod brute;
pub mod inquisitor;
pub mod soldier;
pub mod thief;

use crate::components::{Direction, GridPosition};
use crate::plugins::maze::MazeMap;

/// Choose the next direction for an enemy to move, given its position,
/// the player's position and direction, and the maze layout.
/// Returns None if no valid move exists.
pub fn next_direction_toward(
    from: GridPosition,
    target: GridPosition,
    maze: &MazeMap,
) -> Option<Direction> {
    // Use A* to find path, then return direction of first step
    let path = pathfinding::prelude::astar(
        &from,
        |pos| {
            maze.enemy_neighbors(*pos)
                .into_iter()
                .map(|n| (n, 1u32))
        },
        |pos| manhattan(pos, &target),
        |pos| *pos == target,
    );

    path.and_then(|(steps, _cost)| {
        if steps.len() < 2 {
            return None;
        }
        let next = steps[1];
        direction_between(from, next)
    })
}

/// Manhattan distance heuristic.
pub fn manhattan(a: &GridPosition, b: &GridPosition) -> u32 {
    (a.x - b.x).unsigned_abs() + (a.y - b.y).unsigned_abs()
}

/// Determine direction from one adjacent position to another.
fn direction_between(from: GridPosition, to: GridPosition) -> Option<Direction> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    match (dx, dy) {
        (1, 0) => Some(Direction::Right),
        (-1, 0) => Some(Direction::Left),
        (0, 1) => Some(Direction::Down),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_maze() -> MazeMap {
        // Simple corridor maze for AI testing
        MazeMap::parse("#####\n#P .#\n# # #\n#G  #\n#####").unwrap()
    }

    #[test]
    fn next_direction_finds_path() {
        let maze = test_maze();
        let enemy = GridPosition { x: 1, y: 3 };
        let player = GridPosition { x: 1, y: 1 };
        let dir = next_direction_toward(enemy, player, &maze);
        assert!(dir.is_some());
    }

    #[test]
    fn manhattan_distance() {
        let a = GridPosition { x: 0, y: 0 };
        let b = GridPosition { x: 3, y: 4 };
        assert_eq!(manhattan(&a, &b), 7);
    }

    #[test]
    fn direction_between_adjacent() {
        let a = GridPosition { x: 1, y: 1 };
        assert_eq!(
            direction_between(a, GridPosition { x: 2, y: 1 }),
            Some(Direction::Right)
        );
        assert_eq!(
            direction_between(a, GridPosition { x: 0, y: 1 }),
            Some(Direction::Left)
        );
        assert_eq!(
            direction_between(a, GridPosition { x: 1, y: 0 }),
            Some(Direction::Up)
        );
        assert_eq!(
            direction_between(a, GridPosition { x: 1, y: 2 }),
            Some(Direction::Down)
        );
    }
}
