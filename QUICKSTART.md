# Quick Start Guide

Get up and running with instrument_to_midi in 5 minutes!

## Prerequisites

Install system dependencies:

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev pkg-config
```

**macOS:**
```bash
brew install pkg-config
```

**Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Installation

```bash
# Clone the repository
git clone https://github.com/ianlintner/instrument_to_midi.git
cd instrument_to_midi

# Build (takes 1-2 minutes first time)
cargo build --release
```

## Usage

### 1. Check Available MIDI Ports

```bash
cargo run --release -- list-ports
```

This will show all available MIDI ports on your system.

### 2. Start Converting Guitar to MIDI

**Option A: Use Virtual MIDI Port (Linux default)**
```bash
cargo run --release -- stream
```

**Option B: Use Specific MIDI Port**
```bash
cargo run --release -- stream --port "Your MIDI Port Name"
```

**Option C: With Custom Settings**
```bash
cargo run --release -- stream --buffer-size 1024 --velocity 90 --verbose
```

**Option D: Record to MIDI File**
```bash
# Record with auto-generated filename (recording_YYYYMMDD_HHMMSS.mid)
cargo run --release -- stream --record

# Record to specific file
cargo run --release -- stream --record --output my_performance.mid
```

### 3. Connect Your Guitar

1. Plug your guitar into your audio interface or microphone
2. Make sure the input is selected as your default recording device
3. Start the application
4. Play your guitar!

## Testing

To verify everything works without a guitar:

```bash
# Run tests (generates example audio files)
cargo test

# Check the generated files
ls -lh examples/audio/
```

## Troubleshooting

**No audio input detected:**
- Check your audio interface is connected
- Verify input permissions on your OS
- Try running with `--verbose` flag for debug info

**High latency:**
- Reduce buffer size: `--buffer-size 1024` or `--buffer-size 512`
- Trade-off: Lower buffer = less accuracy for low notes

**MIDI port not found:**
- On Linux, you may need to create a virtual port: `modprobe snd-virmidi`
- Use `list-ports` command to see available ports
- Check MIDI connections in your DAW

## Next Steps

- Read [README.md](README.md) for detailed documentation
- Check [CONTRIBUTING.md](CONTRIBUTING.md) to contribute
- Customize settings with `config.example.json`

## Quick Tips

1. **Lower latency:** Reduce buffer size (512-1024) but expect less accuracy on low notes
2. **Better accuracy:** Increase buffer size (2048-4096) but expect higher latency
3. **Clean signal:** Use direct input or audio interface for best results
4. **MIDI velocity:** Adjust with `--velocity` flag (0-127)
5. **Debug:** Use `--verbose` flag to see detected frequencies
6. **Record sessions:** Use `--record` flag to save MIDI files for later use in DAWs

## Example: Complete Setup

```bash
# Install dependencies (Linux)
sudo apt-get install -y libasound2-dev pkg-config

# Clone and build
git clone https://github.com/ianlintner/instrument_to_midi.git
cd instrument_to_midi
cargo build --release

# Run tests to verify
cargo test

# Create config file
cargo run --release -- generate-config my-config.json

# Start streaming (edit config as needed)
cargo run --release -- stream --config my-config.json --verbose
```

Now play your guitar and watch the MIDI notes appear in your DAW or synth!
