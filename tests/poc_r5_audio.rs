use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

// ---------------------------------------------------------------------------
// Test 1: AudioPlugin initializes under MinimalPlugins (headless)
// ---------------------------------------------------------------------------

#[test]
fn audio_plugin_initializes_headless() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(AudioPlugin);
    app.finish();
    app.cleanup();

    // Run a few updates — should not panic even without an audio device.
    for _ in 0..3 {
        app.update();
    }

    // Verify the Audio resource (AudioChannel<MainTrack>) was registered.
    assert!(
        app.world().get_resource::<AudioChannel<MainTrack>>().is_some(),
        "Audio resource should exist after AudioPlugin initialization"
    );
}

// ---------------------------------------------------------------------------
// Test 2: OGG audio asset handles can be created
// ---------------------------------------------------------------------------

#[test]
fn ogg_asset_handle_creation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(AudioPlugin);
    app.finish();
    app.cleanup();

    // Load audio files via AssetServer — the handles should be created
    // even if the audio backend isn't available.
    let asset_server = app.world().resource::<AssetServer>();

    let music_handle: Handle<AudioSource> = asset_server.load("audio/music/menu_theme.ogg");
    let sfx_handle: Handle<AudioSource> = asset_server.load("audio/sfx/dot_pickup.ogg");

    // Handles should be strong (AssetServer::load returns strong handles).
    assert!(music_handle.is_strong(), "Music handle should be strong");
    assert!(sfx_handle.is_strong(), "SFX handle should be strong");

    // Run a few updates to let the asset system process.
    for _ in 0..5 {
        app.update();
    }
}
