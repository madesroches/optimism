//! Enemy spawning, AI dispatch, collision with player, death handling, and pen release.

use bevy::prelude::*;
use micromegas_tracing::prelude::span_scope;

use crate::ai;
use crate::app_state::{AppState, PlayingState};
use crate::components::*;
use crate::plugins::combat::{ActiveWeapon, Frightened, Respawning, WeaponTimer};
use crate::plugins::maze::{grid_to_world, load_maze, MazeMap, MazeEntity, TILE_SIZE};
use crate::plugins::sprites::{
    AnimationState, AnimationTimer, CharacterSheetRef, FacingDirection, SpriteSheetLibrary,
};
use crate::resources::{GameStats, LevelConfig, Lives};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            spawn_enemies.after(load_maze),
        );
        app.add_systems(
            Update,
            (enemy_ai, enemy_player_collision.after(enemy_ai))
                .run_if(in_state(PlayingState::Playing)),
        );
        app.add_systems(OnEnter(PlayingState::PlayerDeath), handle_player_death);
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            init_pen_release_timer.after(load_maze),
        );
        app.add_systems(
            Update,
            pen_release.run_if(in_state(PlayingState::Playing)),
        );
        app.add_systems(
            OnEnter(PlayingState::LevelTransition),
            remove_pen_release_timer,
        );
        app.add_systems(OnExit(AppState::InGame), remove_pen_release_timer);
    }
}

/// Timer that controls when enemies are released from the pen.
#[derive(Resource)]
pub struct PenReleaseTimer {
    pub timer: Timer,
}

/// Spawn enemies at their maze positions.
pub fn spawn_enemies(
    mut commands: Commands,
    maze: Res<MazeMap>,
    config: Res<LevelConfig>,
    mut library: ResMut<SpriteSheetLibrary>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    if config.is_garden {
        return;
    }

    let enemy_types = [
        ("soldier", EnemyKind::Soldier, 4.5),
        ("inquisitor", EnemyKind::Inquisitor, 4.0),
        ("thief", EnemyKind::Thief, 3.5),
        ("brute", EnemyKind::Brute, 3.0),
    ];

    for (i, spawn_pos) in maze.enemy_spawns.iter().enumerate() {
        let (name, kind, base_speed) = if i < enemy_types.len() {
            enemy_types[i]
        } else {
            enemy_types[i % enemy_types.len()]
        };
        let speed = base_speed * config.enemy_speed_multiplier;

        // Load sprite sheet if needed
        if !library.sheets.contains_key(name) {
            let _ = library.load(
                name,
                &format!("sprites/{}.png", name),
                &asset_server,
                &mut layouts,
            );
        }

        let world_pos = grid_to_world(*spawn_pos, maze.width, maze.height);

        let mut entity = commands.spawn((
            Enemy,
            kind,
            InPen,
            *spawn_pos,
            SpawnPosition(*spawn_pos),
            MoveSpeed(speed),
            FacingDirection::Down,
            MazeEntity,
            Transform::from_xyz(world_pos.x, world_pos.y, 10.0),
        ));

        if let Some(sheet) = library.sheets.get(name) {
            let start_index = sheet
                .meta
                .animations
                .get("idle")
                .or_else(|| sheet.meta.animations.get("walk_down"))
                .map(|r| r.start)
                .unwrap_or(0);

            entity.insert((
                Sprite {
                    image: sheet.image.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: sheet.layout.clone(),
                        index: start_index,
                    }),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                CharacterSheetRef(name.to_string()),
                AnimationState::new("walk_down", true),
                AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
            ));
        }
    }
}

