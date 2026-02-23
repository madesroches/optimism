use std::collections::HashMap;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_kira_audio::AudioSource;

use crate::components::LuxuryType;
use crate::plugins::combat::WeaponType;

#[derive(Resource, Debug)]
pub struct Score(pub u64);

#[derive(Resource, Debug)]
pub struct CurrentLevel(pub u32);

#[derive(Resource, Debug)]
pub struct Lives(pub u32);

// ---------------------------------------------------------------------------
// Level config
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Clone)]
pub struct LevelConfig {
    pub maze_file: String,
    pub weapon_type: WeaponType,
    pub luxury_type: LuxuryType,
    pub enemy_speed_multiplier: f32,
    pub weapon_duration_secs: f32,
    pub pen_release_interval_secs: f32,
}

/// Build the level configuration for a given level number.
pub fn level_config(level: u32) -> LevelConfig {
    // Garden level
    if level >= 13 {
        return LevelConfig {
            maze_file: "assets/maps/garden.txt".to_string(),
            weapon_type: WeaponType::BrassKnuckles,
            luxury_type: LuxuryType::GoldGrill,
            enemy_speed_multiplier: 0.0,
            weapon_duration_secs: 8.0,
            pen_release_interval_secs: 3.0,
        };
    }

    // Maze cycling: levels 1-12 cycle through 4 maps
    let maze_file = match ((level - 1) % 4) + 1 {
        1 => "assets/maps/level_01.txt",
        2 => "assets/maps/level_02.txt",
        3 => "assets/maps/level_03.txt",
        _ => "assets/maps/level_04.txt",
    }
    .to_string();

    // Weapon/luxury escalation per 2-level pairs
    let (weapon_type, luxury_type) = match level {
        1..=2 => (WeaponType::BrassKnuckles, LuxuryType::GoldGrill),
        3..=4 => (WeaponType::Bat, LuxuryType::Chain),
        5..=6 => (WeaponType::Knife, LuxuryType::Rolex),
        7..=8 => (WeaponType::Axe, LuxuryType::Goblet),
        9..=10 => (WeaponType::Chainsaw, LuxuryType::FurCoat),
        _ => (WeaponType::Chainsaw, LuxuryType::GoldToilet),
    };

    let enemy_speed_multiplier = 1.0 + (level - 1) as f32 * 0.08;
    let weapon_duration_secs = (8.0 - (level - 1) as f32 * 0.4).max(3.0);
    let pen_release_interval_secs = (3.0 - (level - 1) as f32 * 0.15).max(1.0);

    LevelConfig {
        maze_file,
        weapon_type,
        luxury_type,
        enemy_speed_multiplier,
        weapon_duration_secs,
        pen_release_interval_secs,
    }
}

// ---------------------------------------------------------------------------
// Game stats
// ---------------------------------------------------------------------------

#[derive(Resource, Debug, Default)]
pub struct GameStats {
    pub deaths: u32,
    pub kills_by_weapon: HashMap<WeaponType, u32>,
    pub money_collected: u64,
    pub luxuries_collected: Vec<LuxuryType>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_1_config() {
        let cfg = level_config(1);
        assert_eq!(cfg.weapon_type, WeaponType::BrassKnuckles);
        assert_eq!(cfg.luxury_type, LuxuryType::GoldGrill);
        assert_eq!(cfg.maze_file, "assets/maps/level_01.txt");
        assert!((cfg.enemy_speed_multiplier - 1.0).abs() < 0.01);
        assert!((cfg.weapon_duration_secs - 8.0).abs() < 0.01);
        assert!((cfg.pen_release_interval_secs - 3.0).abs() < 0.01);
    }

    #[test]
    fn weapon_escalation() {
        assert_eq!(level_config(1).weapon_type, WeaponType::BrassKnuckles);
        assert_eq!(level_config(2).weapon_type, WeaponType::BrassKnuckles);
        assert_eq!(level_config(3).weapon_type, WeaponType::Bat);
        assert_eq!(level_config(5).weapon_type, WeaponType::Knife);
        assert_eq!(level_config(7).weapon_type, WeaponType::Axe);
        assert_eq!(level_config(9).weapon_type, WeaponType::Chainsaw);
        assert_eq!(level_config(11).weapon_type, WeaponType::Chainsaw);
    }

    #[test]
    fn luxury_escalation() {
        assert_eq!(level_config(1).luxury_type, LuxuryType::GoldGrill);
        assert_eq!(level_config(3).luxury_type, LuxuryType::Chain);
        assert_eq!(level_config(5).luxury_type, LuxuryType::Rolex);
        assert_eq!(level_config(7).luxury_type, LuxuryType::Goblet);
        assert_eq!(level_config(9).luxury_type, LuxuryType::FurCoat);
        assert_eq!(level_config(11).luxury_type, LuxuryType::GoldToilet);
    }

    #[test]
    fn maze_cycling() {
        assert!(level_config(1).maze_file.contains("level_01"));
        assert!(level_config(2).maze_file.contains("level_02"));
        assert!(level_config(3).maze_file.contains("level_03"));
        assert!(level_config(4).maze_file.contains("level_04"));
        // Cycles back
        assert!(level_config(5).maze_file.contains("level_01"));
        assert!(level_config(9).maze_file.contains("level_01"));
    }

    #[test]
    fn speed_increases_with_level() {
        let s1 = level_config(1).enemy_speed_multiplier;
        let s6 = level_config(6).enemy_speed_multiplier;
        let s12 = level_config(12).enemy_speed_multiplier;
        assert!(s6 > s1);
        assert!(s12 > s6);
    }

    #[test]
    fn weapon_duration_decreases_with_level() {
        let d1 = level_config(1).weapon_duration_secs;
        let d6 = level_config(6).weapon_duration_secs;
        let d12 = level_config(12).weapon_duration_secs;
        assert!(d6 < d1);
        assert!(d12 <= d6);
        assert!(d12 >= 3.0); // min clamp
    }

    #[test]
    fn pen_release_decreases_with_level() {
        let p1 = level_config(1).pen_release_interval_secs;
        let p12 = level_config(12).pen_release_interval_secs;
        assert!(p12 < p1);
        assert!(p12 >= 1.0); // min clamp
    }

    #[test]
    fn garden_level() {
        let cfg = level_config(13);
        assert!(cfg.maze_file.contains("garden"));
        assert_eq!(cfg.enemy_speed_multiplier, 0.0);
    }

    #[test]
    fn levels_above_13_are_garden() {
        let cfg = level_config(14);
        assert!(cfg.maze_file.contains("garden"));
        assert_eq!(cfg.enemy_speed_multiplier, 0.0);
    }
}

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
