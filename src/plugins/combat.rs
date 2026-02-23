//! Weapons and combat: weapon pickups, frightened mode, enemy kills, respawning.

use bevy::prelude::*;
use micromegas::tracing::prelude::{imetric, info};

use crate::app_state::PlayingState;
use crate::components::*;
use crate::events::{EnemyKilled, WeaponPickedUp};
use crate::plugins::maze::{grid_to_world, load_maze, MazeMap, MazeEntity, TILE_SIZE};
use crate::resources::{GameStats, LevelConfig, Score};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            spawn_weapons.after(load_maze),
        );
        app.add_systems(
            Update,
            (
                weapon_pickup,
                weapon_timer.after(weapon_pickup),
                player_kills_enemy.after(weapon_pickup),
                enemy_respawn,
            )
                .run_if(in_state(PlayingState::Playing)),
        );
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Weapon type determines combat flavor and score bonus.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponType {
    BrassKnuckles,
    Bat,
    Knife,
    Axe,
    Chainsaw,
}

/// Marker for weapon pickup entities in the maze.
#[derive(Component, Debug)]
pub struct WeaponPickup;

/// Player's currently active weapon.
#[derive(Component, Debug)]
pub struct ActiveWeapon(pub WeaponType);

/// Timer for how long the weapon effect lasts.
#[derive(Component, Debug, Deref, DerefMut)]
pub struct WeaponTimer(pub Timer);

/// Marker: enemy is frightened (player has a weapon).
#[derive(Component, Debug)]
pub struct Frightened;

/// Enemy is respawning (hidden, timer ticking).
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Respawning(pub Timer);

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawn weapon pickups at WeaponSpawn positions using LevelConfig.
fn spawn_weapons(mut commands: Commands, maze: Res<MazeMap>, config: Res<LevelConfig>) {
    let weapon_color = Color::srgb(0.9, 0.2, 0.2);

    for spawn_pos in &maze.weapon_spawns {
        let world_pos = grid_to_world(*spawn_pos, maze.width, maze.height);
        commands.spawn((
            WeaponPickup,
            config.weapon_type,
            *spawn_pos,
            MazeEntity,
            Sprite::from_color(weapon_color, Vec2::splat(TILE_SIZE * 0.6)),
            Transform::from_xyz(world_pos.x, world_pos.y, 2.0),
        ));
    }
}

/// Player touches a weapon pickup → activate weapon and frighten enemies.
#[allow(clippy::type_complexity)]
fn weapon_pickup(
    mut commands: Commands,
    player_query: Query<(Entity, &GridPosition), With<Player>>,
    weapon_query: Query<(Entity, &GridPosition, &WeaponType), With<WeaponPickup>>,
    enemy_query: Query<Entity, (With<Enemy>, Without<Frightened>, Without<Respawning>)>,
    config: Option<Res<LevelConfig>>,
) {
    let Ok((player_entity, player_pos)) = player_query.single() else {
        return;
    };

    let duration = config.map(|c| c.weapon_duration_secs).unwrap_or(8.0);

    for (weapon_entity, weapon_pos, weapon_type) in &weapon_query {
        if player_pos == weapon_pos {
            commands.entity(weapon_entity).despawn();

            commands.entity(player_entity).insert((
                ActiveWeapon(*weapon_type),
                WeaponTimer(Timer::from_seconds(duration, TimerMode::Once)),
            ));

            for enemy_entity in &enemy_query {
                commands.entity(enemy_entity).insert(Frightened);
            }

            commands.trigger(WeaponPickedUp);
            break;
        }
    }
}

/// Tick the weapon timer; remove weapon and frightened state on expiry.
fn weapon_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut player_query: Query<(Entity, &mut WeaponTimer), With<Player>>,
    frightened_query: Query<Entity, With<Frightened>>,
) {
    for (player_entity, mut timer) in &mut player_query {
        timer.tick(time.delta());
        if timer.just_finished() {
            commands
                .entity(player_entity)
                .remove::<ActiveWeapon>()
                .remove::<WeaponTimer>();

            // Remove frightened from all enemies
            for enemy_entity in &frightened_query {
                commands.entity(enemy_entity).remove::<Frightened>();
            }
        }
    }
}

