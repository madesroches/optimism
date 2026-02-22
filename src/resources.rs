use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioSource;

#[derive(Resource, Debug)]
pub struct Score(pub u64);

#[derive(Resource, Debug)]
pub struct CurrentLevel(pub u32);

#[derive(Resource, Debug)]
pub struct Lives(pub u32);

#[derive(Resource, Debug, Default)]
pub struct LevelConfig;

#[derive(AssetCollection, Resource)]
pub struct AudioAssets {
    #[asset(path = "audio/music/menu_theme.ogg")]
    pub menu_theme: Handle<AudioSource>,
    #[asset(path = "audio/music/gameplay.ogg")]
    pub gameplay: Handle<AudioSource>,
    #[asset(path = "audio/sfx/dot_pickup.ogg")]
    pub dot_pickup: Handle<AudioSource>,
    #[asset(path = "audio/sfx/power_pellet.ogg")]
    pub power_pellet: Handle<AudioSource>,
    #[asset(path = "audio/sfx/ghost_eaten.ogg")]
    pub ghost_eaten: Handle<AudioSource>,
    #[asset(path = "audio/sfx/death.ogg")]
    pub death: Handle<AudioSource>,
    #[asset(path = "audio/sfx/level_complete.ogg")]
    pub level_complete: Handle<AudioSource>,
}