/// Run AI for each enemy: choose target direction based on enemy kind.
/// Frightened enemies flee; respawning enemies don't move.
#[allow(clippy::type_complexity)]
fn enemy_ai(
    maze: Res<MazeMap>,
    player_query: Query<(&GridPosition, &FacingDirection), With<Player>>,
    mut enemy_query: Query<
        (Entity, &GridPosition, &EnemyKind, &mut FacingDirection, Has<Frightened>),
        (With<Enemy>, Without<Player>, Without<InPen>, Without<MoveLerp>, Without<MoveDirection>, Without<Respawning>),
    >,
    mut commands: Commands,
) {
    span_scope!("enemy_ai");

    let Ok((player_pos, player_facing)) = player_query.single() else {
        return;
    };

    let player_dir: Direction = match *player_facing {
        FacingDirection::Up => Direction::Up,
        FacingDirection::Down => Direction::Down,
        FacingDirection::Left => Direction::Left,
        FacingDirection::Right => Direction::Right,
    };

    for (entity, enemy_pos, kind, mut facing, is_frightened) in &mut enemy_query {
        let dir = if is_frightened {
            crate::plugins::combat::frightened_direction(*enemy_pos, *player_pos, &maze)
        } else {
            match kind {
                EnemyKind::Soldier => {
                    ai::soldier::choose_direction(*enemy_pos, *player_pos, player_dir, &maze)
                }
                EnemyKind::Inquisitor => {
                    ai::inquisitor::choose_direction(*enemy_pos, *player_pos, player_dir, &maze)
                }
                EnemyKind::Thief => {
                    ai::thief::choose_direction(*enemy_pos, *player_pos, player_dir, &maze)
                }
                EnemyKind::Brute => {
                    ai::brute::choose_direction(*enemy_pos, *player_pos, player_dir, &maze)
                }
            }
        };

        if let Some(dir) = dir {
            *facing = dir.into();
            commands.entity(entity).insert(MoveDirection(dir));
        }
    }
}

/// Insert a fresh pen release timer from LevelConfig at start of each level.
fn init_pen_release_timer(mut commands: Commands, config: Res<LevelConfig>) {
    commands.insert_resource(PenReleaseTimer {
        timer: Timer::from_seconds(config.pen_release_interval_secs, TimerMode::Repeating),
    });
}

/// Remove the pen release timer when leaving a level.
fn remove_pen_release_timer(mut commands: Commands) {
    commands.remove_resource::<PenReleaseTimer>();
}

/// Check if any non-frightened, non-respawning enemy occupies the same tile as the player,
/// or if the player and enemy swapped tiles this frame (head-on pass-through).
/// Frightened enemies are handled by combat::player_kills_enemy instead.
#[allow(clippy::type_complexity)]
fn enemy_player_collision(
    player_query: Query<(&GridPosition, Option<&PreviousGridPosition>), With<Player>>,
    enemy_query: Query<(&GridPosition, Option<&PreviousGridPosition>), (With<Enemy>, Without<InPen>, Without<Frightened>, Without<Respawning>)>,
    mut next_state: ResMut<NextState<PlayingState>>,
    mut lives: ResMut<Lives>,
    mut stats: ResMut<GameStats>,
) {
    let Ok((player_pos, player_prev)) = player_query.single() else {
        return;
    };
    for (enemy_pos, enemy_prev) in &enemy_query {
        // Same tile
        let same_tile = player_pos == enemy_pos;
        // Crossed paths: player moved from A→B while enemy moved from B→A
        let crossed = match (player_prev, enemy_prev) {
            (Some(pp), Some(ep)) => pp.0 == *enemy_pos && ep.0 == *player_pos,
            _ => false,
        };
        if same_tile || crossed {
            if lives.0 > 0 {
                lives.0 -= 1;
            }
            stats.deaths += 1;
            next_state.set(PlayingState::PlayerDeath);
            return;
        }
    }
}

/// On player death: reset positions, clean up movement and combat state.
/// If no lives left, game over.
#[allow(clippy::type_complexity)]
fn handle_player_death(
    mut commands: Commands,
    mut player_query: Query<(Entity, &mut GridPosition, &SpawnPosition, &mut InputDirection), (With<Player>, Without<Enemy>)>,
    mut enemy_query: Query<(Entity, &mut GridPosition, &SpawnPosition), (With<Enemy>, Without<Player>)>,
    maze: Res<MazeMap>,
    lives: Res<Lives>,
    config: Res<LevelConfig>,
    mut next_playing: ResMut<NextState<PlayingState>>,
    mut next_app: ResMut<NextState<AppState>>,
) {
    if lives.0 == 0 {
        next_app.set(AppState::GameOver);
        return;
    }

    // Reset player position and clean up movement/weapon components
    if let Ok((entity, mut player_pos, spawn, mut input)) = player_query.single_mut() {
        *player_pos = spawn.0;
        input.0 = None;
        let world = grid_to_world(spawn.0, maze.width, maze.height);
        commands
            .entity(entity)
            .remove::<MoveLerp>()
            .remove::<MoveDirection>()
            .remove::<PreviousGridPosition>()
            .remove::<ActiveWeapon>()
            .remove::<WeaponTimer>()
            .insert(Transform::from_xyz(world.x, world.y, 10.0));
    }

    // Reset enemy positions, return to pen, clear combat state
    for (entity, mut enemy_pos, spawn) in &mut enemy_query {
        *enemy_pos = spawn.0;
        let world = grid_to_world(spawn.0, maze.width, maze.height);
        commands
            .entity(entity)
            .remove::<MoveLerp>()
            .remove::<MoveDirection>()
            .remove::<PreviousGridPosition>()
            .remove::<Frightened>()
            .remove::<Respawning>()
            .insert(InPen)
            .insert(Visibility::Inherited)
            .insert(Transform::from_xyz(world.x, world.y, 10.0));
    }

    // Reset pen release timer so enemies are released gradually
    commands.insert_resource(PenReleaseTimer {
        timer: Timer::from_seconds(config.pen_release_interval_secs, TimerMode::Repeating),
    });

    next_playing.set(PlayingState::Playing);
}

