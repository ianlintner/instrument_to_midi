use log::debug;
use rustfft::{num_complex::Complex, FftPlanner};

const MIN_FREQUENCY: f32 = 80.0; // Low E on guitar (82.41 Hz)
const MAX_FREQUENCY: f32 = 1320.0; // High E on guitar (1319 Hz)

/// Represents a detected pitch with its strength
#[derive(Debug, Clone, Copy)]
pub struct PitchCandidate {
    pub frequency: f32,
    pub magnitude: f32,
    pub midi_note: u8,
}

pub struct PolyphonicPitchDetector {
    sample_rate: f32,
    buffer_size: usize,
    fft_planner: FftPlanner<f32>,
    min_peak_magnitude: f32,
}

impl PolyphonicPitchDetector {
    pub fn new(sample_rate: u32, buffer_size: usize, min_peak_magnitude: f32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            buffer_size,
            fft_planner: FftPlanner::new(),
            min_peak_magnitude,
        }
    }

    /// Detect multiple pitches using FFT-based spectral peak detection
    pub fn detect_pitches(&mut self, samples: &[f32]) -> Vec<PitchCandidate> {
        if samples.len() < self.buffer_size {
            return vec![];
        }

        // Apply Hamming window to reduce spectral leakage
        let windowed: Vec<Complex<f32>> = samples
            .iter()
            .take(self.buffer_size)
            .enumerate()
            .map(|(i, &sample)| {
                let window = 0.54
                    - 0.46
                        * (2.0 * std::f32::consts::PI * i as f32 / (self.buffer_size - 1) as f32)
                            .cos();
                Complex::new(sample * window, 0.0)
            })
            .collect();

        // Perform FFT
        let mut buffer = windowed;
        let fft = self.fft_planner.plan_fft_forward(self.buffer_size);
        fft.process(&mut buffer);

        // Calculate magnitude spectrum
        let magnitudes: Vec<f32> = buffer
            .iter()
            .take(self.buffer_size / 2)
            .map(|c| c.norm())
            .collect();

        // Find spectral peaks
        let peaks = self.find_spectral_peaks(&magnitudes);

        // Convert peaks to pitch candidates
        let mut candidates: Vec<PitchCandidate> = peaks
            .into_iter()
            .filter_map(|(bin, magnitude)| {
                let frequency = bin as f32 * self.sample_rate / self.buffer_size as f32;

                // Filter to guitar frequency range
                if (MIN_FREQUENCY..=MAX_FREQUENCY).contains(&frequency) {
                    let midi_note = Self::frequency_to_midi(frequency);
                    Some(PitchCandidate {
                        frequency,
                        magnitude,
                        midi_note,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Remove harmonic duplicates - keep only fundamental frequencies
        candidates = self.remove_harmonics(candidates);

        // Sort by magnitude (strongest first)
        candidates.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());

        // Limit to maximum 6 simultaneous notes (reasonable for guitar chords)
        candidates.truncate(6);

        if !candidates.is_empty() {
            debug!("Detected {} simultaneous pitches", candidates.len());
            for candidate in &candidates {
                debug!(
                    "  Note: {} ({:.2} Hz), magnitude: {:.2}",
                    Self::midi_to_note_name(candidate.midi_note),
                    candidate.frequency,
                    candidate.magnitude
                );
            }
        }

        candidates
    }

    /// Find spectral peaks in the magnitude spectrum
    fn find_spectral_peaks(&self, magnitudes: &[f32]) -> Vec<(usize, f32)> {
        let mut peaks = Vec::new();

        // Find local maxima
        for i in 1..magnitudes.len() - 1 {
            let current = magnitudes[i];
            let prev = magnitudes[i - 1];
            let next = magnitudes[i + 1];

            // Check if this is a local maximum above threshold
            if current > prev && current > next && current > self.min_peak_magnitude {
                peaks.push((i, current));
            }
        }

        peaks
    }

    /// Remove harmonic duplicates, keeping only fundamental frequencies
    fn remove_harmonics(&self, mut candidates: Vec<PitchCandidate>) -> Vec<PitchCandidate> {
        if candidates.len() <= 1 {
            return candidates;
        }

        // Sort by frequency for easier harmonic detection
        candidates.sort_by(|a, b| a.frequency.partial_cmp(&b.frequency).unwrap());

        let mut fundamentals: Vec<PitchCandidate> = Vec::new();

        for candidate in candidates {
            let mut is_harmonic = false;

            // Check if this frequency is a harmonic of any existing fundamental
            for fundamental in &fundamentals {
                let ratio = candidate.frequency / fundamental.frequency;

                // Check if frequency ratio is close to an integer (harmonic relationship)
                // Allow 5% tolerance for imperfect harmonics
                let nearest_integer = ratio.round();
                if nearest_integer >= 2.0
                    && (ratio - nearest_integer).abs() / nearest_integer < 0.05
                {
                    is_harmonic = true;
                    break;
                }
            }

            if !is_harmonic {
                fundamentals.push(candidate);
            }
        }

        fundamentals
    }

    /// Convert frequency to MIDI note number
    pub fn frequency_to_midi(frequency: f32) -> u8 {
        let note = 69.0 + 12.0 * (frequency / 440.0).log2();
        note.round().clamp(0.0, 127.0) as u8
    }

    /// Get MIDI note name from note number
    pub fn midi_to_note_name(midi_note: u8) -> String {
        let note_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let octave = (midi_note / 12) as i32 - 1;
        let note_index = (midi_note % 12) as usize;
        format!("{}{}", note_names[note_index], octave)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_polyphonic_detector_creation() {
        let detector = PolyphonicPitchDetector::new(44100, 2048, 0.1);
        assert_eq!(detector.sample_rate, 44100.0);
        assert_eq!(detector.buffer_size, 2048);
    }

    #[test]
    fn test_detect_single_pitch() {
        let sample_rate = 44100;
        let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

        // Generate a 440 Hz sine wave (A4)
        let frequency = 440.0;
        let duration = 0.1;
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = vec![0.0; num_samples];

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / sample_rate as f32;
            *sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
        }

        let pitches = detector.detect_pitches(&samples);

        // Should detect at least one pitch
        assert!(!pitches.is_empty());

        // First detected pitch should be close to 440 Hz
        let detected_freq = pitches[0].frequency;
        assert_relative_eq!(detected_freq, frequency, epsilon = 10.0);
    }

    #[test]
    fn test_detect_multiple_pitches() {
        let sample_rate = 44100;
        let mut detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

        // Generate a chord: C4 (261.63 Hz) + E4 (329.63 Hz) + G4 (392 Hz)
        let frequencies = [261.63, 329.63, 392.0];
        let duration = 0.1;
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = vec![0.0; num_samples];

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / sample_rate as f32;
            for &freq in &frequencies {
                *sample += (2.0 * std::f32::consts::PI * freq * t).sin() / frequencies.len() as f32;
            }
        }

        let pitches = detector.detect_pitches(&samples);

        // Should detect multiple pitches
        assert!(
            pitches.len() >= 2,
            "Should detect at least 2 notes in the chord"
        );
    }

    #[test]
    fn test_harmonic_removal() {
        let sample_rate = 44100;
        let detector = PolyphonicPitchDetector::new(sample_rate, 2048, 0.1);

        // Create candidates with fundamental and harmonics
        let candidates = vec![
            PitchCandidate {
                frequency: 220.0, // A3 (fundamental)
                magnitude: 1.0,
                midi_note: 57,
            },
            PitchCandidate {
                frequency: 440.0, // A4 (2nd harmonic)
                magnitude: 0.8,
                midi_note: 69,
            },
            PitchCandidate {
                frequency: 330.0, // E4 (not a harmonic)
                magnitude: 0.9,
                midi_note: 64,
            },
        ];

        let filtered = detector.remove_harmonics(candidates);

        // Should keep A3 and E4, remove A4 (harmonic of A3)
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|c| (c.frequency - 220.0).abs() < 1.0));
        assert!(filtered.iter().any(|c| (c.frequency - 330.0).abs() < 1.0));
    }

    #[test]
    fn test_frequency_to_midi() {
        // A4 = 440 Hz = MIDI note 69
        assert_eq!(PolyphonicPitchDetector::frequency_to_midi(440.0), 69);

        // E2 (low E on guitar) = 82.41 Hz = MIDI note 40
        assert_eq!(PolyphonicPitchDetector::frequency_to_midi(82.41), 40);

        // C4 = 261.63 Hz = MIDI note 60
        assert_eq!(PolyphonicPitchDetector::frequency_to_midi(261.63), 60);
    }

    #[test]
    fn test_midi_to_note_name() {
        assert_eq!(PolyphonicPitchDetector::midi_to_note_name(69), "A4");
        assert_eq!(PolyphonicPitchDetector::midi_to_note_name(60), "C4");
        assert_eq!(PolyphonicPitchDetector::midi_to_note_name(40), "E2");
    }
}
