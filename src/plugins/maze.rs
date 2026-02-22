//! Maze loading, rendering, and collision grid.
//!
//! Parses ASCII map files into ECS entities and builds a walkability grid
//! for pathfinding. Walls and floors are rendered as colored rectangles.

use bevy::prelude::*;

use crate::app_state::PlayingState;
use crate::components::{GridPosition, Money, Wall};
use crate::resources::CurrentLevel;

pub struct MazePlugin;

impl Plugin for MazePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            (load_maze, auto_start_level.after(load_maze)),
        );
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of a single tile in world units (pixels).
pub const TILE_SIZE: f32 = 32.0;

// Tile colors
const WALL_COLOR: Color = Color::srgb(0.15, 0.15, 0.4);
const MONEY_COLOR: Color = Color::srgb(1.0, 0.85, 0.0);
const PEN_GATE_COLOR: Color = Color::srgb(0.8, 0.3, 0.5);
const FLOOR_COLOR: Color = Color::srgb(0.05, 0.05, 0.1);

/// Money dot size relative to tile.
const MONEY_SIZE: f32 = 6.0;

// ---------------------------------------------------------------------------
// Tile types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Wall,
    Floor,
    Money,
    PenGate,
    PlayerSpawn,
    EnemySpawn,
    WeaponSpawn,
    LuxurySpawn,
}

impl TileType {
    fn from_char(c: char) -> Option<Self> {
        match c {
            '#' => Some(TileType::Wall),
            '.' => Some(TileType::Money),
            ' ' => Some(TileType::Floor),
            'P' => Some(TileType::PlayerSpawn),
            'G' => Some(TileType::EnemySpawn),
            'W' => Some(TileType::WeaponSpawn),
            'L' => Some(TileType::LuxurySpawn),
            '-' => Some(TileType::PenGate),
            _ => None,
        }
    }

    /// Whether entities can walk on this tile (floor-like).
    pub fn is_walkable_floor(&self) -> bool {
        matches!(
            self,
            TileType::Floor
                | TileType::Money
                | TileType::PlayerSpawn
                | TileType::EnemySpawn
                | TileType::WeaponSpawn
                | TileType::LuxurySpawn
        )
    }
}

// ---------------------------------------------------------------------------
// Maze map resource
// ---------------------------------------------------------------------------

/// Stores the parsed maze grid and spawn positions.
#[derive(Resource, Debug, Clone)]
pub struct MazeMap {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<TileType>>,
    pub player_spawn: GridPosition,
    pub enemy_spawns: Vec<GridPosition>,
    pub weapon_spawns: Vec<GridPosition>,
    pub luxury_spawns: Vec<GridPosition>,
}

impl MazeMap {
    /// Parse an ASCII maze string into a MazeMap.
    pub fn parse(text: &str) -> Result<Self, String> {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return Err("Empty maze".to_string());
        }

        let height = lines.len();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        if width == 0 {
            return Err("Maze has zero width".to_string());
        }

        let mut tiles = Vec::with_capacity(height);
        let mut player_spawn = None;
        let mut enemy_spawns = Vec::new();
        let mut weapon_spawns = Vec::new();
        let mut luxury_spawns = Vec::new();

        for (y, line) in lines.iter().enumerate() {
            let mut row = Vec::with_capacity(width);
            for (x, ch) in line.chars().enumerate() {
                let tile = TileType::from_char(ch).ok_or_else(|| {
                    format!("Unknown tile character '{}' at ({}, {})", ch, x, y)
                })?;

                let pos = GridPosition {
                    x: x as i32,
                    y: y as i32,
                };

                match tile {
                    TileType::PlayerSpawn => {
                        if player_spawn.is_some() {
                            return Err(format!("Multiple player spawns at ({}, {})", x, y));
                        }
                        player_spawn = Some(pos);
                    }
                    TileType::EnemySpawn => enemy_spawns.push(pos),
                    TileType::WeaponSpawn => weapon_spawns.push(pos),
                    TileType::LuxurySpawn => luxury_spawns.push(pos),
                    _ => {}
                }

                row.push(tile);
            }
            // Pad short rows with Floor
            while row.len() < width {
                row.push(TileType::Floor);
            }
            tiles.push(row);
        }

        let player_spawn = player_spawn.ok_or("No player spawn ('P') found in maze")?;