/// Armed player kills frightened enemies on contact.
#[allow(clippy::type_complexity)]
fn player_kills_enemy(
    mut commands: Commands,
    player_query: Query<(&GridPosition, &ActiveWeapon), With<Player>>,
    enemy_query: Query<
        (Entity, &GridPosition),
        (With<Enemy>, With<Frightened>, Without<Respawning>),
    >,
    mut score: ResMut<Score>,
    mut stats: ResMut<GameStats>,
) {
    let Ok((player_pos, active_weapon)) = player_query.single() else {
        return;
    };

    for (enemy_entity, enemy_pos) in &enemy_query {
        if player_pos == enemy_pos {
            commands
                .entity(enemy_entity)
                .insert(Respawning(Timer::from_seconds(5.0, TimerMode::Once)))
                .insert(Visibility::Hidden)
                .remove::<Frightened>()
                .remove::<MoveDirection>()
                .remove::<MoveLerp>();
            *stats.kills_by_weapon.entry(active_weapon.0).or_insert(0) += 1;
            score.0 += 200;
            let total_kills: u32 = stats.kills_by_weapon.values().sum();
            info!("enemy_killed: weapon={:?} score={}", active_weapon.0, score.0);
            imetric!("kills", "count", total_kills as u64);
            commands.trigger(EnemyKilled);
        }
    }
}

/// Tick respawn timers; when done, return enemy to pen.
fn enemy_respawn(
    mut commands: Commands,
    time: Res<Time>,
    maze: Option<Res<MazeMap>>,
    mut query: Query<(Entity, &mut Respawning, &SpawnPosition, &mut GridPosition), With<Enemy>>,
) {
    let Some(maze) = maze else { return };
    for (entity, mut timer, spawn, mut pos) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            // Return to spawn position (pen)
            *pos = spawn.0;
            let world = grid_to_world(spawn.0, maze.width, maze.height);
            commands
                .entity(entity)
                .remove::<Respawning>()
                .insert(InPen)
                .insert(Visibility::Inherited)
                .insert(Transform::from_xyz(world.x, world.y, 10.0));
        }
    }
}

// ---------------------------------------------------------------------------
// Frightened AI override
// ---------------------------------------------------------------------------

/// For frightened enemies, override AI to flee from player.
/// This is called from the enemy_ai system in enemies.rs.
pub fn frightened_direction(
    enemy_pos: GridPosition,
    player_pos: GridPosition,
    maze: &MazeMap,
) -> Option<Direction> {
    // Move away from player: pick the neighbor farthest from player
    let neighbors = maze.enemy_neighbors(enemy_pos);
    neighbors
        .into_iter()
        .max_by_key(|n| crate::ai::manhattan(n, &player_pos))
        .and_then(|target| {
            let dx = target.x - enemy_pos.x;
            let dy = target.y - enemy_pos.y;
            match (dx, dy) {
                (1, 0) => Some(Direction::Right),
                (-1, 0) => Some(Direction::Left),
                (0, 1) => Some(Direction::Down),
                (0, -1) => Some(Direction::Up),
                _ => None,
            }
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::app_state::AppState>();
        app.add_sub_state::<PlayingState>();
        app.insert_resource(Score(0));
        app.init_resource::<GameStats>();
        app.add_systems(
            Update,
            (weapon_pickup, player_kills_enemy.after(weapon_pickup))
                .run_if(in_state(PlayingState::Playing)),
        );

        // Transition to Playing
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
    fn weapon_pickup_activates_and_frightens() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };

        let player = app.world_mut().spawn((Player, pos)).id();
        app.world_mut()
            .spawn((WeaponPickup, WeaponType::Bat, pos));
        let enemy = app
            .world_mut()
            .spawn((Enemy, EnemyKind::Soldier, GridPosition { x: 3, y: 3 }))
            .id();

        app.update();

        assert!(app.world().entity(player).get::<ActiveWeapon>().is_some());
        assert!(app.world().entity(enemy).get::<Frightened>().is_some());
    }

    #[test]
    fn player_kills_frightened_enemy() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };

        app.world_mut()
            .spawn((Player, pos, ActiveWeapon(WeaponType::Bat)));
        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyKind::Soldier,
                Frightened,
                pos,
                SpawnPosition(GridPosition { x: 5, y: 5 }),
            ))
            .id();

        app.update();

        // Enemy should be respawning
        assert!(app.world().entity(enemy).get::<Respawning>().is_some());
        assert!(app.world().entity(enemy).get::<Frightened>().is_none());
    }

    #[test]
    fn unarmed_player_doesnt_kill() {
        let mut app = setup_app();
        let pos = GridPosition { x: 1, y: 1 };

        // Player WITHOUT ActiveWeapon
        app.world_mut().spawn((Player, pos));
        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyKind::Soldier,
                Frightened,
                pos,
                SpawnPosition(GridPosition { x: 5, y: 5 }),
            ))
            .id();

        app.update();

        // Enemy should NOT be respawning
        assert!(app.world().entity(enemy).get::<Respawning>().is_none());
    }

    #[test]
    fn frightened_direction_flees_player() {
        let maze = crate::plugins::maze::MazeMap::parse("#####\n#   #\n# P #\n#   #\n#####")
            .unwrap();
        let dir = frightened_direction(
            GridPosition { x: 2, y: 2 }, // enemy at center
            GridPosition { x: 1, y: 2 }, // player to the left
            &maze,
        );
        // Should flee right (away from player)
        assert_eq!(dir, Some(Direction::Right));
    }
}
