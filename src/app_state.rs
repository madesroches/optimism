use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, SubStates)]
#[source(AppState = AppState::InGame)]
pub enum PlayingState {
    #[default]
    LevelIntro,
    Playing,
    Paused,
    PlayerDeath,
    LevelComplete,
    LevelTransition,
}
