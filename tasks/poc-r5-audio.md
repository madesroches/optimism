# PoC R5: Audio Pipeline

**Risk**: R5 (Medium) — Procedural audio generation + Bevy integration
**Goal**: Prove that we can generate period-appropriate music (harpsichord via MIDI+FluidSynth) and synthesized SFX (numpy), then load and play them in Bevy via `bevy_kira_audio` — all headlessly testable.
**Status**: PASSED

---

## 1. Questions to Answer

1. Can FluidSynth render MIDI to WAV with a harpsichord soundfont, then convert to OGG?
2. Can numpy synthesize retro-style SFX (dot pickup, ghost collision, power pellet) and export as OGG?
3. Does `bevy_kira_audio` load OGG assets under `MinimalPlugins` (headless, no audio device)?
4. Can we verify audio asset handles resolve without actually playing sound?
5. Is the full pipeline reproducible — run one Python script, get all audio assets?

---

## 2. Why This Matters

The game design doc calls for "whimsical harpsichord-driven" music evoking Candide's 18th-century setting, plus arcade SFX for gameplay feedback. We need to validate:

- The toolchain (Python MIDI generation → FluidSynth rendering → OGG encoding) works end-to-end
- Generated assets are loadable by Bevy's audio stack without a real audio device
- The pipeline is scriptable and reproducible (no manual DAW work)

If MIDI+FluidSynth doesn't produce acceptable harpsichord sounds, or if `bevy_kira_audio` can't load assets headlessly, we need to know before building the full soundtrack.

---

## 3. Implementation Plan

### Step 1: Python Environment

- Create `.venv/` with `midiutil`, `numpy`, `scipy`
- Install system packages: `fluidsynth`, `fluid-soundfont-gm` (or equivalent)
- Add `.venv/` to `.gitignore`, create `requirements.txt`

### Step 2: Audio Generation Script (`tools/generate_audio.py`)

**Music tracks** (MIDI → FluidSynth → OGG):
- `menu_theme.ogg` — Simple harpsichord melody, ~30s loop
- `gameplay.ogg` — Uptempo harpsichord piece, ~60s loop

**SFX** (numpy synthesis → OGG):
- `dot_pickup.ogg` — Short chirp (sine wave sweep up)
- `power_pellet.ogg` — Longer ascending tone with vibrato
- `ghost_eaten.ogg` — Descending warble
- `death.ogg` — Dramatic descending tone
- `level_complete.ogg` — Ascending fanfare arpeggio

### Step 3: Cargo.toml Update

Add `ogg` feature to `bevy_kira_audio` (it supports `ogg`, `mp3`, `wav`, `flac`).

### Step 4: Headless Test (`tests/poc_r5_audio.rs`)

Test that `bevy_kira_audio` plugin initializes under `MinimalPlugins` and OGG assets can be loaded (handle creation succeeds). We cannot play audio without a device, but we can verify the asset pipeline.

---

## 4. Test Plan

### Test 1: Audio plugin initializes headlessly

```
Create App with MinimalPlugins + AudioPlugin
app.finish() + app.cleanup()
Run app.update() — no panic
```

**Pass criteria**: `bevy_kira_audio` doesn't crash without an audio device.

### Test 2: OGG asset handles can be created

```
Create App with MinimalPlugins + AudioPlugin + AssetPlugin
Load an OGG file via AssetServer
Verify the handle is valid (not errored)
```

**Pass criteria**: Asset handle is created without error.

---

## 5. Files to Modify

| File | Change |
|------|--------|
| `tasks/poc-r5-audio.md` | This file — plan + results |
| `.venv/` | New — Python virtual environment |
| `.gitignore` | Add `.venv/` |
| `requirements.txt` | New — `midiutil`, `numpy`, `scipy` |
| `tools/generate_audio.py` | New — music + SFX generation script |
| `assets/audio/music/` | New directory — generated OGG tracks |
| `assets/audio/sfx/` | New directory — generated OGG effects |
| `Cargo.toml` | No change needed — `ogg` is default feature |
| `tests/poc_r5_audio.rs` | New — headless audio loading test |

---

