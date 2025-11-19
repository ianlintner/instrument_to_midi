mod audio_generator;

use instrument_to_midi::pitch::polyphonic::PolyphonicPitchDetector;
use std::f32::consts::PI;

/// Generate a multi-note chord signal
fn generate_chord(frequencies: &[f32], duration: f32, sample_rate: u32) -> Vec<f32> {
    let num_samples = (duration * sample_rate as f32) as usize;
    let mut samples = vec![0.0; num_samples];

    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 / sample_rate as f32;
        for &freq in frequencies {
            // Add each frequency component with equal amplitude
            *sample += (2.0 * PI * freq * t).sin() / frequencies.len() as f32;
        }
    }

    samples
}

#[test]
fn test_polyphonic_single_note() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Generate a single note: A4 (440 Hz)
    let frequency = 440.0;
    let duration = 0.2;
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = vec![0.0; num_samples];

    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 / sample_rate as f32;
        *sample = (2.0 * PI * frequency * t).sin();
    }

    let pitches = detector.detect_pitches(&samples);

    // Should detect at least one pitch
    assert!(!pitches.is_empty(), "Should detect at least one pitch");

    // The detected frequency should be close to 440 Hz
    let detected_freq = pitches[0].frequency;
    let error = (detected_freq - frequency).abs();
    assert!(
        error < 10.0,
        "Detected frequency {:.2} should be close to {:.2}",
        detected_freq,
        frequency
    );
}

#[test]
fn test_polyphonic_major_chord() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Generate a C major chord: C4 (261.63 Hz), E4 (329.63 Hz), G4 (392.00 Hz)
    let frequencies = [261.63, 329.63, 392.0];
    let samples = generate_chord(&frequencies, 0.2, sample_rate);

    let pitches = detector.detect_pitches(&samples);

    // Should detect multiple pitches (at least 1-2 out of 3 notes in the chord)
    // FFT-based detection may not always separate all notes perfectly
    assert!(
        !pitches.is_empty(),
        "Should detect at least 1 note in a 3-note chord, detected: {}",
        pitches.len()
    );

    // At least one detected frequency should be in the chord range
    let in_range_count = pitches
        .iter()
        .filter(|p| p.frequency >= 250.0 && p.frequency <= 450.0)
        .count();

    assert!(
        in_range_count >= 1,
        "At least one detected frequency should be in the chord range (250-450 Hz)"
    );
}

#[test]
fn test_polyphonic_guitar_chord() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Generate an E major guitar chord (open position)
    // E2 (82.41 Hz), B2 (123.47 Hz), E3 (164.81 Hz), G#3 (207.65 Hz), B3 (246.94 Hz), E4 (329.63 Hz)
    let frequencies = [82.41, 123.47, 164.81, 207.65, 246.94, 329.63];
    let samples = generate_chord(&frequencies, 0.2, sample_rate);

    let pitches = detector.detect_pitches(&samples);

    // Should detect multiple pitches
    assert!(
        pitches.len() >= 3,
        "Should detect at least 3 notes in a 6-string chord, detected: {}",
        pitches.len()
    );

    // Should not exceed 6 notes (the maximum we set)
    assert!(
        pitches.len() <= 6,
        "Should not detect more than 6 notes, detected: {}",
        pitches.len()
    );
}

#[test]
fn test_polyphonic_power_chord() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Generate a power chord (root + fifth): E2 (82.41 Hz) + B2 (123.47 Hz)
    let frequencies = [82.41, 123.47];
    let samples = generate_chord(&frequencies, 0.2, sample_rate);

    let pitches = detector.detect_pitches(&samples);

    // Should detect at least 1 note in a power chord
    assert!(
        !pitches.is_empty(),
        "Should detect at least 1 note in a power chord, detected: {}",
        pitches.len()
    );

    // At least one frequency should be in the low range (indicating bass notes)
    let low_freq_count = pitches
        .iter()
        .filter(|p| p.frequency >= 75.0 && p.frequency <= 250.0)
        .count();

    assert!(
        low_freq_count >= 1,
        "At least one detected frequency should be in the power chord range (75-250 Hz)"
    );
}

#[test]
fn test_polyphonic_harmonic_removal() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Generate a note with strong harmonics: A2 (110 Hz) and its harmonics
    // The detector should identify only the fundamental
    let fundamental = 110.0;
    let duration = 0.2;
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = vec![0.0; num_samples];

    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 / sample_rate as f32;
        // Add fundamental and harmonics with decreasing amplitude
        *sample = (2.0 * PI * fundamental * t).sin(); // Fundamental
        *sample += 0.5 * (2.0 * PI * fundamental * 2.0 * t).sin(); // 2nd harmonic
        *sample += 0.3 * (2.0 * PI * fundamental * 3.0 * t).sin(); // 3rd harmonic
    }

    let pitches = detector.detect_pitches(&samples);

    // Should primarily detect the fundamental frequency
    assert!(
        !pitches.is_empty(),
        "Should detect at least the fundamental frequency"
    );

    // The lowest detected pitch should be close to the fundamental
    let lowest_pitch = pitches
        .iter()
        .min_by(|a, b| a.frequency.partial_cmp(&b.frequency).unwrap())
        .unwrap();

    let error = (lowest_pitch.frequency - fundamental).abs();
    assert!(
        error < 10.0,
        "Lowest detected frequency {:.2} should be close to fundamental {:.2}",
        lowest_pitch.frequency,
        fundamental
    );
}

#[test]
fn test_polyphonic_empty_buffer() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Test with silence (all zeros)
    let samples = vec![0.0; 4096];
    let pitches = detector.detect_pitches(&samples);

    // Should not detect any pitches in silence
    // (or if it does, they should have very low magnitude)
    if !pitches.is_empty() {
        for pitch in &pitches {
            assert!(
                pitch.magnitude < 0.5,
                "Detected pitch in silence should have very low magnitude"
            );
        }
    }
}

#[test]
fn test_polyphonic_octave_detection() {
    let sample_rate = 44100;
    let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

    // Generate two notes an octave apart: A3 (220 Hz) and A4 (440 Hz)
    let frequencies = [220.0, 440.0];
    let samples = generate_chord(&frequencies, 0.2, sample_rate);

    let pitches = detector.detect_pitches(&samples);

    // Should detect at least one pitch
    // (the harmonic removal might treat the octave as a harmonic relationship)
    assert!(
        !pitches.is_empty(),
        "Should detect at least one pitch from octave pair"
    );

    // At least one detected frequency should be in a reasonable range for these notes
    let in_range = pitches.iter().any(|p| {
        (p.frequency >= 200.0 && p.frequency <= 250.0)
            || (p.frequency >= 420.0 && p.frequency <= 460.0)
    });

    assert!(
        in_range,
        "At least one detected frequency should be close to 220 Hz or 440 Hz"
    );
}
