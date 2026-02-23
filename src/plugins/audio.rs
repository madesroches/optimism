//! Audio: music loops per state, SFX via observers.

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use micromegas_tracing::prelude::*;

use crate::app_state::{AppState, PlayingState};
use crate::events::{EnemyKilled, LuxuryCollected, MoneyCollected, WeaponPickedUp};
use crate::resources::AudioAssets;

#[derive(Resource)]
pub struct MusicChannel;

#[derive(Resource)]
pub struct SfxChannel;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_channel::<MusicChannel>()
            .add_audio_channel::<SfxChannel>();

        // Music state transitions
        app.add_systems(OnEnter(AppState::MainMenu), start_menu_music);
        app.add_systems(OnExit(AppState::MainMenu), stop_menu_music);
        app.add_systems(OnEnter(AppState::InGame), start_gameplay_music);
        app.add_systems(OnExit(AppState::InGame), stop_gameplay_music);

        // SFX via observers
        app.add_observer(on_money_collected);
        app.add_observer(on_weapon_picked_up);
        app.add_observer(on_enemy_killed);
        app.add_observer(on_luxury_collected);

        // SFX via state hooks
        app.add_systems(OnEnter(PlayingState::PlayerDeath), play_death_sfx);
        app.add_systems(OnEnter(PlayingState::LevelComplete), play_level_complete_sfx);
    }
}

// ---------------------------------------------------------------------------
// Music
// ---------------------------------------------------------------------------

#[span_fn]
fn start_menu_music(music: Res<AudioChannel<MusicChannel>>, assets: Res<AudioAssets>) {
    music.play(assets.menu_theme.clone()).looped();
}

#[span_fn]
fn stop_menu_music(music: Res<AudioChannel<MusicChannel>>) {
    music.stop();
}

#[span_fn]
fn start_gameplay_music(music: Res<AudioChannel<MusicChannel>>, assets: Res<AudioAssets>) {
    music.play(assets.gameplay.clone()).looped();
}

#[span_fn]
fn stop_gameplay_music(music: Res<AudioChannel<MusicChannel>>) {
    music.stop();
}

// ---------------------------------------------------------------------------
// SFX observers
// ---------------------------------------------------------------------------

#[span_fn]
fn on_money_collected(
    _trigger: On<MoneyCollected>,
    sfx: Res<AudioChannel<SfxChannel>>,
    assets: Res<AudioAssets>,
) {
    sfx.play(assets.dot_pickup.clone());
}

#[span_fn]
fn on_weapon_picked_up(
    _trigger: On<WeaponPickedUp>,
    sfx: Res<AudioChannel<SfxChannel>>,
    assets: Res<AudioAssets>,
) {
    sfx.play(assets.power_pellet.clone());
}

#[span_fn]
fn on_enemy_killed(
    _trigger: On<EnemyKilled>,
    sfx: Res<AudioChannel<SfxChannel>>,
    assets: Res<AudioAssets>,
) {
    sfx.play(assets.ghost_eaten.clone());
}

#[span_fn]
fn on_luxury_collected(
    _trigger: On<LuxuryCollected>,
    sfx: Res<AudioChannel<SfxChannel>>,
    assets: Res<AudioAssets>,
) {
    sfx.play(assets.power_pellet.clone());
}

// ---------------------------------------------------------------------------
// SFX state hooks
// ---------------------------------------------------------------------------

#[span_fn]
fn play_death_sfx(sfx: Res<AudioChannel<SfxChannel>>, assets: Res<AudioAssets>) {
    sfx.play(assets.death.clone());
}

#[span_fn]
fn play_level_complete_sfx(sfx: Res<AudioChannel<SfxChannel>>, assets: Res<AudioAssets>) {
    sfx.play(assets.level_complete.clone());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use bevy::state::app::StatesPlugin;
    use bevy_kira_audio::AudioPlugin;

    #[test]
    fn audio_plugin_initializes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(StatesPlugin);
        app.add_plugins(AudioPlugin);
        app.init_state::<AppState>();
        app.add_sub_state::<PlayingState>();
        app.add_plugins(GameAudioPlugin);

        app.update();

        // Channel resources should exist
        assert!(app.world().get_resource::<AudioChannel<MusicChannel>>().is_some());
        assert!(app.world().get_resource::<AudioChannel<SfxChannel>>().is_some());
    }
}
