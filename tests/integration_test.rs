mod audio_generator;

use audio_generator::generate_guitar_note;
use std::fs;

#[test]
fn test_generate_example_audio_files() {
    // Create examples directory if it doesn't exist
    fs::create_dir_all("examples/audio").unwrap();

    // Generate guitar notes for standard tuning
    let sample_rate = 44100;
    let duration = 1.0;

    // Low E (82.41 Hz)
    generate_guitar_note(
        82.41,
        duration,
        sample_rate,
        "examples/audio/guitar_low_e.wav",
    );

    // A (110 Hz)
    generate_guitar_note(110.0, duration, sample_rate, "examples/audio/guitar_a.wav");

    // D (146.83 Hz)
    generate_guitar_note(146.83, duration, sample_rate, "examples/audio/guitar_d.wav");

    // G (196 Hz)
    generate_guitar_note(196.0, duration, sample_rate, "examples/audio/guitar_g.wav");

    // B (246.94 Hz)
    generate_guitar_note(246.94, duration, sample_rate, "examples/audio/guitar_b.wav");

    // High E (329.63 Hz)
    generate_guitar_note(
        329.63,
        duration,
        sample_rate,
        "examples/audio/guitar_high_e.wav",
    );

    // Verify files were created
    assert!(std::path::Path::new("examples/audio/guitar_low_e.wav").exists());
    assert!(std::path::Path::new("examples/audio/guitar_a.wav").exists());
    assert!(std::path::Path::new("examples/audio/guitar_d.wav").exists());
    assert!(std::path::Path::new("examples/audio/guitar_g.wav").exists());
    assert!(std::path::Path::new("examples/audio/guitar_b.wav").exists());
    assert!(std::path::Path::new("examples/audio/guitar_high_e.wav").exists());
}
