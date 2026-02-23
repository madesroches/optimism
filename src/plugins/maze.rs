//! Maze loading, rendering, and collision grid.
//!
//! Parses ASCII map files into ECS entities and builds a walkability grid
//! for pathfinding. Walls and floors are rendered as colored rectangles.

use bevy::prelude::*;
use micromegas_tracing::prelude::{info, span_scope};

use crate::app_state::{AppState, PlayingState};
use crate::components::{GridPosition, Money, Wall};
use crate::resources::{level_config, CurrentLevel, LevelConfig};

pub struct MazePlugin;

impl Plugin for MazePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            (
                update_level_config,
                load_maze.after(update_level_config),
                show_level_intro.after(load_maze),
            ),
        );
        app.add_systems(
            Update,
            level_intro_input.run_if(in_state(PlayingState::LevelIntro)),
        );
        app.add_systems(OnExit(PlayingState::LevelIntro), despawn_level_intro_ui);

        app.add_systems(
            OnEnter(PlayingState::LevelComplete),
            start_level_complete_timer,
        );
        app.add_systems(
            Update,
            level_complete_delay.run_if(in_state(PlayingState::LevelComplete)),
        );

        app.add_systems(
            OnEnter(PlayingState::LevelTransition),
            (despawn_maze_entities, advance_level.after(despawn_maze_entities)),
        );

        // Clean up all maze entities when leaving InGame (e.g. GameOver).
        // LevelTransition already despawns them for normal level progression,
        // but exiting via GameOver skips that path entirely.
        app.add_systems(OnExit(AppState::InGame), cleanup_on_exit_game);
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

// ---------------------------------------------------------------------------
// Level intro UI
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct LevelIntroUI;

#[derive(Resource, Deref, DerefMut)]
pub struct LevelIntroTimer(pub Timer);

#[derive(Resource, Deref, DerefMut)]
pub struct LevelCompleteTimer(pub Timer);

/// Pangloss quotes for level intros.
const INTRO_QUOTES: &[&str] = &[
    "All is for the best in this best of all possible worlds.",
    "Let us cultivate our garden.",
    "Optimism is the madness of insisting that all is well.",
    "We must cultivate our garden, said Candide.",
    "In this best of all possible worlds, all events are linked.",
    "The nose was formed to bear spectacles.",
    "Private misfortunes make for the general good.",
    "There is no effect without a cause.",
    "Everything is necessary, everything is useful.",
    "If this is the best of worlds, what are the others?",
    "Man was not born to be idle.",
    "Work keeps at bay three great evils: boredom, vice, and need.",
    "Judge a man by his questions rather than his answers.",
];

/// Insert LevelConfig resource for the current level.
fn update_level_config(mut commands: Commands, level: Res<CurrentLevel>) {
    commands.insert_resource(level_config(level.0));
}

/// Load and spawn the maze for the current level.
pub fn load_maze(mut commands: Commands, config: Res<LevelConfig>) {
    span_scope!("maze_load");
    let path = &config.maze_file;
    let text = std::fs::read_to_string(path)
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
                    commands.spawn((
                        MazeEntity,
                        Sprite::from_color(FLOOR_COLOR, Vec2::splat(TILE_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                    ));
                    commands.spawn((
                        Money,
                        pos,
                        MazeEntity,
                        Sprite::from_color(MONEY_COLOR, Vec2::splat(MONEY_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
                    ));
                }
                _ => {
                    commands.spawn((
                        MazeEntity,
                        Sprite::from_color(FLOOR_COLOR, Vec2::splat(TILE_SIZE)),
                        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                    ));
                }
            }
        }
    }

    info!("maze loaded: {} ({}x{})", path, maze.width, maze.height);
    commands.insert_resource(maze);
}

/// Show level intro UI with level number and a Pangloss quote.
fn show_level_intro(mut commands: Commands, level: Res<CurrentLevel>) {
    let quote_idx = (level.0 as usize).wrapping_sub(1) % INTRO_QUOTES.len();
    let quote = INTRO_QUOTES[quote_idx];

    commands.insert_resource(LevelIntroTimer(Timer::from_seconds(3.0, TimerMode::Once)));

    commands
        .spawn((
            LevelIntroUI,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("Level {}", level.0)),
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new(quote.to_string()),
                TextColor(Color::srgb(0.8, 0.8, 0.9)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("Press Enter to begin"),
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));
        });
}

/// Tick intro timer or respond to Enter press → transition to Playing.
fn level_intro_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut timer: ResMut<LevelIntroTimer>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    timer.tick(time.delta());
    if timer.just_finished() || keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(PlayingState::Playing);
    }
}

/// Clean up level intro UI.
fn despawn_level_intro_ui(
    mut commands: Commands,
    query: Query<Entity, With<LevelIntroUI>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LevelIntroTimer>();
}

/// Start a 1.5s timer on LevelComplete.
fn start_level_complete_timer(mut commands: Commands) {
    commands.insert_resource(LevelCompleteTimer(Timer::from_seconds(1.5, TimerMode::Once)));
}

/// Tick level complete timer → transition to LevelTransition.
fn level_complete_delay(
    time: Res<Time>,
    mut timer: ResMut<LevelCompleteTimer>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
        next_state.set(PlayingState::LevelTransition);
    }
}

/// Despawn all maze entities during level transition.
fn despawn_maze_entities(
    mut commands: Commands,
    query: Query<Entity, With<MazeEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<MazeMap>();
    commands.remove_resource::<LevelCompleteTimer>();
    commands.remove_resource::<LevelConfig>();
}

/// Clean up maze entities when exiting InGame state (covers GameOver path).
fn cleanup_on_exit_game(
    mut commands: Commands,
    query: Query<Entity, With<MazeEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<MazeMap>();
    commands.remove_resource::<LevelCompleteTimer>();
    commands.remove_resource::<LevelConfig>();
}

/// Increment level and transition back to LevelIntro.
fn advance_level(
    mut level: ResMut<CurrentLevel>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    level.0 += 1;
    next_state.set(PlayingState::LevelIntro);
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

    #[test]
    fn parse_all_level_files() {
        for name in &["level_01", "level_02", "level_03", "level_04", "garden"] {
            let path = format!("assets/maps/{}.txt", name);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
            let maze = MazeMap::parse(&text)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path, e));
            assert!(maze.width > 0, "{} has zero width", name);
            assert!(maze.height > 0, "{} has zero height", name);
        }
    }

    #[test]
    fn garden_has_no_enemies_or_weapons() {
        let text = std::fs::read_to_string("assets/maps/garden.txt").unwrap();
        let maze = MazeMap::parse(&text).unwrap();
        assert!(maze.enemy_spawns.is_empty(), "Garden should have no enemy spawns");
        assert!(maze.weapon_spawns.is_empty(), "Garden should have no weapon spawns");
        assert!(maze.luxury_spawns.is_empty(), "Garden should have no luxury spawns");
    }
}
