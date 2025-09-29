# Audio-AI

Audio-AI is a Rust-based project for real-time audio processing and analysis. It leverages the `cpal` crate for audio streaming and provides modular components for analysis and processing.

## Features
- Real-time audio streaming
- Audio analysis utilities
- Modular processor design
- Extensible for AI/ML integration

## Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- Cargo (comes with Rust)
- Docker (optional, for containerized builds)

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run --release
```

### Test
```bash
cargo test
```

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
