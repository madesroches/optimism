# PoC R2: Dependency Compatibility

**Risk**: R2 (Critical) — Architecture doc Section 13
**Goal**: Verify that all dependencies from the architecture doc (Section 1) compile together against Bevy 0.18 on Rust 1.93, and identify version corrections needed before writing game code.

---

## 1. Research Findings

### Crate version corrections

The architecture doc (Section 1) lists several versions that are now stale:

| Crate | Arch doc version | Correct version | Notes |
|-------|-----------------|-----------------|-------|
| `bevy_kira_audio` | `"0.24"` | `"0.25"` | 0.24 targets Bevy 0.17. 0.25.0 released 2026-01-14, targets Bevy 0.18 |
| `bevy_asset_loader` | `"0.25.0-rc.1"` | `"0.25"` | Stable 0.25.0 released 2026-01-14 — RC pin and comment are outdated |
| `avian2d` | `"0.5"` | `"0.5"` | Correct. 0.5.0 released 2026-01-17, targets Bevy 0.18 |
| `micromegas` | `"0.20"` | `"0.20"` | Correct. Validated by PoC R1 |
| `pathfinding` | `"4"` | `"4"` | No Bevy dependency |
| `rand` | `"0.8"` | `"0.8"` | No Bevy dependency |

### Bevy feature configuration

PoC R1 discovered that `default_app` pulls in winit which "fails on Rust 1.93". The actual issue was more nuanced: winit compiles fine on 1.93, but only when a **platform backend** feature (`x11` or `wayland`) is specified. Without one, winit's `platform_impl` module has no concrete types, causing type inference errors.

The high-level Bevy feature collections (`2d`, `ui`) include `default_platform`, which pulls in `wayland`. The `wayland` feature requires `libwayland-dev` (linked at build time via pkg-config). The `x11` feature uses `x11-dl` which loads libX11 at runtime via `dlopen` — no dev headers needed at build time.

**Solution**: Compose Bevy features from mid-level collections instead of using `2d`/`ui`:

```toml
bevy = { version = "0.18", default-features = false, features = [
    # Core app framework
    "default_app",
    "bevy_winit",
    "multi_threaded",
    # Platform (x11 only — wayland needs libwayland-dev)
    "std",
    "x11",
    # 2D rendering
    "bevy_render",
    "bevy_core_pipeline",
    "bevy_sprite",
    "bevy_sprite_render",
    "bevy_gizmos_render",
    "bevy_post_process",
    # UI
    "ui_api",
    "ui_bevy_render",
    # Other
    "scene",
    "picking",
    "default_font",
] }
```

### System library dependencies

| Feature | System Library | Linking | Status on this machine |
|---------|---------------|---------|----------------------|
| `x11` | libX11 | dlopen (runtime) | No build-time dep needed |
| `wayland` | libwayland-client | pkg-config (build) | **Missing** — avoid this feature |
| `audio` (bevy_audio) | libasound (ALSA) | pkg-config (build) | **Missing** — `bevy_kira_audio` will need `libasound2-dev` |

Audio compilation will require: `sudo apt install libasound2-dev`

---

## 2. Implementation Steps

### Step 1: Update Cargo.toml with full dependency list

Add all game dependencies to `Cargo.toml` with corrected versions. Use the mid-level Bevy feature set from the research above. Keep the existing `micromegas` and `serial_test` deps unchanged.

### Step 2: Install system dependencies

Install `libasound2-dev` for audio crate compilation (needed by kira → cpal → ALSA).

### Step 3: Run `cargo check`

Verify the full dependency set resolves and compiles. This is the core test — if this passes, all crates are compatible with each other and with Bevy 0.18 on Rust 1.93.

### Step 4: Minimal smoke test

Write a trivial `cargo test` that constructs a Bevy `App` using `MinimalPlugins` + individual plugins from each new crate (`AudioPlugin`, `PhysicsPlugins`, `AssetLoaderPlugin`, etc.). This confirms the crates don't just compile in isolation but can coexist in the same headless Bevy app.

### Step 5: Record results

The PoC proves compatibility — it doesn't add game code. After recording results:
- Keep the corrected `Cargo.toml` (these are the deps we'll use going forward)
- Remove any smoke-test code that isn't needed
- Document findings in this file (update Status to DONE)
- Note any architecture doc corrections needed

---

## 3. Success Criteria

| Criterion | Command | Expected |
|-----------|---------|----------|
| All deps resolve | `cargo check` | Compiles with no errors |
| Existing PoC R1 tests still pass | `cargo test -- --test-threads=1` | 3 passed, 0 failed |
| Smoke test passes | `cargo test dep_compat` | New test passes |

---

## 4. Architecture Doc Updates Needed (post-PoC)

If successful, the architecture doc should be updated:
1. Section 1: `bevy_kira_audio = "0.24"` → `"0.25"`
2. Section 1: `bevy_asset_loader = "0.25.0-rc.1"` → `"0.25"` (remove RC comment)
3. Section 1: Add explicit Bevy feature list instead of bare `bevy = "0.18"`
4. Section 13 R2: Mark as resolved with findings
5. Section 13 R1: Correct the winit finding — the issue was missing platform backend, not a Rust 1.93 incompatibility

---

## 5. Fallback Plan

If any crate fails to compile:
- **bevy_kira_audio**: Fall back to Bevy's built-in `bevy_audio`. Loses two-channel architecture but the game still works.
- **bevy_asset_loader**: Fall back to manual `AssetServer` loading. More boilerplate but functional.
- **avian2d**: Drop it entirely. The architecture already treats Avian as a safety net — grid logic is the source of truth for collision.
