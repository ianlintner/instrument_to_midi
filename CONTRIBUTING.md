# Contributing to instrument_to_midi

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing to the project.

## Development Setup

### Prerequisites

1. **Install Rust** (stable channel):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Install system dependencies**:
   
   **Linux (Ubuntu/Debian)**:
   ```bash
   sudo apt-get update
   sudo apt-get install -y libasound2-dev pkg-config
   ```
   
   **macOS**:
   ```bash
   brew install pkg-config
   ```

3. **Clone the repository**:
   ```bash
   git clone https://github.com/ianlintner/instrument_to_midi.git
   cd instrument_to_midi
   ```

### Building and Testing

```bash
# Build the project
cargo build

# Run tests
cargo test

# Build release version
cargo build --release

# Run the application
cargo run -- list-ports
cargo run -- stream
```

## Code Quality Standards

### Before Committing

Always run these checks before committing:

```bash
# 1. Format code
cargo fmt

# 2. Check formatting (CI requirement)
cargo fmt -- --check

# 3. Run clippy (CI requirement - must pass with -D warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run tests (CI requirement)
cargo test

# 5. Verify release build works
cargo build --release
```

### Formatting

We use `rustfmt` with the configuration in `rustfmt.toml`. Key points:
- Maximum line width: 100 characters
- 4-space indentation
- Unix-style newlines
- Automatic import reordering

### Linting

We use `clippy` with strict settings:
- All warnings are treated as errors in CI
- Custom allow attributes are used sparingly and only when necessary
- Focus on idiomatic Rust code

## Pull Request Process

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**:
   - Write clear, self-documenting code
   - Add tests for new functionality
   - Update documentation if needed

3. **Test your changes**:
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test
   ```

4. **Commit with descriptive messages**:
   ```bash
   git commit -m "Add feature: description of what you did"
   ```

5. **Push and create a pull request**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Ensure CI passes**:
   - All formatting checks pass
   - All clippy lints pass
   - All tests pass
   - Security audit passes

## Testing Guidelines

### Unit Tests

- Place unit tests in the same file as the code they test, in a `#[cfg(test)]` module
- Test both success and error cases
- Use descriptive test names that explain what's being tested

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_to_midi_converts_a440_correctly() {
        assert_eq!(PitchDetector::frequency_to_midi(440.0), 69);
    }
}
```

### Integration Tests

- Place integration tests in the `tests/` directory
- Test complete workflows and module interactions
- Generate test data as needed (see `tests/audio_generator.rs`)

### Test Coverage

- Aim for high test coverage, especially for critical paths
- Test edge cases and error conditions
- Tests should be fast and deterministic

## Code Style Guidelines

### General Principles

- **Clarity over cleverness**: Write code that's easy to understand
- **DRY (Don't Repeat Yourself)**: Extract common patterns
- **KISS (Keep It Simple)**: Avoid unnecessary complexity
- **Documentation**: Document public APIs and complex logic

### Rust-Specific Guidelines

1. **Error Handling**:
   - Use `Result<T, E>` for fallible operations
   - Use `anyhow::Result` for application code
   - Use `thiserror` for library errors
   - Provide context with `.context()` when appropriate

2. **Ownership**:
   - Prefer borrowing over cloning when possible
   - Use `&str` for string parameters, `String` for owned strings
   - Avoid unnecessary `Box<T>` or `Rc<T>`

3. **Async/Await**:
   - Currently not used in the core audio processing loop (latency-sensitive)
   - Available for future network or I/O operations

4. **Comments**:
   - Write doc comments (`///`) for public items
   - Use inline comments sparingly, only for non-obvious logic
   - Keep comments up-to-date with code changes

### Module Organization

```
src/
├── main.rs           # CLI entry point
├── audio/            # Audio input module
├── pitch/            # Pitch detection module
├── midi/             # MIDI output module
├── config/           # Configuration management
└── processor.rs      # Stream processing coordinator
```

## Documentation

### Code Documentation

- All public functions, structs, and modules should have doc comments
- Include examples in doc comments where helpful
- Document panics, errors, and safety requirements

Example:
```rust
/// Detect pitch using the YIN algorithm.
///
/// # Arguments
///
/// * `samples` - Audio samples to analyze (should be >= buffer_size)
///
/// # Returns
///
/// `Some(frequency)` if a clear pitch is detected, `None` otherwise
///
/// # Examples
///
/// ```
/// let detector = PitchDetector::new(44100, 2048);
/// let frequency = detector.detect_pitch(&samples);
/// ```
pub fn detect_pitch(&self, samples: &[f32]) -> Option<f32> {
    // ...
}
```

### README Updates

When adding new features, update:
- Feature list
- Usage examples
- Configuration options
- Troubleshooting section (if relevant)

## Continuous Integration

Our CI pipeline runs on every push and PR:

1. **Lint Job**:
   - Checks formatting with `rustfmt`
   - Runs `clippy` with strict settings

2. **Build Job**:
   - Builds release binary
   - Uploads artifacts

3. **Test Job**:
   - Runs all unit and integration tests
   - Uploads example audio files

4. **Security Audit Job**:
   - Checks for known vulnerabilities with `cargo-audit`

All jobs must pass before merging.

## Project Structure

```
.
├── .github/workflows/    # CI/CD configuration
├── src/                  # Source code
│   ├── audio/           # Audio input handling
│   ├── pitch/           # Pitch detection (YIN algorithm)
│   ├── midi/            # MIDI output handling
│   ├── config/          # Configuration management
│   ├── processor.rs     # Stream processing
│   └── main.rs          # CLI application
├── tests/               # Integration tests
│   ├── audio_generator.rs   # Test audio generation
│   └── integration_test.rs  # Integration tests
├── examples/audio/      # Example audio files (generated by tests)
├── Cargo.toml          # Project configuration
├── rustfmt.toml        # Formatting configuration
├── README.md           # Project documentation
└── CONTRIBUTING.md     # This file
```

## Getting Help

- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- Provide detailed information: OS, Rust version, error messages
- Include minimal reproducible examples when reporting bugs

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