        Ok(MazeMap {
            width,
            height,
            tiles,
            player_spawn,
            enemy_spawns,
            weapon_spawns,
            luxury_spawns,
        })
    }

    /// Check if a tile is walkable. Out-of-bounds is not walkable.
    pub fn is_walkable(&self, pos: GridPosition) -> bool {
        self.tile_at(pos)
            .is_some_and(|t| t.is_walkable_floor() || t == TileType::PenGate)
    }

    /// Check if a tile is walkable for the player (pen gates are NOT walkable for player).
    pub fn is_walkable_for_player(&self, pos: GridPosition) -> bool {
        self.tile_at(pos).is_some_and(|t| t.is_walkable_floor())
    }

    /// Check if a tile is walkable for enemies (pen gates ARE walkable for enemies).
    pub fn is_walkable_for_enemy(&self, pos: GridPosition) -> bool {
        self.is_walkable(pos)
    }

    /// Get tile type at a position, or None if out of bounds.
    pub fn tile_at(&self, pos: GridPosition) -> Option<TileType> {
        if pos.x < 0 || pos.y < 0 {
            return None;
        }
        let (x, y) = (pos.x as usize, pos.y as usize);
        self.tiles.get(y).and_then(|row| row.get(x)).copied()
    }

    /// Get walkable neighbors for pathfinding (4-directional, for enemies).
    pub fn enemy_neighbors(&self, pos: GridPosition) -> Vec<GridPosition> {
        let dirs = [(0, -1), (0, 1), (-1, 0), (1, 0)];
        dirs.iter()
            .map(|(dx, dy)| GridPosition {
                x: pos.x + dx,
                y: pos.y + dy,
            })
            .filter(|p| self.is_walkable_for_enemy(*p))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Coordinate conversion
// ---------------------------------------------------------------------------

/// Convert a grid position to world coordinates.
/// Grid (0,0) is top-left; world Y is flipped (positive up).
pub fn grid_to_world(pos: GridPosition, maze_width: usize, maze_height: usize) -> Vec2 {
    let half_w = (maze_width as f32 * TILE_SIZE) / 2.0;
    let half_h = (maze_height as f32 * TILE_SIZE) / 2.0;
    Vec2::new(
        pos.x as f32 * TILE_SIZE + TILE_SIZE / 2.0 - half_w,
        -(pos.y as f32 * TILE_SIZE + TILE_SIZE / 2.0 - half_h),
    )
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

/// Marker for pen gate entities.
#[derive(Component, Debug)]
pub struct PenGate;

/// Marker for entities that belong to the current maze (despawned on level transition).
#[derive(Component, Debug)]
pub struct MazeEntity;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Level-to-map-file mapping.
fn map_file_for_level(level: u32) -> String {
    match level {
        1 => "assets/maps/level_01.txt".to_string(),
        // Phases 7 adds more levels; for now cycle back to level 1
        _ => "assets/maps/level_01.txt".to_string(),
    }
}

/// Load and spawn the maze for the current level.
pub fn load_maze(mut commands: Commands, level: Res<CurrentLevel>) {
    let path = map_file_for_level(level.0);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read maze file {}: {}", path, e));

    let maze = MazeMap::parse(&text)
        .unwrap_or_else(|e| panic!("Failed to parse maze file {}: {}", path, e));

    // Spawn tile entities
    for y in 0..maze.height {
        for x in 0..maze.width {
            let tile = maze.tiles[y][x];
            let pos = GridPosition {
                x: x as i32,
                y: y as i32,
            };
            let world_pos = grid_to_world(pos, maze.width, maze.height);

            // Floor background for all walkable tiles
            match tile {
                TileType::Wall => {
                    commands.spawn((
                        Wall,
                        pos,
                        MazeEntity,
                        Sprite::from_color(WALL_COLOR, Vec2::splat(TILE_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                    ));
                }
                TileType::PenGate => {
                    // Floor under the gate
                    commands.spawn((
                        MazeEntity,
                        Sprite::from_color(FLOOR_COLOR, Vec2::splat(TILE_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                    ));
                    commands.spawn((
                        PenGate,
                        pos,
                        MazeEntity,
                        Sprite::from_color(PEN_GATE_COLOR, Vec2::new(TILE_SIZE, 4.0)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
                    ));
                }
                TileType::Money => {
                    // Floor background
                    commands.spawn((
                        MazeEntity,
                        Sprite::from_color(FLOOR_COLOR, Vec2::splat(TILE_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                    ));
                    // Money dot
                    commands.spawn((
                        Money,
                        pos,
                        MazeEntity,
                        Sprite::from_color(MONEY_COLOR, Vec2::splat(MONEY_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
                    ));
                }
                _ => {
                    // Floor tile for empty, spawns, etc.
                    commands.spawn((
                        MazeEntity,
                        Sprite::from_color(FLOOR_COLOR, Vec2::splat(TILE_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                    ));
                }
            }
        }
    }

    commands.insert_resource(maze);
}

/// Temporary shim: immediately transition from LevelIntro to Playing.
/// Phase 7 replaces this with a real intro screen.
fn auto_start_level(mut next_state: ResMut<NextState<PlayingState>>) {
    next_state.set(PlayingState::Playing);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MAZE: &str = "\
####
#P.#
#G #
####";

    #[test]
    fn parse_small_maze() {
        let maze = MazeMap::parse(TEST_MAZE).unwrap();
        assert_eq!(maze.width, 4);
        assert_eq!(maze.height, 4);
        assert_eq!(maze.player_spawn, GridPosition { x: 1, y: 1 });
        assert_eq!(maze.enemy_spawns.len(), 1);
        assert_eq!(maze.enemy_spawns[0], GridPosition { x: 1, y: 2 });
    }

    #[test]
    fn walkability() {
        let maze = MazeMap::parse(TEST_MAZE).unwrap();
        // Walls are not walkable
        assert!(!maze.is_walkable(GridPosition { x: 0, y: 0 }));
        // Money dot is walkable
        assert!(maze.is_walkable(GridPosition { x: 2, y: 1 }));
        // Player spawn is walkable
        assert!(maze.is_walkable(GridPosition { x: 1, y: 1 }));
        // Enemy spawn is walkable
        assert!(maze.is_walkable(GridPosition { x: 1, y: 2 }));
        // Floor is walkable
        assert!(maze.is_walkable(GridPosition { x: 2, y: 2 }));
        // Out of bounds is not walkable
        assert!(!maze.is_walkable(GridPosition { x: -1, y: 0 }));
        assert!(!maze.is_walkable(GridPosition { x: 10, y: 10 }));
    }

    #[test]
    fn pen_gate_walkability() {
        let maze = MazeMap::parse("#-#\n#P#\n###").unwrap();
        let gate = GridPosition { x: 1, y: 0 };
        // Enemy can walk through pen gate
        assert!(maze.is_walkable_for_enemy(gate));
        // Player cannot walk through pen gate
        assert!(!maze.is_walkable_for_player(gate));
    }

    #[test]
    fn grid_to_world_center() {
        // For a 4x4 maze, tile (0,0) should be top-left
        let pos = grid_to_world(GridPosition { x: 0, y: 0 }, 4, 4);
        // Center of tile (0,0) in a 4-wide maze: 0*32 + 16 - 64 = -48
        assert!((pos.x - (-48.0)).abs() < 0.01);
        // Y is flipped: -(0*32 + 16 - 64) = 48
        assert!((pos.y - 48.0).abs() < 0.01);
    }

    #[test]
    fn grid_to_world_roundtrip() {
        let width = 10;
        let height = 8;
        for y in 0..height {
            for x in 0..width {
                let pos = GridPosition {
                    x: x as i32,
                    y: y as i32,
                };
                let world = grid_to_world(pos, width, height);
                // Reverse: x = (world.x + half_w - TILE_SIZE/2) / TILE_SIZE
                let half_w = (width as f32 * TILE_SIZE) / 2.0;
                let half_h = (height as f32 * TILE_SIZE) / 2.0;
                let rx = ((world.x + half_w - TILE_SIZE / 2.0) / TILE_SIZE).round() as i32;
                let ry = ((-world.y + half_h - TILE_SIZE / 2.0) / TILE_SIZE).round() as i32;
                assert_eq!(rx, pos.x, "X roundtrip failed for ({}, {})", x, y);
                assert_eq!(ry, pos.y, "Y roundtrip failed for ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn malformed_maze_no_player() {
        let result = MazeMap::parse("####\n#..#\n####");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No player spawn"));
    }

    #[test]
    fn malformed_maze_bad_char() {
        let result = MazeMap::parse("####\n#P?#\n####");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tile character"));
    }

    #[test]
    fn malformed_maze_duplicate_player() {
        let result = MazeMap::parse("####\n#PP#\n####");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Multiple player spawns"));
    }

    #[test]
    fn weapon_and_luxury_spawns_parsed() {
        let maze = MazeMap::parse("####\n#PW#\n#L #\n####").unwrap();
        assert_eq!(maze.weapon_spawns.len(), 1);
        assert_eq!(maze.weapon_spawns[0], GridPosition { x: 2, y: 1 });
        assert_eq!(maze.luxury_spawns.len(), 1);
        assert_eq!(maze.luxury_spawns[0], GridPosition { x: 1, y: 2 });
    }

    #[test]
    fn enemy_neighbors() {
        let maze = MazeMap::parse(TEST_MAZE).unwrap();
        // From (1,2) which is 'G' — neighbors are (1,1)=P and (2,2)=floor
        let neighbors = maze.enemy_neighbors(GridPosition { x: 1, y: 2 });
        assert!(neighbors.contains(&GridPosition { x: 1, y: 1 }));
        assert!(neighbors.contains(&GridPosition { x: 2, y: 2 }));
        assert!(!neighbors.contains(&GridPosition { x: 0, y: 2 })); // wall
    }
}
