# PoC R4: Bevy Headless Testing

**Risk**: R4 (High) — Architecture doc Section 13
**Goal**: Prove that Bevy ECS systems can be unit-tested headlessly — state transitions, event propagation, component queries, and system ordering all behave correctly under `MinimalPlugins` without a window.
**Status**: TODO

---

## 1. Questions to Answer

1. Do `States` transitions work headlessly? How many `app.update()` calls are needed before `OnEnter`/`OnExit` systems run and `State<S>` reflects the new value?
2. Do `SubStates` (e.g., `PlayingState` derived from `AppState::InGame`) activate and deactivate correctly?
3. Do events (`EventWriter` → `EventReader`) propagate within the same frame? How many frames before they're dropped?
4. Can systems be tested in isolation by constructing a minimal `App`, spawning entities, running one update, and asserting on component/resource values?
5. Does system ordering (`before`/`after`, `run_if(in_state(...))`) behave identically in test vs. real app?
6. Do `OnEnter`/`OnExit` schedule systems run at the right time relative to `Update` systems?

---

## 2. Why This Matters

The architecture doc (Section 12) commits to automated tests for every system — maze parsing, grid movement, collectibles, combat, AI, state transitions, narration. All of these assume headless ECS testing works reliably. If state transitions silently fail, or events are dropped, or system ordering differs between test and real app, we'd build a full test suite that gives false confidence.

The existing tests (`tests/telemetry_integration.rs`, `tests/dep_compat.rs`) prove that `MinimalPlugins` + `app.update()` works for basic scenarios. But they don't test states, substates, events, or `OnEnter`/`OnExit` — exactly the patterns the game will rely on heavily.

---

## 3. Test Plan

One test file: `tests/poc_r4_headless_testing.rs`. Each test validates a specific ECS testing pattern the game will need.

### Test 1: State transitions with `NextState`

```
Define: TestState { A (default), B, C }
Init state, verify State<TestState> == A
Set NextState to B
Call app.update() in a loop, assert State<TestState> == B within N frames
Record how many updates were needed (expected: 1-2)
Set NextState to C, repeat
```

**Pass criteria**: State changes propagate within a bounded number of updates. Document the exact count.

### Test 2: `OnEnter` / `OnExit` schedule systems

```
Define: TestState { Setup (default), Playing }
Register OnEnter(Playing) system that inserts a marker resource
Register OnExit(Setup) system that inserts a different marker resource
Transition Setup → Playing
Assert both marker resources exist after sufficient updates
```

**Pass criteria**: `OnEnter`/`OnExit` systems run during state transitions, their side effects (inserted resources) are observable.

### Test 3: `SubStates` activation

```
Define: ParentState { Menu (default), InGame }
Define: ChildState (SubState of InGame) { Running (default), Paused }
Init ParentState, init ChildState
Transition to InGame → verify ChildState exists and == Running
Set ChildState to Paused → verify it propagates
Transition back to Menu → verify ChildState is removed / inaccessible
```

**Pass criteria**: SubStates appear when parent enters the source variant, disappear when parent leaves it. Child transitions work independently of parent.

### Test 4: Event propagation within a frame

```
Register event type TestEvent
Add system A that sends TestEvent
Add system B (ordered after A) that reads TestEvent and sets a resource flag
Run one update
Assert the flag is set
```

**Pass criteria**: Events sent by one system are readable by another system in the same frame (with correct ordering).

### Test 5: Event lifetime across frames

```
Register event type TestEvent
In frame 1: send an event via world access (not a system)
In frame 2: run a system that reads events, count them
In frame 3: run same system again, count them
```

**Pass criteria**: Document exactly how many frames events survive. Bevy docs say 2 update cycles.

### Test 6: Component and resource mutation

```
Define: Health(i32) component, DamageDealt(u32) resource
Spawn 3 entities with Health(10), Health(5), Health(1)
Add system: for each Health > 0, subtract 1, increment DamageDealt
Run one update
Assert: Health values are 9, 4, 0; DamageDealt == 3
Run another update
Assert: Health values are 8, 3, 0; DamageDealt == 5 (only 2 had health > 0)
```

**Pass criteria**: Systems mutate components and resources correctly, observable after each `app.update()`.

### Test 7: `run_if` state gating

```
Define: TestState { Inactive (default), Active }
Add system gated with run_if(in_state(Active)) that increments a counter resource
Run 3 updates in Inactive → counter == 0
Transition to Active, run 3 updates → counter == 3
Transition back to Inactive, run 3 updates → counter still 3
```

**Pass criteria**: `run_if(in_state(...))` correctly gates system execution.

### Test 8: Combined pattern — game-like scenario

```
Model a minimal game loop:
- AppState { Loading (default), Playing }
- OnEnter(Playing): spawn a "player" entity with Position(0) and Speed(1)
- Update system (gated on Playing): Position += Speed
- Event: LevelComplete (sent when Position >= 5)
- System: on LevelComplete event, set NextState back to Loading

Transition Loading → Playing
Run updates until LevelComplete fires
Assert: state returned to Loading, Position reached >= 5
```

**Pass criteria**: The full pattern — states + OnEnter + gated systems + events + state transitions driven by events — works end-to-end headlessly.

---

## 4. Implementation

### File: `tests/poc_r4_headless_testing.rs`

Single file, 8 `#[test]` functions. Each test constructs its own `App` — no shared state between tests.

Required plugins per test:
- `MinimalPlugins` — always
- `StatesPlugin` — for tests 1-3, 7-8 (if not already included in MinimalPlugins)

Check whether `MinimalPlugins` includes `StatesPlugin` in Bevy 0.18. If not, add it explicitly (precedent: `dep_compat.rs` line 34 adds `StatesPlugin` separately).

### Assertions

Use `app.world().resource::<State<T>>()` to read state values.
Use `app.world().resource::<R>()` to read resources.
Use queries via `app.world_mut().run_system_once(...)` or direct world access to read components.

### No new dependencies

All tests use only `bevy` and `std`. No additional dev-dependencies needed.

---

## 5. Pass / Fail Criteria

**PASS** if all 8 tests pass under `cargo test`. Document in this file:
- Exact number of `app.update()` calls needed for state transitions
- Event lifetime in frames
- Any gotchas discovered (e.g., need for `app.finish()` / `app.cleanup()` before first update)

**FAIL** if any core pattern doesn't work headlessly:
- States don't transition
- SubStates don't activate/deactivate
- Events are dropped before they can be read
- `run_if` gating doesn't work
- `OnEnter`/`OnExit` systems don't fire

If FAIL: document what broke and whether workarounds exist. Consider whether Avian2D or other plugins interfere (test with and without).

---

## 6. Files to Modify

| File | Change |
|------|--------|
| `tests/poc_r4_headless_testing.rs` | New — all 8 tests |
| `tasks/poc-r4-headless-testing.md` | Update status and findings |

---

## 7. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| `StatesPlugin` not in `MinimalPlugins` | Low | Add explicitly, precedent in dep_compat.rs |
| `SubStates` require parent state plugin setup | Low | Follow Bevy docs for `add_sub_state` |
| `run_system_once` API changed in 0.18 | Low | Check docs, fall back to direct world queries |
| State transitions need >2 updates | Medium | Document exact count, adjust test helpers |
