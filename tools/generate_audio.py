#!/usr/bin/env python3
"""Generate all audio assets for Optimism.

Music: MIDI → FluidSynth → OGG (harpsichord, period-appropriate)
SFX:   numpy synthesis → WAV → OGG

Usage:
    python3 tools/generate_audio.py

Output:
    assets/audio/music/menu_theme.ogg
    assets/audio/music/gameplay.ogg
    assets/audio/sfx/dot_pickup.ogg
    assets/audio/sfx/power_pellet.ogg
    assets/audio/sfx/ghost_eaten.ogg
    assets/audio/sfx/death.ogg
    assets/audio/sfx/level_complete.ogg
"""

import os
import subprocess
import sys
import tempfile

import numpy as np
from midiutil import MIDIFile
from scipy.io import wavfile

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

SAMPLE_RATE = 44100
SOUNDFONT = "/usr/share/sounds/sf2/FluidR3_GM.sf2"
HARPSICHORD_PROGRAM = 6  # GM program 6 = harpsichord

PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MUSIC_DIR = os.path.join(PROJECT_ROOT, "assets", "audio", "music")
SFX_DIR = os.path.join(PROJECT_ROOT, "assets", "audio", "sfx")


def ensure_dirs():
    os.makedirs(MUSIC_DIR, exist_ok=True)
    os.makedirs(SFX_DIR, exist_ok=True)


# ---------------------------------------------------------------------------
# MIDI → OGG via FluidSynth
# ---------------------------------------------------------------------------


def midi_to_ogg(midi_path: str, ogg_path: str):
    """Render a MIDI file to OGG using FluidSynth."""
    wav_path = midi_path.replace(".mid", ".wav")
    # FluidSynth render to WAV
    subprocess.run(
        [
            "fluidsynth",
            "-ni",
            SOUNDFONT,
            midi_path,
            "-F",
            wav_path,
            "-r",
            str(SAMPLE_RATE),
        ],
        check=True,
        capture_output=True,
    )
    # WAV → OGG via ffmpeg or sox; fall back to keeping WAV if neither available
    wav_to_ogg(wav_path, ogg_path)
    os.remove(wav_path)


def wav_to_ogg(wav_path: str, ogg_path: str):
    """Convert WAV to OGG using ffmpeg, sox, or oggenc."""
    for cmd in [
        ["ffmpeg", "-y", "-i", wav_path, "-c:a", "libvorbis", "-q:a", "4", ogg_path],
        ["oggenc", "-o", ogg_path, wav_path],
        ["sox", wav_path, ogg_path],
    ]:
        try:
            subprocess.run(cmd, check=True, capture_output=True)
            return
        except FileNotFoundError:
            continue
    raise SystemExit("No OGG encoder found. Install one with: sudo apt-get install ffmpeg")


def numpy_to_ogg(samples: np.ndarray, ogg_path: str):
    """Write numpy float array to OGG via intermediate WAV."""
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
        wav_path = f.name
    # Normalize to int16 range
    peak = np.max(np.abs(samples))
    if peak > 0:
        samples = samples / peak
    int_samples = (samples * 32767).astype(np.int16)
    wavfile.write(wav_path, SAMPLE_RATE, int_samples)
    wav_to_ogg(wav_path, ogg_path)
    os.remove(wav_path)


# ---------------------------------------------------------------------------
# Music generation
# ---------------------------------------------------------------------------

# Baroque-ish note sequences (MIDI note numbers)
# C major / A minor patterns for that Candide-era vibe

