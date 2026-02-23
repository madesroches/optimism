use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use optimism::app_state::{AppState, PlayingState};
use optimism::plugins::camera::CameraPlugin;
use optimism::resources::{CurrentLevel, Lives, Score};

// ---------------------------------------------------------------------------
// Helper: run updates until state reaches target or panic
// ---------------------------------------------------------------------------

fn wait_for_state<S: States>(app: &mut App, target: S, max_updates: usize) {
    for i in 0..max_updates {
        app.update();
        if *app.world().resource::<State<S>>().get() == target {
            return;
        }
        assert!(
            i < max_updates - 1,
            "State never reached {:?} after {max_updates} updates",
            target,
        );
    }
}

// ---------------------------------------------------------------------------
// Test 1: State machine transitions
// ---------------------------------------------------------------------------

#[test]
fn state_machine_full_cycle() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.finish();
    app.cleanup();

    // Initial state is Loading.
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Loading,
    );

    // Loading → MainMenu
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    wait_for_state(&mut app, AppState::MainMenu, 5);

    // MainMenu → InGame
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    wait_for_state(&mut app, AppState::InGame, 5);

    // InGame → GameOver
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::GameOver);
    wait_for_state(&mut app, AppState::GameOver, 5);
}

// ---------------------------------------------------------------------------
// Test 2: SubStates activate/deactivate with correct default
// ---------------------------------------------------------------------------

#[test]
fn playing_substates_lifecycle() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.add_sub_state::<PlayingState>();
    app.finish();
    app.cleanup();

    // In Loading — PlayingState should not exist.
    assert!(
        app.world().get_resource::<State<PlayingState>>().is_none(),
        "PlayingState should not exist while in Loading",
    );

    // Transition to InGame.
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..5 {
        app.update();
    }

    // PlayingState should now exist with default LevelIntro.
    let state = app
        .world()
        .get_resource::<State<PlayingState>>()
        .expect("PlayingState should exist when AppState is InGame");
    assert_eq!(*state.get(), PlayingState::LevelIntro);

    // Can transition PlayingState within InGame.
    app.world_mut()
        .resource_mut::<NextState<PlayingState>>()
        .set(PlayingState::Playing);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<PlayingState>>().get(),
        PlayingState::Playing,
    );

    // Leave InGame — PlayingState should disappear.
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..5 {
        app.update();
    }
    assert!(
        app.world().get_resource::<State<PlayingState>>().is_none(),
        "PlayingState should be removed when AppState leaves InGame",
    );
}

// ---------------------------------------------------------------------------
// Test 3: Camera entity spawns on startup
// ---------------------------------------------------------------------------

#[test]
fn camera_entity_spawns() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CameraPlugin);
    app.finish();
    app.cleanup();
    app.update();

    let mut query = app.world_mut().query::<&Camera2d>();
    let count = query.iter(app.world()).count();
    assert_eq!(count, 1, "Expected exactly one Camera2d entity");
}

// ---------------------------------------------------------------------------
// Test 4: Game resources initialized with correct defaults
// ---------------------------------------------------------------------------

#[test]
fn resources_initialized() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(Score(0));
    app.insert_resource(CurrentLevel(1));
    app.insert_resource(Lives(3));
    app.finish();
    app.cleanup();

    assert_eq!(app.world().resource::<Score>().0, 0);
    assert_eq!(app.world().resource::<CurrentLevel>().0, 1);
    assert_eq!(app.world().resource::<Lives>().0, 3);
}
