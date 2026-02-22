use avian2d::prelude::PhysicsPlugins;
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy_asset_loader::loading_state::LoadingStateAppExt;
use bevy_asset_loader::prelude::LoadingState;

// Import to prove bevy_kira_audio compiles alongside everything else.
// AudioPlugin can't run headless — kira's ALSA backend panics without
// an audio device (expected in WSL2/CI).
#[allow(unused_imports)]
use bevy_kira_audio::AudioPlugin;

/// All headless-safe plugins coexist in a single headless Bevy app.
/// bevy_kira_audio is excluded at runtime (needs ALSA device) but verified
/// at compile time via the import above.
#[test]
fn dep_compat_all_plugins_coexist() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
    enum TestGameState {
        #[default]
        Loading,
        Running,
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // Plugins required by avian2d but not in MinimalPlugins
    app.add_plugins(TransformPlugin);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ScenePlugin);
    // bevy_asset_loader
    app.add_plugins(StatesPlugin);
    // Game plugins
    app.add_plugins(PhysicsPlugins::default());
    app.init_state::<TestGameState>();
    app.add_loading_state(
        LoadingState::new(TestGameState::Loading)
            .continue_to_state(TestGameState::Running),
    );
    // finish() must be called before update() — Plugin::finish() is where
    // avian2d registers its diagnostics resources.
    app.finish();
    app.cleanup();
    for _ in 0..3 {
        app.update();
    }
}
