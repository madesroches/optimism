use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

// ---------------------------------------------------------------------------
// Test 1: State transitions with NextState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum TransitionState {
    #[default]
    A,
    B,
    C,
}

#[test]
fn state_transitions_with_next_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<TransitionState>();
    app.finish();
    app.cleanup();

    // Initial state is A.
    assert_eq!(
        *app.world().resource::<State<TransitionState>>().get(),
        TransitionState::A,
    );

    // Request transition to B.
    app.world_mut()
        .resource_mut::<NextState<TransitionState>>()
        .set(TransitionState::B);

    // Run updates until state changes (bounded).
    let mut updates_to_b = 0;
    for _ in 0..10 {
        app.update();
        updates_to_b += 1;
        if *app.world().resource::<State<TransitionState>>().get() == TransitionState::B {
            break;
        }
    }
    assert_eq!(
        *app.world().resource::<State<TransitionState>>().get(),
        TransitionState::B,
        "State never reached B after {updates_to_b} updates",
    );
    println!("State A->B took {updates_to_b} update(s)");

    // Request transition to C.
    app.world_mut()
        .resource_mut::<NextState<TransitionState>>()
        .set(TransitionState::C);

    let mut updates_to_c = 0;
    for _ in 0..10 {
        app.update();
        updates_to_c += 1;
        if *app.world().resource::<State<TransitionState>>().get() == TransitionState::C {
            break;
        }
    }
    assert_eq!(
        *app.world().resource::<State<TransitionState>>().get(),
        TransitionState::C,
        "State never reached C after {updates_to_c} updates",
    );
    println!("State B->C took {updates_to_c} update(s)");
}

// ---------------------------------------------------------------------------
// Test 2: OnEnter / OnExit schedule systems
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum ScheduleState {
    #[default]
    Setup,
    Playing,
}

#[derive(Resource, Default)]
struct EnteredPlaying(bool);

#[derive(Resource, Default)]
struct ExitedSetup(bool);

fn on_enter_playing(mut marker: ResMut<EnteredPlaying>) {
    marker.0 = true;
}

fn on_exit_setup(mut marker: ResMut<ExitedSetup>) {
    marker.0 = true;
}

#[test]
fn on_enter_on_exit_systems_fire() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<ScheduleState>();
    app.init_resource::<EnteredPlaying>();
    app.init_resource::<ExitedSetup>();
    app.add_systems(OnEnter(ScheduleState::Playing), on_enter_playing);
    app.add_systems(OnExit(ScheduleState::Setup), on_exit_setup);
    app.finish();
    app.cleanup();

    // Transition Setup -> Playing.
    app.world_mut()
        .resource_mut::<NextState<ScheduleState>>()
        .set(ScheduleState::Playing);

    for _ in 0..5 {
        app.update();
    }

    assert!(
        app.world().resource::<EnteredPlaying>().0,
        "OnEnter(Playing) system did not run",
    );
    assert!(
        app.world().resource::<ExitedSetup>().0,
        "OnExit(Setup) system did not run",
    );
}

// ---------------------------------------------------------------------------
// Test 3: SubStates activation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum ParentState {
    #[default]
    Menu,
    InGame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(ParentState = ParentState::InGame)]
enum ChildState {
    #[default]
    Running,
    Paused,
}

#[test]
fn substates_activate_and_deactivate() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<ParentState>();
    app.add_sub_state::<ChildState>();
    app.finish();
    app.cleanup();

    // In Menu — ChildState should not exist.
    assert!(
        app.world().get_resource::<State<ChildState>>().is_none(),
        "ChildState should not exist while in Menu",
    );

    // Transition to InGame.
    app.world_mut()
        .resource_mut::<NextState<ParentState>>()
        .set(ParentState::InGame);
    for _ in 0..5 {
        app.update();
    }

    // ChildState should now exist with default value Running.
    let child = app
        .world()
        .get_resource::<State<ChildState>>()
        .expect("ChildState should exist when parent is InGame");
    assert_eq!(*child.get(), ChildState::Running);

    // Transition child to Paused.
    app.world_mut()
        .resource_mut::<NextState<ChildState>>()
        .set(ChildState::Paused);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<ChildState>>().get(),
        ChildState::Paused,
    );

    // Return parent to Menu — ChildState should disappear.
    app.world_mut()
        .resource_mut::<NextState<ParentState>>()
        .set(ParentState::Menu);
    for _ in 0..5 {
        app.update();
    }
    assert!(
        app.world().get_resource::<State<ChildState>>().is_none(),
        "ChildState should be removed when parent leaves InGame",
    );
}

