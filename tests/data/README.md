# Test Audio Samples

This directory contains sample audio files for integration testing of the audio-ai application.

## Overview

These samples are generated programmatically to ensure consistent, reproducible test data with known characteristics. They can be regenerated at any time using the generation script.

## Generated Files

### Single Tone Samples
Simple sine wave tones at specific frequencies for basic pitch detection testing:
- `tone_a4_440hz.wav` - A4 note (440 Hz), 2 seconds
- `tone_c4_262hz.wav` - C4 note (261.63 Hz), 2 seconds
- `tone_e4_330hz.wav` - E4 note (329.63 Hz), 2 seconds
- `tone_c5_523hz.wav` - C5 note (523.25 Hz), 2 seconds

### Note Sequences
Multi-note sequences for testing note extraction and comparison:
- `scale_c_major.wav` - C major scale (C4-C5), 8 notes
- `melody_simple.wav` - Simple 7-note melody (A4-A4-B4-C5-C5-B4-A4)
- `melody_simple_timing_variant.wav` - Same melody with timing variations
- `melody_simple_pitch_variant.wav` - Same melody with pitch variations (slightly out of tune)

### Rhythm Patterns
Consistent tempo patterns for rhythm and timing tests:
- `rhythm_quarter_notes_120bpm.wav` - 8 quarter notes at 120 BPM
- `rhythm_eighth_notes_120bpm.wav` - 16 eighth notes at 120 BPM

### MIDI Files
MIDI files for testing MIDI support and conversion:
- `midi_c_major_chord.mid` - C major chord (C4-E4-G4)
- `midi_simple_melody.mid` - Simple melody (same as melody_simple.wav)
- `midi_c_major_scale.mid` - C major scale (C4-C5)

## Regenerating Samples

To regenerate all test samples:

```bash
cd tests
python3 generate_test_samples.py
```

### Requirements
- Python 3.7+
- sox (command-line audio tool)
- midiutil (Python package: `pip3 install midiutil`)

Install system dependencies:
```bash
# Ubuntu/Debian
sudo apt-get install sox libsox-fmt-all

# macOS (Homebrew)
brew install sox

# Windows (Chocolatey)
choco install sox

# Windows (Scoop)
scoop install sox
# Install Python package
pip3 install midiutil
```

## Usage in Tests

These samples can be used for various integration tests:

### Basic Pitch Detection
```rust
let result = analyze_audio("tests/data/tone_a4_440hz.wav")?;
assert!(result.pitch_hz.iter().any(|&f| (f - 440.0).abs() < 5.0));
```

### Note Sequence Extraction
```rust
let result = analyze_audio("tests/data/scale_c_major.wav")?;
let notes = extract_note_sequence(&result);
assert_eq!(notes.len(), 8); // C major scale has 8 notes
```

### Comparison Testing
```rust
let reference = analyze_audio("tests/data/melody_simple.wav")?;
let variant = analyze_audio("tests/data/melody_simple_pitch_variant.wav")?;
let metrics = compare_recordings(&reference, &variant);
assert!(metrics.pitch_accuracy < 1.0); // Should detect pitch errors
```

### Rhythm Analysis
```rust
let result = analyze_audio("tests/data/rhythm_quarter_notes_120bpm.wav")?;
let rhythm = extract_rhythm_pattern(&result);
assert!((rhythm.tempo_bpm.unwrap() - 120.0).abs() < 5.0);
```

## File Characteristics

All WAV files are generated with:
- Sample rate: 44100 Hz
- Channels: Mono (1 channel)
- Bit depth: 16-bit
- Format: PCM WAV

MIDI files use:
- Tempo: 120 BPM (unless specified otherwise)
- Single track
- Standard MIDI format

## Notes

- These files are intentionally small and simple to keep repository size manageable
- Files are tracked in git (exception added to .gitignore)
- Generation script is deterministic - running it multiple times produces identical files
- For more complex test scenarios, consider generating custom samples on-the-fly in tests
