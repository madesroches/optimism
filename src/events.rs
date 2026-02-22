//! Game events triggered by gameplay systems and observed by audio/narration.

use bevy::prelude::*;

#[derive(Event)]
pub struct MoneyCollected;

#[derive(Event)]
pub struct WeaponPickedUp;

#[derive(Event)]
pub struct EnemyKilled;
