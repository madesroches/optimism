use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Grid and spatial
// ---------------------------------------------------------------------------

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

/// Cardinal direction for movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Grid offset for this direction.
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

/// Current movement direction. Cleared when the entity arrives at a tile.
#[derive(Component, Debug, Clone, Copy)]
pub struct MoveDirection(pub Direction);

/// Tiles per second.
#[derive(Component, Debug, Clone, Copy)]
pub struct MoveSpeed(pub f32);

/// Buffered input direction (player only). Applied when arriving at a tile.
#[derive(Component, Debug, Default)]
pub struct InputDirection(pub Option<Direction>);

/// Smooth visual interpolation between tiles.
#[derive(Component, Debug)]
pub struct MoveLerp {
    pub from: Vec2,
    pub to: Vec2,
    pub t: f32,
}

// ---------------------------------------------------------------------------
// Entity markers
// ---------------------------------------------------------------------------

#[derive(Component, Debug)]
pub struct Player;

#[derive(Component, Debug)]
pub struct Enemy;

/// The kind of enemy, determining AI behavior.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    Soldier,
    Inquisitor,
    Thief,
    Brute,
}

/// Marker: enemy is inside the pen and waiting for release.
#[derive(Component, Debug)]
pub struct InPen;

#[derive(Component, Debug)]
pub struct Wall;

#[derive(Component, Debug)]
pub struct Money;

/// Spawn position for respawning after death.
#[derive(Component, Debug, Clone, Copy)]
pub struct SpawnPosition(pub GridPosition);