def generate_menu_theme():
    """Simple harpsichord minuet — 30 seconds, loopable."""
    print("  Generating menu_theme...")
    midi = MIDIFile(1)
    track, channel, volume = 0, 0, 100
    tempo = 100  # BPM
    midi.addTempo(track, 0, tempo)
    midi.addProgramChange(track, channel, 0, HARPSICHORD_PROGRAM)

    # A gentle minuet-like melody in C major (3/4 feel)
    # Each tuple: (pitch, duration_in_beats)
    melody = [
        # Phrase 1
        (72, 1), (74, 0.5), (76, 0.5), (77, 1), (76, 1),
        (74, 1), (72, 0.5), (71, 0.5), (69, 1), (71, 1),
        (72, 2), (0, 1),  # 0 = rest
        # Phrase 2
        (76, 1), (77, 0.5), (79, 0.5), (81, 1), (79, 1),
        (77, 1), (76, 0.5), (74, 0.5), (72, 1), (74, 1),
        (76, 2), (0, 1),
        # Phrase 3 — descending
        (84, 1), (81, 0.5), (79, 0.5), (77, 1), (76, 1),
        (74, 1), (72, 0.5), (71, 0.5), (69, 1), (67, 1),
        (69, 2), (0, 1),
        # Phrase 4 — resolution
        (72, 1), (76, 1), (79, 1),
        (77, 0.5), (76, 0.5), (74, 0.5), (72, 0.5), (71, 1),
        (72, 3),
    ]

    # Simple bass accompaniment
    bass = [
        (48, 3), (45, 3), (48, 3), (43, 3),
        (48, 3), (45, 3), (48, 3), (43, 3),
        (48, 3), (45, 3), (41, 3), (43, 3),
        (48, 3), (43, 3), (45, 3), (48, 3),
    ]

    # Write melody
    time = 0
    for pitch, dur in melody:
        if pitch > 0:
            midi.addNote(track, channel, pitch, time, dur * 0.9, volume)
        time += dur

    # Write bass on channel 1
    midi.addProgramChange(track, 1, 0, HARPSICHORD_PROGRAM)
    time = 0
    for pitch, dur in bass:
        midi.addNote(track, 1, pitch, time, dur * 0.9, volume - 20)
        time += dur

    with tempfile.NamedTemporaryFile(suffix=".mid", delete=False) as f:
        midi.writeFile(f)
        midi_path = f.name

    midi_to_ogg(midi_path, os.path.join(MUSIC_DIR, "menu_theme.ogg"))
    os.remove(midi_path)


def generate_gameplay_music():
    """Uptempo harpsichord piece — ~45 seconds, loopable."""
    print("  Generating gameplay...")
    midi = MIDIFile(1)
    track, channel, volume = 0, 0, 105
    tempo = 140  # BPM — brisk
    midi.addTempo(track, 0, tempo)
    midi.addProgramChange(track, channel, 0, HARPSICHORD_PROGRAM)

    # Lively baroque-inspired running notes in D minor
    melody = [
        # Fast running passage 1
        (62, 0.5), (65, 0.5), (69, 0.5), (72, 0.5),
        (74, 1), (72, 0.5), (69, 0.5),
        (65, 0.5), (67, 0.5), (69, 0.5), (70, 0.5),
        (69, 1), (67, 1),
        # Phrase 2
        (65, 0.5), (69, 0.5), (72, 0.5), (74, 0.5),
        (77, 1), (74, 0.5), (72, 0.5),
        (69, 0.5), (70, 0.5), (72, 0.5), (74, 0.5),
        (72, 1), (70, 1),
        # Energetic middle section
        (74, 0.5), (77, 0.5), (74, 0.5), (72, 0.5),
        (69, 0.5), (72, 0.5), (69, 0.5), (67, 0.5),
        (65, 0.5), (67, 0.5), (69, 0.5), (70, 0.5),
        (72, 1), (74, 1),
        # Resolution back to D
        (77, 0.5), (74, 0.5), (72, 0.5), (69, 0.5),
        (70, 0.5), (67, 0.5), (65, 0.5), (62, 0.5),
        (60, 0.5), (62, 0.5), (65, 0.5), (69, 0.5),
        (62, 2),
    ]

    # Driving bass
    bass = [
        (38, 2), (41, 2), (43, 2), (45, 2),
        (38, 2), (41, 2), (43, 2), (45, 2),
        (46, 2), (43, 2), (41, 2), (38, 2),
        (46, 2), (43, 2), (41, 1), (38, 1), (38, 2),
    ]

    time = 0
    for pitch, dur in melody:
        if pitch > 0:
            midi.addNote(track, channel, pitch, time, dur * 0.85, volume)
        time += dur

    midi.addProgramChange(track, 1, 0, HARPSICHORD_PROGRAM)
    time = 0
    for pitch, dur in bass:
        midi.addNote(track, 1, pitch, time, dur * 0.85, volume - 15)
        time += dur

    with tempfile.NamedTemporaryFile(suffix=".mid", delete=False) as f:
        midi.writeFile(f)
        midi_path = f.name

    midi_to_ogg(midi_path, os.path.join(MUSIC_DIR, "gameplay.ogg"))
    os.remove(midi_path)


# ---------------------------------------------------------------------------
# SFX generation (numpy synthesis)
# ---------------------------------------------------------------------------


def fade_in_out(samples: np.ndarray, fade_ms: int = 5) -> np.ndarray:
    """Apply short fade in/out to avoid clicks."""
    fade_len = int(SAMPLE_RATE * fade_ms / 1000)
    fade_len = min(fade_len, len(samples) // 2)
    samples[:fade_len] *= np.linspace(0, 1, fade_len)
    samples[-fade_len:] *= np.linspace(1, 0, fade_len)
    return samples


def generate_dot_pickup():
    """Short chirp — sine sweep from 800Hz to 1200Hz, 80ms."""
    print("  Generating dot_pickup...")
    duration = 0.08
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration), endpoint=False)
    freq = np.linspace(800, 1200, len(t))
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    samples = 0.8 * np.sin(phase)
    samples = fade_in_out(samples)
    numpy_to_ogg(samples, os.path.join(SFX_DIR, "dot_pickup.ogg"))