// ---------------------------------------------------------------------------
// Test 4: Message propagation within a frame
//
// Bevy 0.18 replaced EventReader/EventWriter with the Message system
// (pull-based) and Observers (push-based). Messages are the direct
// replacement for the old event pattern used in game systems.
// ---------------------------------------------------------------------------

#[derive(Message)]
struct TestMessage;

#[derive(Resource, Default)]
struct MessageReceived(bool);

fn send_test_message(mut writer: MessageWriter<TestMessage>) {
    writer.write(TestMessage);
}

fn receive_test_message(mut reader: MessageReader<TestMessage>, mut flag: ResMut<MessageReceived>) {
    for _ in reader.read() {
        flag.0 = true;
    }
}

#[test]
fn messages_propagate_within_same_frame() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<TestMessage>();
    app.init_resource::<MessageReceived>();
    app.add_systems(Update, send_test_message.before(receive_test_message));
    app.add_systems(Update, receive_test_message);
    app.finish();
    app.cleanup();

    app.update();

    assert!(
        app.world().resource::<MessageReceived>().0,
        "Message was not received in the same frame it was sent",
    );
}

// ---------------------------------------------------------------------------
// Test 5: Message lifetime across frames
//
// Messages use double-buffering like the old Events. Test how many
// frames a message survives.
// ---------------------------------------------------------------------------

#[derive(Message)]
struct LifetimeMessage;

#[derive(Resource, Default)]
struct MsgCount(u32);

fn count_lifetime_messages(
    mut reader: MessageReader<LifetimeMessage>,
    mut count: ResMut<MsgCount>,
) {
    count.0 = reader.read().count() as u32;
}

#[test]
fn message_lifetime_across_frames() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<LifetimeMessage>();
    app.init_resource::<MsgCount>();
    app.add_systems(Update, count_lifetime_messages);
    app.finish();
    app.cleanup();

    // Send message via direct world access.
    app.world_mut().write_message(LifetimeMessage);

    // Frame 1: message should be visible.
    app.update();
    let count_frame1 = app.world().resource::<MsgCount>().0;
    println!("Frame 1 message count: {count_frame1}");

    // Frame 2: double-buffered, may still be visible.
    app.update();
    let count_frame2 = app.world().resource::<MsgCount>().0;
    println!("Frame 2 message count: {count_frame2}");

    // Frame 3: should be gone.
    app.update();
    let count_frame3 = app.world().resource::<MsgCount>().0;
    println!("Frame 3 message count: {count_frame3}");

    assert!(count_frame1 > 0, "Message not visible in frame 1");
    assert_eq!(count_frame3, 0, "Message still alive in frame 3");
}

// ---------------------------------------------------------------------------
// Test 6: Component and resource mutation
// ---------------------------------------------------------------------------

#[derive(Component)]
struct Health(i32);

#[derive(Resource, Default)]
struct DamageDealt(u32);

fn damage_system(mut query: Query<&mut Health>, mut damage: ResMut<DamageDealt>) {
    for mut hp in &mut query {
        if hp.0 > 0 {
            hp.0 -= 1;
            damage.0 += 1;
        }
    }
}

#[test]
fn component_and_resource_mutation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<DamageDealt>();
    app.add_systems(Update, damage_system);
    app.finish();
    app.cleanup();

    // Spawn 3 entities with different health values.
    let e1 = app.world_mut().spawn(Health(10)).id();
    let e2 = app.world_mut().spawn(Health(5)).id();
    let e3 = app.world_mut().spawn(Health(1)).id();

    // Update 1: all 3 take damage.
    app.update();
    assert_eq!(app.world().entity(e1).get::<Health>().unwrap().0, 9);
    assert_eq!(app.world().entity(e2).get::<Health>().unwrap().0, 4);
    assert_eq!(app.world().entity(e3).get::<Health>().unwrap().0, 0);
    assert_eq!(app.world().resource::<DamageDealt>().0, 3);

    // Update 2: only 2 entities have health > 0.
    app.update();
    assert_eq!(app.world().entity(e1).get::<Health>().unwrap().0, 8);
    assert_eq!(app.world().entity(e2).get::<Health>().unwrap().0, 3);
    assert_eq!(app.world().entity(e3).get::<Health>().unwrap().0, 0);
    assert_eq!(app.world().resource::<DamageDealt>().0, 5);
}

