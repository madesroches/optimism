//! Narration: Candide-themed quotes triggered by gameplay events.

use bevy::prelude::*;
use micromegas_tracing::prelude::*;

use crate::app_state::{AppState, PlayingState};
use crate::events::{EnemyKilled, MoneyCollected, WeaponPickedUp};
use crate::resources::CurrentLevel;

pub struct NarrationPlugin;

impl Plugin for NarrationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_money_collected);
        app.add_observer(on_weapon_picked_up);
        app.add_observer(on_enemy_killed);

        app.add_systems(OnEnter(PlayingState::Playing), on_level_start);
        app.add_systems(OnEnter(PlayingState::PlayerDeath), on_player_death);

        app.add_systems(
            Update,
            fade_narration.run_if(in_state(AppState::InGame)),
        );
    }
}

// ---------------------------------------------------------------------------
// Quote pools
// ---------------------------------------------------------------------------

const MONEY_QUOTES: &[&str] = &[
    "Gold is the best of all things.",
    "Riches do not bring happiness, but they help.",
    "A coin! Pangloss would approve.",
    "Money makes the world less miserable.",
    "Wealth is but a means to cultivate one's garden.",
];

const WEAPON_QUOTES: &[&str] = &[
    "In this best of all possible worlds, one must arm oneself.",
    "Power, at last! But for how long?",
    "The sword of optimism cuts both ways.",
    "Armed with hope and steel alike.",
    "Even Candide must sometimes fight.",
];

const DEATH_QUOTES: &[&str] = &[
    "All is for the best, even this setback.",
    "Misfortune is but a stepping stone.",
    "To die is nothing; but to live defeated is to die daily.",
    "Pangloss would say this was necessary.",
    "The worst is never certain... usually.",
];

const KILL_QUOTES: &[&str] = &[
    "A necessary evil in this best of worlds.",
    "Justice, swift and terrible!",
    "One less obstacle to cultivating one's garden.",
    "Victory! But at what philosophical cost?",
    "Optimism prevails through force.",
];

const LEVEL_START_QUOTES: &[&str] = &[
    "A new chapter in this best of all possible worlds.",
    "Onward! There are gardens yet to cultivate.",
    "Each room holds new wonders and terrors.",
    "Pangloss would marvel at such adventure.",
    "Let us see what fortune has in store.",
];

// ---------------------------------------------------------------------------
// Resources and components
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct NarrationState {
    pub last_quote: Option<usize>,
    pub money_counter: u32,
}

#[derive(Component)]
pub struct NarrationText;

#[derive(Component, Deref, DerefMut)]
pub struct NarrationFadeTimer(pub Timer);

/// Garden level (level 13) — all narration suppressed.
const GARDEN_LEVEL: u32 = 13;

// ---------------------------------------------------------------------------
// Quote picker
// ---------------------------------------------------------------------------