def generate_power_pellet():
    """Ascending tone with vibrato, 300ms."""
    print("  Generating power_pellet...")
    duration = 0.3
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration), endpoint=False)
    freq = np.linspace(400, 1000, len(t))
    vibrato = 30 * np.sin(2 * np.pi * 12 * t)  # 12Hz vibrato
    phase = 2 * np.pi * np.cumsum(freq + vibrato) / SAMPLE_RATE
    samples = 0.8 * np.sin(phase)
    samples = fade_in_out(samples)
    numpy_to_ogg(samples, os.path.join(SFX_DIR, "power_pellet.ogg"))


def generate_ghost_eaten():
    """Descending warble, 250ms."""
    print("  Generating ghost_eaten...")
    duration = 0.25
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration), endpoint=False)
    freq = np.linspace(1200, 300, len(t))
    warble = 80 * np.sin(2 * np.pi * 15 * t)  # 15Hz warble
    phase = 2 * np.pi * np.cumsum(freq + warble) / SAMPLE_RATE
    samples = 0.7 * np.sin(phase)
    samples = fade_in_out(samples)
    numpy_to_ogg(samples, os.path.join(SFX_DIR, "ghost_eaten.ogg"))


def generate_death():
    """Dramatic descending tone, 500ms."""
    print("  Generating death...")
    duration = 0.5
    t = np.linspace(0, duration, int(SAMPLE_RATE * duration), endpoint=False)
    freq = np.linspace(600, 100, len(t))
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    # Add some harmonics for drama
    samples = (
        0.5 * np.sin(phase)
        + 0.3 * np.sin(2 * phase)
        + 0.1 * np.sin(3 * phase)
    )
    # Exponential decay envelope
    envelope = np.exp(-3 * t / duration)
    samples *= envelope
    samples = fade_in_out(samples)
    numpy_to_ogg(samples, os.path.join(SFX_DIR, "death.ogg"))


def generate_level_complete():
    """Ascending fanfare arpeggio — C-E-G-C, 600ms total."""
    print("  Generating level_complete...")
    note_dur = 0.15
    notes_hz = [523.25, 659.25, 783.99, 1046.50]  # C5-E5-G5-C6
    all_samples = []
    for freq in notes_hz:
        t = np.linspace(0, note_dur, int(SAMPLE_RATE * note_dur), endpoint=False)
        note = 0.8 * np.sin(2 * np.pi * freq * t)
        # Add a bit of sparkle with 2nd harmonic
        note += 0.2 * np.sin(2 * np.pi * freq * 2 * t)
        note = fade_in_out(note)
        all_samples.append(note)
    # Final note rings longer
    final_dur = 0.3
    t = np.linspace(0, final_dur, int(SAMPLE_RATE * final_dur), endpoint=False)
    final_note = 0.8 * np.sin(2 * np.pi * 1046.50 * t)
    final_note += 0.2 * np.sin(2 * np.pi * 1046.50 * 2 * t)
    envelope = np.exp(-3 * t / final_dur)
    final_note *= envelope
    final_note = fade_in_out(final_note)
    all_samples[-1] = final_note  # Replace last short note with ringing one

    samples = np.concatenate(all_samples)
    numpy_to_ogg(samples, os.path.join(SFX_DIR, "level_complete.ogg"))


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    if not os.path.isfile(SOUNDFONT):
        print(f"ERROR: Soundfont not found at {SOUNDFONT}")
        print("Install with: sudo apt-get install fluid-soundfont-gm")
        sys.exit(1)

    ensure_dirs()

    print("Generating music tracks (MIDI → FluidSynth → OGG)...")
    generate_menu_theme()
    generate_gameplay_music()

    print("Generating SFX (numpy synthesis → OGG)...")
    generate_dot_pickup()
    generate_power_pellet()
    generate_ghost_eaten()
    generate_death()
    generate_level_complete()

    print("\nDone! Generated files:")
    for root, dirs, files in os.walk(os.path.join(PROJECT_ROOT, "assets", "audio")):
        for f in sorted(files):
            path = os.path.join(root, f)
            size_kb = os.path.getsize(path) / 1024
            rel = os.path.relpath(path, PROJECT_ROOT)
            print(f"  {rel} ({size_kb:.1f} KB)")


if __name__ == "__main__":
    main()
