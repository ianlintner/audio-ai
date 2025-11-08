#!/usr/bin/env python3
"""
Generate sample audio files for integration testing.

This script creates various WAV and MIDI files with known characteristics
for testing the audio-ai application's analysis and comparison features.
"""

import os
import sys
import subprocess
from midiutil import MIDIFile

# Output directory for generated samples
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "data")

def ensure_output_dir():
    """Create output directory if it doesn't exist."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    print(f"Output directory: {OUTPUT_DIR}")

def generate_simple_tone(frequency, duration, output_file):
    """Generate a simple sine wave tone using sox."""
    filepath = os.path.join(OUTPUT_DIR, output_file)
    cmd = [
        "sox", "-n", "-r", "44100", "-c", "1", "-b", "16", filepath,
        "synth", str(duration), "sine", str(frequency)
    ]
    subprocess.run(cmd, check=True)
    print(f"✓ Generated: {output_file} ({frequency}Hz, {duration}s)")

def generate_note_sequence(notes_with_duration, output_file):
    """Generate a sequence of notes using sox.
    
    Args:
        notes_with_duration: List of (frequency_hz, duration_seconds) tuples
        output_file: Output filename
    """
    filepath = os.path.join(OUTPUT_DIR, output_file)
    temp_files = []
    
    try:
        # Generate each note as a separate file
        for i, (freq, dur) in enumerate(notes_with_duration):
            temp_file = f"/tmp/note_{i}.wav"
            cmd = [
                "sox", "-n", "-r", "44100", "-c", "1", "-b", "16", temp_file,
                "synth", str(dur), "sine", str(freq)
            ]
            subprocess.run(cmd, check=True)
            temp_files.append(temp_file)
        
        # Concatenate all notes
        cmd = ["sox"] + temp_files + [filepath]
        subprocess.run(cmd, check=True)
        
        note_names = [f"{freq}Hz" for freq, _ in notes_with_duration]
        print(f"✓ Generated: {output_file} (notes: {', '.join(note_names)})")
    finally:
        # Clean up temp files
        for temp_file in temp_files:
            if os.path.exists(temp_file):
                os.remove(temp_file)

def generate_midi_file(notes, output_file, tempo=120):
    """Generate a MIDI file with specified notes.
    
    Args:
        notes: List of (midi_note, start_beat, duration_beats) tuples
        output_file: Output filename
        tempo: Tempo in BPM
    """
    filepath = os.path.join(OUTPUT_DIR, output_file)
    
    # Create MIDI file with 1 track
    midi = MIDIFile(1)
    track = 0
    channel = 0
    time = 0
    midi.addTrackName(track, time, "Test Track")
    midi.addTempo(track, time, tempo)
    
    # Add notes
    volume = 100
    for midi_note, start_beat, duration in notes:
        midi.addNote(track, channel, midi_note, start_beat, duration, volume)
    
    # Write to file
    with open(filepath, "wb") as f:
        midi.writeFile(f)
    
    note_names = [str(n[0]) for n in notes]
    print(f"✓ Generated: {output_file} (MIDI notes: {', '.join(note_names)}, tempo: {tempo})")

def convert_midi_to_wav(midi_file, wav_file):
    """Convert MIDI to WAV using fluidsynth or timidity if available."""
    wav_path = os.path.join(OUTPUT_DIR, wav_file)
    
    # Try fluidsynth first (if available with a soundfont)
    # Since we may not have soundfonts, we'll skip this for now
    # Instead, document that MIDI files are available for testing MIDI support
    print(f"ℹ MIDI file generated: {midi_file} (WAV conversion requires fluidsynth + soundfont)")

def main():
    """Generate all test sample files."""
    ensure_output_dir()
    print("\n=== Generating Audio Test Samples ===\n")
    
    # 1. Single tone samples at different frequencies
    print("1. Single tone samples:")
    generate_simple_tone(440.0, 2.0, "tone_a4_440hz.wav")  # A4
    generate_simple_tone(261.63, 2.0, "tone_c4_262hz.wav")  # C4
    generate_simple_tone(329.63, 2.0, "tone_e4_330hz.wav")  # E4
    generate_simple_tone(523.25, 2.0, "tone_c5_523hz.wav")  # C5
    
    # 2. Note sequences for comparison testing
    print("\n2. Note sequences:")
    # Simple C major scale: C4, D4, E4, F4, G4, A4, B4, C5
    c_major_scale = [
        (261.63, 0.5),  # C4
        (293.66, 0.5),  # D4
        (329.63, 0.5),  # E4
        (349.23, 0.5),  # F4
        (392.00, 0.5),  # G4
        (440.00, 0.5),  # A4
        (493.88, 0.5),  # B4
        (523.25, 0.5),  # C5
    ]
    generate_note_sequence(c_major_scale, "scale_c_major.wav")
    
    # Simple melody: A4, A4, B4, C5, C5, B4, A4
    simple_melody = [
        (440.0, 0.5),   # A4
        (440.0, 0.5),   # A4
        (493.88, 0.5),  # B4
        (523.25, 0.5),  # C5
        (523.25, 0.5),  # C5
        (493.88, 0.5),  # B4
        (440.0, 0.5),   # A4
    ]
    generate_note_sequence(simple_melody, "melody_simple.wav")
    
    # Same melody with slight timing variations (for comparison testing)
    simple_melody_variant = [
        (440.0, 0.55),   # A4 - slightly longer
        (440.0, 0.45),   # A4 - slightly shorter
        (493.88, 0.52),  # B4
        (523.25, 0.48),  # C5
        (523.25, 0.5),   # C5
        (493.88, 0.5),   # B4
        (440.0, 0.5),    # A4
    ]
    generate_note_sequence(simple_melody_variant, "melody_simple_timing_variant.wav")
    
    # Same melody with pitch variations (slightly out of tune)
    simple_melody_pitch_variant = [
        (440.0, 0.5),    # A4 - correct
        (445.0, 0.5),    # ~A4 - slightly sharp
        (493.88, 0.5),   # B4 - correct
        (528.0, 0.5),    # ~C5 - slightly sharp
        (523.25, 0.5),   # C5 - correct
        (490.0, 0.5),    # ~B4 - slightly flat
        (440.0, 0.5),    # A4 - correct
    ]
    generate_note_sequence(simple_melody_pitch_variant, "melody_simple_pitch_variant.wav")
    
    # 3. Rhythm pattern samples with consistent tempo
    print("\n3. Rhythm patterns:")
    # Quarter notes at 120 BPM (0.5s each)
    quarter_notes = [(440.0, 0.5) for _ in range(8)]
    generate_note_sequence(quarter_notes, "rhythm_quarter_notes_120bpm.wav")
    
    # Eighth notes at 120 BPM (0.25s each)
    eighth_notes = [(440.0, 0.25) for _ in range(16)]
    generate_note_sequence(eighth_notes, "rhythm_eighth_notes_120bpm.wav")
    
    # 4. Generate MIDI files for MIDI-specific testing
    print("\n4. MIDI files:")
    # Simple C major chord
    midi_notes_chord = [
        (60, 0, 2),  # C4
        (64, 0, 2),  # E4
        (67, 0, 2),  # G4
    ]
    generate_midi_file(midi_notes_chord, "midi_c_major_chord.mid", tempo=120)
    
    # Simple melody in MIDI
    midi_notes_melody = [
        (69, 0.0, 0.5),   # A4
        (69, 0.5, 0.5),   # A4
        (71, 1.0, 0.5),   # B4
        (72, 1.5, 0.5),   # C5
        (72, 2.0, 0.5),   # C5
        (71, 2.5, 0.5),   # B4
        (69, 3.0, 0.5),   # A4
    ]
    generate_midi_file(midi_notes_melody, "midi_simple_melody.mid", tempo=120)
    
    # C major scale in MIDI
    midi_scale = [
        (60, 0.0, 0.5),  # C4
        (62, 0.5, 0.5),  # D4
        (64, 1.0, 0.5),  # E4
        (65, 1.5, 0.5),  # F4
        (67, 2.0, 0.5),  # G4
        (69, 2.5, 0.5),  # A4
        (71, 3.0, 0.5),  # B4
        (72, 3.5, 0.5),  # C5
    ]
    generate_midi_file(midi_scale, "midi_c_major_scale.mid", tempo=120)
    
    print("\n=== Sample Generation Complete ===")
    print(f"\nAll files saved to: {OUTPUT_DIR}")
    print("\nGenerated files:")
    for file in sorted(os.listdir(OUTPUT_DIR)):
        filepath = os.path.join(OUTPUT_DIR, file)
        size = os.path.getsize(filepath)
        print(f"  - {file} ({size:,} bytes)")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
