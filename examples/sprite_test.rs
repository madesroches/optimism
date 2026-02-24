//! Minimal test window for validating sprite sheets.
//!
//! Usage:
//!     cargo run --example sprite_test -- [character_name]
//!
//! Controls:
//!     Arrow keys — move (switches walk direction)
//!     Space      — attack animation
//!     D          — death animation
//!     I          — idle animation
//!     1-5        — switch character (if multiple loaded)

use bevy::prelude::*;
use optimism::plugins::sprites::*;

const MOVE_SPEED: f32 = 150.0;
const ANIMATION_FPS: f32 = 8.0;
const SPRITE_SCALE: f32 = 4.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SpriteSheetPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_animation_from_movement))
        .run();
}

/// Marker component for the player-controlled test character.
#[derive(Component)]
struct TestCharacter;

/// Tracks current movement intent for animation selection.
#[derive(Component, Default)]
struct Movement {
    velocity: Vec2,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut library: ResMut<SpriteSheetLibrary>,
) {
    commands.spawn(Camera2d);

    // Determine which character to load from CLI args, default to "candide_base"
    let char_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "candide_base".to_string());
    let png_path = format!("sprites/{}.png", char_name);

    match library.load(&char_name, &png_path, &asset_server, &mut layouts) {
        Ok(()) => info!("Loaded sprite sheet for '{}'", char_name),
        Err(e) => {
            error!("Failed to load sprite sheet for '{}': {}", char_name, e);
            error!("Make sure you've rendered sprites first: python3 tools/render_all.py");
            return;
        }
    }

    let sheet = library.sheets.get(&char_name).unwrap();

    // Determine initial animation
    let initial_anim = if sheet.meta.animations.contains_key("idle") {
        "idle"
    } else if sheet.meta.animations.contains_key("walk_down") {
        "walk_down"
    } else {
        sheet
            .meta
            .animations
            .keys()
            .next()
            .map(|s| s.as_str())
            .unwrap_or("idle")
    };

    let initial_index = sheet
        .meta
        .animations
        .get(initial_anim)
        .map(|r| r.start)
        .unwrap_or(0);

    commands.spawn((
        TestCharacter,
        Movement::default(),
        Sprite::from_atlas_image(
            sheet.image.clone(),
            TextureAtlas {
                layout: sheet.layout.clone(),
                index: initial_index,
            },
        ),
        Transform::from_scale(Vec3::splat(SPRITE_SCALE)),
        CharacterSheetRef(char_name.clone()),
        AnimationState::new(initial_anim, true),
        AnimationTimer(Timer::from_seconds(
            1.0 / ANIMATION_FPS,
            TimerMode::Repeating,
        )),
        FacingDirection::Down,
    ));

    // HUD text
    commands.spawn((
        Text::new(format!(
            "Character: {}\nArrows: move | Space: attack | D: death | I: idle",
            char_name
        )),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    library: Res<SpriteSheetLibrary>,
    mut query: Query<
        (
            &CharacterSheetRef,
            &mut Movement,
            &mut FacingDirection,
            &mut AnimationState,
            &mut Sprite,
        ),
        With<TestCharacter>,
    >,
) {
    for (sheet_ref, mut movement, mut facing, mut anim_state, mut sprite) in &mut query {
        let Some(sheet) = library.sheets.get(&sheet_ref.0) else {
            continue;
        };

        // Movement input
        let mut dir = Vec2::ZERO;
        if keys.pressed(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if keys.pressed(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        movement.velocity = if dir != Vec2::ZERO {
            dir.normalize() * MOVE_SPEED
        } else {
            Vec2::ZERO
        };

        // Update facing from movement
        if dir.x < 0.0 {
            *facing = FacingDirection::Left;
        } else if dir.x > 0.0 {
            *facing = FacingDirection::Right;
        } else if dir.y > 0.0 {
            *facing = FacingDirection::Up;
        } else if dir.y < 0.0 {
            *facing = FacingDirection::Down;
        }

        // Action keys (override movement animations)
        if keys.just_pressed(KeyCode::Space) {
            let key = resolve_animation_key("attack", *facing, &sheet.meta);
            set_animation(&mut sprite, &mut anim_state, &key, false, &sheet.meta);
        } else if keys.just_pressed(KeyCode::KeyD) {
            set_animation(&mut sprite, &mut anim_state, "death", false, &sheet.meta);
        } else if keys.just_pressed(KeyCode::KeyI) {
            set_animation(&mut sprite, &mut anim_state, "idle", true, &sheet.meta);
        }
    }
}

fn update_animation_from_movement(
    time: Res<Time>,
    library: Res<SpriteSheetLibrary>,
    mut query: Query<
        (
            &CharacterSheetRef,
            &Movement,
            &FacingDirection,
            &mut AnimationState,
            &mut Sprite,
            &mut Transform,
        ),
        With<TestCharacter>,
    >,
) {
    for (sheet_ref, movement, facing, mut anim_state, mut sprite, mut transform) in &mut query {
        // Apply movement to transform
        let delta = movement.velocity * time.delta_secs();
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;

        // Don't override action animations that are still playing
        if !anim_state.looping && !anim_state.finished {
            return;
        }

        let Some(sheet) = library.sheets.get(&sheet_ref.0) else {
            continue;
        };

        // Switch between walk and idle based on movement
        if movement.velocity.length_squared() > 0.01 {
            let key = resolve_animation_key("walk", *facing, &sheet.meta);
            set_animation(&mut sprite, &mut anim_state, &key, true, &sheet.meta);
        } else {
            // Only switch to idle if we were walking
            if anim_state.current.starts_with("walk") {
                set_animation(&mut sprite, &mut anim_state, "idle", true, &sheet.meta);
            }
        }
    }
}