/// Release enemies from the pen one at a time.
fn pen_release(
    time: Res<Time>,
    mut timer: ResMut<PenReleaseTimer>,
    mut commands: Commands,
    pen_enemies: Query<Entity, (With<Enemy>, With<InPen>)>,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    // Release the first InPen enemy
    if let Some(entity) = pen_enemies.iter().next() {
        commands.entity(entity).remove::<InPen>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::level_config;
    use bevy::state::app::StatesPlugin;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<AppState>();
        app.add_sub_state::<PlayingState>();
        app.insert_resource(Lives(3));
        app.init_resource::<GameStats>();
        app.insert_resource(PenReleaseTimer {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        });
        // MazeMap and LevelConfig required by handle_player_death
        let maze = MazeMap::parse("####\n#P #\n# G#\n####").unwrap();
        app.insert_resource(maze);
        app.insert_resource(level_config(1));
        app.add_systems(
            Update,
            (enemy_player_collision, pen_release).run_if(in_state(PlayingState::Playing)),
        );
        app.add_systems(OnEnter(PlayingState::PlayerDeath), handle_player_death);

        // Transition to Playing
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
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
    fn enemy_collision_triggers_death() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };
        app.world_mut().spawn((Player, pos, SpawnPosition(pos), InputDirection::default()));
        app.world_mut().spawn((
            Enemy,
            EnemyKind::Soldier,
            pos,
            SpawnPosition(pos),
        ));

        app.update();

        let lives = app.world().resource::<Lives>();
        assert_eq!(lives.0, 2);
    }

    #[test]
    fn zero_lives_triggers_game_over() {
        let mut app = setup_app();
        app.insert_resource(Lives(1));

        let pos = GridPosition { x: 1, y: 1 };
        app.world_mut().spawn((Player, pos, SpawnPosition(pos), InputDirection::default()));
        app.world_mut().spawn((
            Enemy,
            EnemyKind::Soldier,
            pos,
            SpawnPosition(pos),
        ));

        // Collision decrements lives to 0
        app.update();

        // Death handler should transition to GameOver
        for _ in 0..5 {
            app.update();
        }

        let state = app.world().resource::<State<AppState>>();
        assert_eq!(*state.get(), AppState::GameOver);
    }

    #[test]
    fn head_on_crossing_triggers_death() {
        let mut app = setup_app();
        // Player at (1,1) with PreviousGridPosition of (2,1) — moved left
        app.world_mut().spawn((
            Player,
            GridPosition { x: 1, y: 1 },
            PreviousGridPosition(GridPosition { x: 2, y: 1 }),
            SpawnPosition(GridPosition { x: 1, y: 1 }),
            InputDirection::default(),
        ));
        // Enemy at (2,1) with PreviousGridPosition of (1,1) — moved right
        app.world_mut().spawn((
            Enemy,
            EnemyKind::Soldier,
            GridPosition { x: 2, y: 1 },
            PreviousGridPosition(GridPosition { x: 1, y: 1 }),
            SpawnPosition(GridPosition { x: 2, y: 1 }),
        ));

        app.update();

        // Crossed paths should trigger death
        let lives = app.world().resource::<Lives>();
        assert_eq!(lives.0, 2);
    }

    #[test]
    fn in_pen_enemies_dont_collide() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };
        app.world_mut().spawn((Player, pos, SpawnPosition(pos), InputDirection::default()));
        app.world_mut().spawn((
            Enemy,
            EnemyKind::Soldier,
            InPen,
            pos,
            SpawnPosition(pos),
        ));

        app.update();

        // InPen enemy should NOT trigger collision
        let lives = app.world().resource::<Lives>();
        assert_eq!(lives.0, 3);
    }
}