## 6. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| FluidSynth not available in WSL2 | Medium | Install via apt, fall back to timidity |
| No harpsichord in GM soundfont | Low | GM program 6 = harpsichord, universally supported |
| `bevy_kira_audio` panics without audio device | Medium | May need to catch/skip, or use mock backend |
| OGG encoding needs extra Python deps | Low | scipy.io.wavfile + subprocess ffmpeg, or use soundfile |

---

## 7. Pass / Fail Criteria

**PASS** if:
- Python script generates all 7 audio files (2 music + 5 SFX)
- All files are valid OGG format
- `cargo test` passes — `bevy_kira_audio` loads assets headlessly
- Pipeline is single-command reproducible

**FAIL** if:
- FluidSynth can't render harpsichord MIDI
- numpy SFX generation produces silence or artifacts
- `bevy_kira_audio` crashes without audio device (and no workaround exists)

---

## 8. Results

**Date**: 2026-02-22
**Verdict**: PASSED — all criteria met

### Audio Generation

Python script (`tools/generate_audio.py`) generates all 7 files in a single run:

| File | Type | Size |
|------|------|------|
| `assets/audio/music/menu_theme.ogg` | Vorbis stereo 44100Hz | 376.8 KB |
| `assets/audio/music/gameplay.ogg` | Vorbis stereo 44100Hz | 212.8 KB |
| `assets/audio/sfx/dot_pickup.ogg` | Vorbis mono 44100Hz | 3.9 KB |
| `assets/audio/sfx/power_pellet.ogg` | Vorbis mono 44100Hz | 4.4 KB |
| `assets/audio/sfx/ghost_eaten.ogg` | Vorbis mono 44100Hz | 4.2 KB |
| `assets/audio/sfx/death.ogg` | Vorbis mono 44100Hz | 5.1 KB |
| `assets/audio/sfx/level_complete.ogg` | Vorbis mono 44100Hz | 5.6 KB |

Pipeline: MIDI → FluidSynth (FluidR3_GM.sf2, program 6 harpsichord) → ffmpeg → OGG for music; numpy synthesis (phase-accumulation frequency sweeps) → scipy WAV → ffmpeg → OGG for SFX.

### Headless Audio Tests

Both tests pass under `cargo test`:

| Test | Result |
|------|--------|
| `audio_plugin_initializes_headless` | PASS |
| `ogg_asset_handle_creation` | PASS |

### Key Findings

1. **`bevy_kira_audio` works headlessly** — contrary to the dep_compat.rs comment, AudioPlugin does NOT panic without an audio device. It wraps the kira AudioManager in `Option` and logs a warning (`"Failed to setup audio"`) when ALSA fails, then silently skips all playback. ALSA stderr warnings appear but don't affect test results.

2. **OGG is the default feature** — no Cargo.toml change needed. `bevy_kira_audio = "0.25"` includes OGG support out of the box.

3. **FluidSynth GM soundfont includes harpsichord** — program 6 renders cleanly at 44100Hz via `fluidsynth -ni`.

4. **Full test suite unaffected** — all 14 tests pass (3 telemetry + 8 headless ECS + 1 dep_compat + 2 audio).

### System Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| fluidsynth | 2.3.4 | MIDI → WAV rendering |
| fluid-soundfont-gm | (system) | GM soundfont at `/usr/share/sounds/sf2/FluidR3_GM.sf2` |
| ffmpeg | (system) | WAV → OGG encoding |
| Python venv | midiutil, numpy, scipy | MIDI generation + waveform synthesis |

### Audio Playback Verification

All 7 files confirmed audible via `ffplay` in WSL2 (PulseAudio bridge to Windows).

### Licensing

All generated audio is rights-free:

| Component | License | Notes |
|-----------|---------|-------|
| FluidR3_GM soundfont | MIT | Frank Wen, 2000-2008 |
| MIDI melodies | Original | Composed in `generate_audio.py` |
| SFX waveforms | Original | Pure numpy synthesis, no samples |
| midiutil | MIT | |
| numpy / scipy | BSD | |
| FluidSynth | LGPL-2.1 | Used as tool, not linked — no obligation on output |