// ---------------------------------------------------------------------------
// Test 7: run_if state gating
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum GateState {
    #[default]
    Inactive,
    Active,
}

#[derive(Resource, Default)]
struct TickCounter(u32);

fn increment_counter(mut counter: ResMut<TickCounter>) {
    counter.0 += 1;
}

#[test]
fn run_if_state_gating() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<GateState>();
    app.init_resource::<TickCounter>();
    app.add_systems(
        Update,
        increment_counter.run_if(in_state(GateState::Active)),
    );
    app.finish();
    app.cleanup();

    // 3 updates in Inactive — counter should stay 0.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<TickCounter>().0, 0);

    // Transition to Active.
    app.world_mut()
        .resource_mut::<NextState<GateState>>()
        .set(GateState::Active);

    // The transition update itself may or may not run the gated system.
    // Run 1 transition update + 3 gated updates.
    for _ in 0..4 {
        app.update();
    }
    let count_after_active = app.world().resource::<TickCounter>().0;
    println!("Counter after 4 updates in Active phase: {count_after_active}");
    assert!(
        count_after_active >= 3,
        "Expected counter >= 3, got {count_after_active}",
    );

    let count_before_inactive = count_after_active;

    // Transition back to Inactive.
    app.world_mut()
        .resource_mut::<NextState<GateState>>()
        .set(GateState::Inactive);

    for _ in 0..4 {
        app.update();
    }
    let count_after_inactive = app.world().resource::<TickCounter>().0;
    println!("Counter after returning to Inactive: {count_after_inactive}");
    assert_eq!(
        count_after_inactive, count_before_inactive,
        "Counter incremented while in Inactive state",
    );
}

// ---------------------------------------------------------------------------
// Test 8: Combined game-like scenario
//
// Uses Messages (pull-based) for inter-system communication within
// the Update loop, plus states + OnEnter + run_if gating.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
enum GameState {
    #[default]
    Loading,
    Playing,
}

#[derive(Component)]
struct Position(i32);

#[derive(Component)]
struct Speed(i32);

#[derive(Message)]
struct LevelComplete;

#[derive(Resource, Default)]
struct LevelCompleted(bool);

fn spawn_player(mut commands: Commands) {
    commands.spawn((Position(0), Speed(1)));
}

fn move_player(mut query: Query<(&mut Position, &Speed)>) {
    for (mut pos, spd) in &mut query {
        pos.0 += spd.0;
    }
}

fn check_level_complete(query: Query<&Position>, mut writer: MessageWriter<LevelComplete>) {
    for pos in &query {
        if pos.0 >= 5 {
            writer.write(LevelComplete);
            return;
        }
    }
}

fn handle_level_complete(
    mut reader: MessageReader<LevelComplete>,
    mut next: ResMut<NextState<GameState>>,
    mut completed: ResMut<LevelCompleted>,
) {
    for _ in reader.read() {
        next.set(GameState::Loading);
        completed.0 = true;
    }
}

#[test]
fn combined_game_like_scenario() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<GameState>();
    app.add_message::<LevelComplete>();
    app.init_resource::<LevelCompleted>();

    app.add_systems(OnEnter(GameState::Playing), spawn_player);
    app.add_systems(
        Update,
        (
            move_player,
            check_level_complete.after(move_player),
            handle_level_complete.after(check_level_complete),
        )
            .run_if(in_state(GameState::Playing)),
    );
    app.finish();
    app.cleanup();

    // Transition Loading -> Playing.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);

    // Run until level completes or we hit a safety limit.
    let mut frames = 0;
    let max_frames = 20;
    loop {
        app.update();
        frames += 1;
        if app.world().resource::<LevelCompleted>().0 {
            break;
        }
        assert!(
            frames < max_frames,
            "Level never completed after {max_frames} frames"
        );
    }

    println!("Game-like scenario completed in {frames} frame(s)");

    // Allow the state transition back to Loading to propagate.
    for _ in 0..3 {
        app.update();
    }

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Loading,
        "State did not return to Loading after LevelComplete",
    );
}