pub fn pick_quote<'a>(pool: &'a [&'a str], state: &mut NarrationState) -> &'a str {
    if pool.len() <= 1 {
        state.last_quote = Some(0);
        return pool.first().copied().unwrap_or("");
    }

    // Simple deterministic pick: cycle sequentially, guarantees no consecutive duplicates
    let idx = state.last_quote.map(|i| (i + 1) % pool.len()).unwrap_or(0);
    state.last_quote = Some(idx);
    pool[idx]
}

// ---------------------------------------------------------------------------
// Spawn / replace narration
// ---------------------------------------------------------------------------

fn show_narration(commands: &mut Commands, text: &str) {
    commands.spawn((
        NarrationText,
        NarrationFadeTimer(Timer::from_seconds(4.0, TimerMode::Once)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(40.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Text::new(text.to_string()),
        TextColor(Color::srgba(1.0, 1.0, 0.8, 1.0)),
        TextFont {
            font_size: 20.0,
            ..default()
        },
    ));
}

fn despawn_old_narration(commands: &mut Commands, query: &Query<Entity, With<NarrationText>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn is_garden_level(level: &CurrentLevel) -> bool {
    level.0 >= GARDEN_LEVEL
}

// ---------------------------------------------------------------------------
// Observer-driven triggers
// ---------------------------------------------------------------------------

#[span_fn]
fn on_money_collected(
    _trigger: On<MoneyCollected>,
    mut commands: Commands,
    mut state: ResMut<NarrationState>,
    level: Res<CurrentLevel>,
    existing: Query<Entity, With<NarrationText>>,
) {
    if is_garden_level(&level) {
        return;
    }
    state.money_counter += 1;
    if !state.money_counter.is_multiple_of(5) {
        return;
    }
    despawn_old_narration(&mut commands, &existing);
    let quote = pick_quote(MONEY_QUOTES, &mut state);
    show_narration(&mut commands, quote);
}

#[span_fn]
fn on_weapon_picked_up(
    _trigger: On<WeaponPickedUp>,
    mut commands: Commands,
    mut state: ResMut<NarrationState>,
    level: Res<CurrentLevel>,
    existing: Query<Entity, With<NarrationText>>,
) {
    if is_garden_level(&level) {
        return;
    }
    despawn_old_narration(&mut commands, &existing);
    let quote = pick_quote(WEAPON_QUOTES, &mut state);
    show_narration(&mut commands, quote);
}

#[span_fn]
fn on_enemy_killed(
    _trigger: On<EnemyKilled>,
    mut commands: Commands,
    mut state: ResMut<NarrationState>,
    level: Res<CurrentLevel>,
    existing: Query<Entity, With<NarrationText>>,
) {
    if is_garden_level(&level) {
        return;
    }
    despawn_old_narration(&mut commands, &existing);
    let quote = pick_quote(KILL_QUOTES, &mut state);
    show_narration(&mut commands, quote);
}

// ---------------------------------------------------------------------------
// State-driven triggers
// ---------------------------------------------------------------------------

#[span_fn]
fn on_level_start(
    mut commands: Commands,
    mut state: ResMut<NarrationState>,
    level: Res<CurrentLevel>,
    existing: Query<Entity, With<NarrationText>>,
) {
    if is_garden_level(&level) {
        return;
    }
    despawn_old_narration(&mut commands, &existing);
    let quote = pick_quote(LEVEL_START_QUOTES, &mut state);
    show_narration(&mut commands, quote);
}

#[span_fn]
fn on_player_death(
    mut commands: Commands,
    mut state: ResMut<NarrationState>,
    level: Res<CurrentLevel>,
    existing: Query<Entity, With<NarrationText>>,
) {
    if is_garden_level(&level) {
        return;
    }
    despawn_old_narration(&mut commands, &existing);
    let quote = pick_quote(DEATH_QUOTES, &mut state);
    show_narration(&mut commands, quote);
}

// ---------------------------------------------------------------------------
// Fade system
// ---------------------------------------------------------------------------

#[span_fn]
fn fade_narration(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut NarrationFadeTimer, &mut TextColor), With<NarrationText>>,
) {
    for (entity, mut timer, mut color) in &mut query {
        timer.tick(time.delta());
        let elapsed = timer.elapsed_secs();

        // 3s display, then 1s fade
        if elapsed > 3.0 {
            let fade_progress = (elapsed - 3.0).min(1.0);
            color.0 = Color::srgba(1.0, 1.0, 0.8, 1.0 - fade_progress);
        }

        if timer.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_quote_returns_from_pool() {
        let mut state = NarrationState::default();
        let quote = pick_quote(MONEY_QUOTES, &mut state);
        assert!(MONEY_QUOTES.contains(&quote));
    }

    #[test]
    fn pick_quote_no_consecutive_duplicates() {
        let mut state = NarrationState::default();
        let first = pick_quote(MONEY_QUOTES, &mut state);
        let second = pick_quote(MONEY_QUOTES, &mut state);
        assert_ne!(first, second);
    }

    #[test]
    fn garden_level_suppresses() {
        let level = CurrentLevel(GARDEN_LEVEL);
        assert!(is_garden_level(&level));

        let normal_level = CurrentLevel(1);
        assert!(!is_garden_level(&normal_level));
    }

    #[test]
    fn narration_text_spawns() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let mut commands = app.world_mut().commands();
        show_narration(&mut commands, "Test quote");
        // Must apply commands for entities to exist
        app.world_mut().flush();

        let count = app
            .world_mut()
            .query::<&NarrationText>()
            .iter(app.world())
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn old_narration_replaced() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Spawn two narration texts
        let mut commands = app.world_mut().commands();
        show_narration(&mut commands, "First");
        show_narration(&mut commands, "Second");
        app.world_mut().flush();

        // Despawn old ones
        let existing: Vec<Entity> = app
            .world_mut()
            .query_filtered::<Entity, With<NarrationText>>()
            .iter(app.world())
            .collect();
        assert_eq!(existing.len(), 2);

        for entity in existing {
            app.world_mut().despawn(entity);
        }

        // Spawn new one
        let mut commands = app.world_mut().commands();
        show_narration(&mut commands, "Third");
        app.world_mut().flush();

        let count = app
            .world_mut()
            .query::<&NarrationText>()
            .iter(app.world())
            .count();
        assert_eq!(count, 1);
    }
}
