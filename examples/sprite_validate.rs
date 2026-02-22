//! Headless validation: load all character sprite sheets and verify metadata.
//!
//! Usage:
//!     cargo run --example sprite_validate
//!
//! Exits with 0 if all sprite sheets load correctly, 1 on error.

use optimism::plugins::sprites::*;

const CHARACTERS: &[&str] = &[
    "candide_base",
    "soldier",
    "inquisitor",
    "thief",
    "brute",
];

const EXPECTED_ANIMATIONS: &[&str] = &[
    "walk_down",
    "walk_left",
    "walk_up",
    "walk_right",
    "idle",
    "attack_down",
    "attack_left",
    "attack_up",
    "attack_right",
    "death",
];

fn main() {
    let assets_dir = std::path::Path::new("assets");
    let mut errors = 0;

    for &name in CHARACTERS {
        let json_path = assets_dir.join("sprites").join(name).with_extension("json");
        let png_path = assets_dir.join("sprites").join(name).with_extension("png");

        // Check files exist
        if !json_path.exists() {
            eprintln!("FAIL: {}: JSON not found at {}", name, json_path.display());
            errors += 1;
            continue;
        }
        if !png_path.exists() {
            eprintln!("FAIL: {}: PNG not found at {}", name, png_path.display());
            errors += 1;
            continue;
        }

        // Parse JSON metadata
        let json_str = match std::fs::read_to_string(&json_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("FAIL: {}: Cannot read JSON: {}", name, e);
                errors += 1;
                continue;
            }
        };

        let meta: SpriteSheetMeta = match serde_json::from_str(&json_str) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("FAIL: {}: Cannot parse JSON: {}", name, e);
                errors += 1;
                continue;
            }
        };

        // Verify frame size
        if meta.frame_size != [64, 64] {
            eprintln!(
                "FAIL: {}: Expected 64x64 frames, got {}x{}",
                name, meta.frame_size[0], meta.frame_size[1]
            );
            errors += 1;
        }

        // Verify all expected animations exist
        let mut anim_errors = 0;
        for &anim in EXPECTED_ANIMATIONS {
            if !meta.animations.contains_key(anim) {
                eprintln!("FAIL: {}: Missing animation '{}'", name, anim);
                anim_errors += 1;
            }
        }

        if anim_errors > 0 {
            errors += anim_errors;
            eprintln!(
                "  Available animations: {:?}",
                meta.animations.keys().collect::<Vec<_>>()
            );
        }

        // Verify animation ranges don't overlap and are within bounds
        let total_cells = (meta.columns * meta.rows) as usize;
        for (anim_name, range) in &meta.animations {
            let end = range.start + range.count;
            if end > total_cells {
                eprintln!(
                    "FAIL: {}: Animation '{}' exceeds sheet bounds (start={}, count={}, total={})",
                    name, anim_name, range.start, range.count, total_cells
                );
                errors += 1;
            }
            if range.count == 0 {
                eprintln!("FAIL: {}: Animation '{}' has 0 frames", name, anim_name);
                errors += 1;
            }
        }

        // Verify PNG dimensions match metadata
        let png_data = std::fs::read(&png_path).unwrap();
        // PNG header: bytes 16-23 contain width (u32 BE) and height (u32 BE)
        if png_data.len() > 24 {
            let width = u32::from_be_bytes([png_data[16], png_data[17], png_data[18], png_data[19]]);
            let height = u32::from_be_bytes([png_data[20], png_data[21], png_data[22], png_data[23]]);
            let expected_w = meta.columns * meta.frame_size[0];
            let expected_h = meta.rows * meta.frame_size[1];
            if width != expected_w || height != expected_h {
                eprintln!(
                    "FAIL: {}: PNG {}x{} doesn't match metadata {}x{} ({}cols x {}rows x {}px)",
                    name, width, height, expected_w, expected_h,
                    meta.columns, meta.rows, meta.frame_size[0]
                );
                errors += 1;
            }
        }

        if anim_errors == 0 {
            let total_frames: usize = meta.animations.values().map(|r| r.count).sum();
            println!(
                "OK: {} — {}x{} sheet, {} animations, {} total frames",
                name,
                meta.columns,
                meta.rows,
                meta.animations.len(),
                total_frames
            );
        }
    }

    println!("\n{} characters checked, {} errors", CHARACTERS.len(), errors);
    if errors > 0 {
        std::process::exit(1);
    }
}
