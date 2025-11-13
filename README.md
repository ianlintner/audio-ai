# Audio-AI

Audio-AI is a Rust-based project for real-time audio processing and analysis with AI-powered feedback. It provides intelligent comparison of musical performances with optimized data transformation for LLM analysis.

## Features
- Real-time audio streaming and analysis
- Musical note extraction (Hz → A4, C#5, etc.)
- Rhythm pattern and tempo analysis
- **Advanced comparison**: Compare student recordings vs reference performances
- **Optimized LLM integration**: 90% token reduction with better musical context
- **Statistical pre-processing**: Validate and analyze before AI submission
- Modular processor design
- Extensible for AI/ML integration

## New in v2.0
- 🎵 **Note sequence extraction**: Converts raw audio to musical patterns
- 📊 **Comparison metrics**: Pitch, timing, rhythm, and note accuracy
- 🚀 **90% smaller context windows**: Optimized JSON export for AI
- ✅ **Pre-processing validation**: Catches errors before expensive API calls
- 🎯 **Specific error detection**: Identifies missed notes, wrong pitches, timing issues

## Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Cargo (comes with Rust)
- System dependencies:
  - ALSA development libraries: `sudo apt-get install libasound2-dev`
  - Aubio development libraries: `sudo apt-get install libaubio-dev`
  - pkg-config: `sudo apt-get install pkg-config`
- Docker (optional, for containerized builds)
- OpenAI API key for AI feedback (optional)

### Build
```bash
cargo build --release
```

### Usage

#### Analyze a Single File
```bash
cargo run --release -- path/to/recording.wav
```

This will:
- Extract musical features (notes, tempo, rhythm)
- Export both legacy and optimized JSON formats
- Send analysis to OpenAI for feedback (if API key is set)

#### Compare Two Recordings
```bash
cargo run --release -- reference.wav student.wav
```

This will:
- Analyze both recordings
- Compute comparison metrics (pitch, timing, rhythm accuracy)
- Display quick summary in terminal
- Export detailed comparison JSON
- Send to OpenAI for personalized feedback

Example output:
```
=== Quick Summary ===
Overall Similarity: 78.5%
Note Accuracy: 85.0%
Pitch Accuracy: 72.3%
Timing Accuracy: 80.1%
Rhythm Accuracy: 76.8%

Missed Notes (2): ["E4 at 1.50s", "A4 at 3.20s"]
Extra Notes (1): ["B4 at 2.10s"]
```

#### Real-time Streaming Analysis
```bash
cargo run --release -- --stream
```

This will:
- Start capturing audio from your microphone
- Detect notes in real-time
- Display them as they're played

### Environment Variables
Create a `.env` file in the project root:
```bash
# Required for AI feedback
OPENAI_API_KEY=your_api_key_here

# Optional: customize the OpenAI model (defaults to gpt-4o-mini)
OPENAI_MODEL=gpt-4o
```

Available models:
- `gpt-4o-mini` (default) - Fast and cost-effective
- `gpt-4o` - More capable, higher quality
- `gpt-4-turbo` - Alternative high-quality option

### Test
```bash
cargo test
```

All tests run without requiring an OpenAI API key. AI integration tests use stubbed/mocked responses to validate functionality without making actual API calls.

### Test Data
The repository includes generated sample audio files for integration testing. These files are located in `tests/data/` and include:
- Single tone WAV files at various frequencies
- Musical sequences (scales, melodies)
- Rhythm patterns for tempo testing
- MIDI files for MIDI support testing

To regenerate test samples:
```bash
cd tests
python3 generate_test_samples.py
```

See [tests/data/README.md](tests/data/README.md) for details on the test samples.

### AI Integration Testing
The project includes comprehensive integration tests for AI functionality using stubbed responses. This allows:
- Testing AI integration without API keys
- Consistent, reproducible test results
- Fast test execution without network calls
- Validation of different feedback scenarios (excellent, poor, constructive)

See `tests/ai_integration_tests.rs` for examples of how to test AI functionality with mocked responses.

## Development

### Linting & Formatting
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
```

### Running Tests
```bash
cargo test
```

## Architecture

See [docs/architecture.md](docs/architecture.md) for detailed architecture documentation.

### Key Modules

- **`audio_analysis.rs`**: Audio feature extraction (pitch, tempo, onsets)
- **`comparison.rs`**: Musical pattern extraction and comparison metrics
- **`processor.rs`**: Data transformation and optimized JSON export
- **`ai_client.rs`**: AI integration with OpenAI API and mock client for testing
- **`streaming.rs`**: Real-time audio capture and analysis
- **`main.rs`**: CLI interface with single-file and comparison modes

### Data Flow

```
Audio File(s)
    ↓
Audio Analysis (pitch, tempo, onsets)
    ↓
Musical Feature Extraction (notes, rhythm)
    ↓
[Optional] Comparison Metrics
    ↓
Optimized JSON Export (90% token reduction)
    ↓
AI Analysis & Feedback
```

## Example Use Cases

### 1. Guitar Practice Assistant
```bash
# Record yourself playing
arecord -d 10 -f cd student.wav

# Compare to reference
cargo run -- reference-song.wav student.wav
```

Get AI feedback on:
- Which notes you missed
- How far off-pitch you were
- Timing accuracy
- Rhythm consistency

### 2. Music Education
Teachers can:
- Upload reference performances
- Students submit their attempts
- Automated comparison identifies issues
- AI generates personalized feedback

### 3. Music Transcription
```bash
cargo run -- song.wav
```

Outputs:
- Note sequences
- Timing information
- Rhythm patterns
- Musical analysis

## API Integration

The optimized JSON export is designed for LLM consumption:

```json
{
  "format_version": "2.0-optimized",
  "pitch_statistics": {
    "average_note": "A4",
    "pitch_range_semitones": 12,
    "pitch_stability": 0.85
  },
  "notes": {
    "total_notes": 10,
    "unique_notes": ["E4", "A4", "D5"],
    "note_sequence": [...]
  },
  "comparison": {
    "overall_similarity": "85.5%",
    "scores": {...},
    "errors": {...}
  }
}
```

**Benefits:**
- 90% smaller than raw data
- Musical context included
- Pre-computed metrics
- Actionable error details

## Docker

Build the Docker image:
```bash
docker build -t audio-ai .
```

Run the container:
```bash
docker run --rm -it audio-ai
```

## Contributing
1. Fork the repository
2. Create a feature branch
3. Commit changes
4. Open a Pull Request

## License
MIT License
