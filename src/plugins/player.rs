//! Player spawning and input handling.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;

use crate::app_state::PlayingState;
use crate::components::*;
use crate::plugins::maze::{grid_to_world, load_maze, MazeMap, TILE_SIZE};
use crate::plugins::telemetry::GameSet;
use crate::plugins::sprites::{
    AnimationState, AnimationTimer, CharacterSheetRef, FacingDirection, SpriteSheetLibrary,
    resolve_animation_key, set_animation,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(PlayingState::LevelIntro),
            spawn_player.after(load_maze),
        );
        app.add_systems(
            Update,
            (player_input, apply_player_direction, sync_facing_to_animation)
                .in_set(GameSet::Player)
                .run_if(in_state(PlayingState::Playing)),
        );
    }
}

/// Spawn the player entity at the maze's player spawn position.
#[span_fn]
pub fn spawn_player(
    mut commands: Commands,
    maze: Res<MazeMap>,
    mut library: ResMut<SpriteSheetLibrary>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Load candide sprite sheet if not already loaded
    if !library.sheets.contains_key("candide_base") {
        let _ = library.load(
            "candide_base",
            "sprites/candide_base.png",
            &asset_server,
            &mut layouts,
        );
    }

    let pos = maze.player_spawn;
    let world_pos = grid_to_world(pos, maze.width, maze.height);

    let sheet = library.sheets.get("candide_base");

    let mut entity_commands = commands.spawn((
        Player,
        pos,
        SpawnPosition(pos),
        MoveSpeed(5.0),
        FacingDirection::Down,
        InputDirection::default(),
        crate::plugins::maze::MazeEntity,
        Transform::from_xyz(world_pos.x, world_pos.y, 10.0),
    ));

    // Add sprite components if sheet is available
    if let Some(sheet) = sheet {
        let anim_key = resolve_animation_key("walk", FacingDirection::Down, &sheet.meta);
        let start_index = sheet
            .meta
            .animations
            .get(&anim_key)
            .map(|r| r.start)
            .unwrap_or(0);

        entity_commands.insert((
            Sprite {
                image: sheet.image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: sheet.layout.clone(),
                    index: start_index,
                }),
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            CharacterSheetRef("candide_base".to_string()),
            AnimationState::new(&anim_key, true),
            AnimationTimer(Timer::from_seconds(0.15, TimerMode::Repeating)),
        ));
    }
}

/// Read keyboard input and buffer the direction.
#[span_fn]
fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut InputDirection, With<Player>>,
) {
    for mut input in &mut query {
        if keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW) {
            input.0 = Some(Direction::Up);
        } else if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
            input.0 = Some(Direction::Down);
        } else if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
            input.0 = Some(Direction::Left);
        } else if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
            input.0 = Some(Direction::Right);
        }
    }
}

/// When the player arrives at a tile (no active lerp, no MoveDirection),
/// apply the buffered input direction.
#[allow(clippy::type_complexity)]
#[span_fn]
fn apply_player_direction(
    mut commands: Commands,
    mut query: Query<
        (Entity, &InputDirection, &mut FacingDirection),
        (With<Player>, Without<MoveLerp>, Without<MoveDirection>),
    >,
) {
    for (entity, input, mut facing) in &mut query {
        if let Some(dir) = input.0 {
            commands.entity(entity).insert(MoveDirection(dir));
            *facing = dir.into();
        }
    }
}

/// Bridge between FacingDirection changes and the sprite animation system.
#[span_fn]
fn sync_facing_to_animation(
    library: Res<SpriteSheetLibrary>,
    mut query: Query<
        (
            &CharacterSheetRef,
            &FacingDirection,
            &mut AnimationState,
            &mut Sprite,
        ),
        Changed<FacingDirection>,
    >,
) {
    for (sheet_ref, facing, mut anim_state, mut sprite) in &mut query {
        let Some(sheet) = library.sheets.get(&sheet_ref.0) else {
            continue;
        };
        let key = resolve_animation_key("walk", *facing, &sheet.meta);
        set_animation(&mut sprite, &mut anim_state, &key, true, &sheet.meta);
    }
}

/// Convert Direction to FacingDirection.
impl From<Direction> for FacingDirection {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::Up => FacingDirection::Up,
            Direction::Down => FacingDirection::Down,
            Direction::Left => FacingDirection::Left,
            Direction::Right => FacingDirection::Right,
        }
    }
}
