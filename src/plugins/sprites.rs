//! Sprite sheet loading and animation for characters rendered from 3D models.
//!
//! Each character has a PNG sprite sheet and a JSON metadata sidecar describing
//! animation frame ranges. This plugin loads them, builds texture atlas layouts,
//! and provides an animation system that ticks through frames.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;
use std::collections::HashMap;

/// Plugin that registers sprite loading and animation systems.
pub struct SpriteSheetPlugin;

impl Plugin for SpriteSheetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteSheetLibrary>()
            .add_systems(Update, animate_sprites);
    }
}

// ---------------------------------------------------------------------------
// JSON metadata (mirrors tools/render_sprites.py output)
// ---------------------------------------------------------------------------

/// Deserialized from the JSON sidecar next to each sprite sheet PNG.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SpriteSheetMeta {
    pub frame_size: [u32; 2],
    pub columns: u32,
    pub rows: u32,
    pub animations: HashMap<String, AnimationRange>,
}

/// A contiguous range of frames in the sprite sheet.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct AnimationRange {
    pub start: usize,
    pub count: usize,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Stores loaded sprite sheet data for each character, keyed by name.
#[derive(Resource, Default)]
pub struct SpriteSheetLibrary {
    pub sheets: HashMap<String, CharacterSheet>,
}

/// All data needed to spawn and animate a character's sprites.
#[derive(Debug, Clone)]
pub struct CharacterSheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub meta: SpriteSheetMeta,
}

impl SpriteSheetLibrary {
    /// Load a character sprite sheet from a PNG path.
    ///
    /// Expects a JSON sidecar at the same path with `.json` extension.
    /// Call this during setup after the asset server is available.
    pub fn load(
        &mut self,
        name: &str,
        png_path: &str,
        asset_server: &AssetServer,
        layouts: &mut Assets<TextureAtlasLayout>,
    ) -> Result<(), String> {
        let image: Handle<Image> = asset_server.load(png_path.to_string());

        // Read JSON metadata from filesystem (assets/ directory)
        let assets_dir = std::path::Path::new("assets");
        let json_path = assets_dir.join(png_path).with_extension("json");
        let json_str = std::fs::read_to_string(&json_path)
            .map_err(|e| format!("Failed to read {}: {}", json_path.display(), e))?;
        let meta: SpriteSheetMeta = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse {}: {}", json_path.display(), e))?;

        let layout = TextureAtlasLayout::from_grid(
            UVec2::new(meta.frame_size[0], meta.frame_size[1]),
            meta.columns,
            meta.rows,
            None,
            None,
        );
        let layout_handle = layouts.add(layout);

        self.sheets.insert(
            name.to_string(),
            CharacterSheet {
                image,
                layout: layout_handle,
                meta,
            },
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Current animation state for a sprite entity.
#[derive(Component)]
pub struct AnimationState {
    /// Key into `SpriteSheetMeta.animations` (e.g., "walk_down", "idle", "death").
    pub current: String,
    /// Whether the animation should loop.
    pub looping: bool,
    /// Set to true when a non-looping animation has played through.
    pub finished: bool,
}

impl AnimationState {
    pub fn new(animation: &str, looping: bool) -> Self {
        Self {
            current: animation.to_string(),
            looping,
            finished: false,
        }
    }
}

/// Timer that controls animation frame rate.
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

/// Which character sheet this entity uses (key into `SpriteSheetLibrary`).
#[derive(Component)]
pub struct CharacterSheetRef(pub String);

/// Direction the character is facing. Used to select directional animation variants.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FacingDirection {
    #[default]
    Down,
    Left,
    Up,
    Right,
}

impl FacingDirection {
    /// Suffix appended to animation base name for directional lookups.
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::Down => "_down",
            Self::Left => "_left",
            Self::Up => "_up",
            Self::Right => "_right",
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Advances sprite animation frames based on timer and current state.
#[span_fn]
fn animate_sprites(
    time: Res<Time>,
    library: Res<SpriteSheetLibrary>,
    mut query: Query<(
        &CharacterSheetRef,
        &mut AnimationState,
        &mut AnimationTimer,
        &mut Sprite,
    )>,
) {
    for (sheet_ref, mut anim_state, mut timer, mut sprite) in &mut query {
        if anim_state.finished {
            continue;
        }

        timer.tick(time.delta());
        if !timer.just_finished() {
            continue;
        }

        let Some(sheet) = library.sheets.get(&sheet_ref.0) else {
            continue;
        };

        let Some(range) = sheet.meta.animations.get(&anim_state.current) else {
            continue;
        };

        let Some(atlas) = &mut sprite.texture_atlas else {
            continue;
        };

        // Advance frame
        let current_offset = atlas.index.saturating_sub(range.start);
        let next_offset = current_offset + 1;

        if next_offset >= range.count {
            if anim_state.looping {
                atlas.index = range.start;
            } else {
                anim_state.finished = true;
            }
        } else {
            atlas.index = range.start + next_offset;
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: resolve animation key with direction fallback
// ---------------------------------------------------------------------------

/// Given a base animation name (e.g., "walk") and a facing direction,
/// returns the full key (e.g., "walk_down"). Falls back to the base name
/// if the directional variant doesn't exist in the metadata.
pub fn resolve_animation_key(
    base: &str,
    direction: FacingDirection,
    meta: &SpriteSheetMeta,
) -> String {
    let directional = format!("{}{}", base, direction.suffix());
    if meta.animations.contains_key(&directional) {
        directional
    } else {
        base.to_string()
    }
}

/// Convenience: switch an entity's animation, resetting the frame to the start.
pub fn set_animation(
    sprite: &mut Sprite,
    anim_state: &mut AnimationState,
    key: &str,
    looping: bool,
    meta: &SpriteSheetMeta,
) {
    if anim_state.current == key && !anim_state.finished {
        return; // Already playing this animation
    }
    anim_state.current = key.to_string();
    anim_state.looping = looping;
    anim_state.finished = false;

    if let Some(range) = meta.animations.get(key)
        && let Some(atlas) = &mut sprite.texture_atlas
    {
        atlas.index = range.start;
    }
}
