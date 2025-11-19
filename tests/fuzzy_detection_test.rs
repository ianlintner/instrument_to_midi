mod audio_generator;

use audio_generator::generate_guitar_note;
use std::fs;

/// Test that example audio files are generated correctly for fuzzy detection
#[test]
fn test_example_audio_generation() {
    // Create examples directory if it doesn't exist
    fs::create_dir_all("examples/audio").unwrap();

    // Generate a test audio file
    let sample_rate = 44100;
    let duration = 0.5;
    generate_guitar_note(
        261.63,
        duration,
        sample_rate,
        "examples/audio/test_fuzzy.wav",
    );

    // Verify file was created
    assert!(std::path::Path::new("examples/audio/test_fuzzy.wav").exists());

    // Clean up
    let _ = fs::remove_file("examples/audio/test_fuzzy.wav");
}
