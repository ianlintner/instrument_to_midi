use log::debug;

const MIN_FREQUENCY: f32 = 80.0; // Low E on guitar (82.41 Hz)
const MAX_FREQUENCY: f32 = 1320.0; // High E on guitar (1319 Hz)

pub struct PitchDetector {
    sample_rate: f32,
    buffer_size: usize,
    threshold: f32,
}

impl PitchDetector {
    pub fn new(sample_rate: u32, buffer_size: usize, threshold: f32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            buffer_size,
            threshold,
        }
    }

    /// Detect pitch using the YIN algorithm
    pub fn detect_pitch(&self, samples: &[f32]) -> Option<f32> {
        if samples.len() < self.buffer_size {
            return None;
        }

        let max_period = (self.sample_rate / MIN_FREQUENCY) as usize;
        let min_period = (self.sample_rate / MAX_FREQUENCY) as usize;

        // Calculate difference function
        let mut diff = vec![0.0; max_period + 1];
        for tau in 1..=max_period {
            for i in 0..(self.buffer_size - max_period) {
                let delta = samples[i] - samples[i + tau];
                diff[tau] += delta * delta;
            }
        }

        // Calculate cumulative mean normalized difference
        let mut cmnd = vec![1.0; max_period + 1];
        cmnd[0] = 1.0;
        let mut running_sum = 0.0;

        for tau in 1..=max_period {
            running_sum += diff[tau];
            if running_sum == 0.0 {
                cmnd[tau] = 1.0;
            } else {
                cmnd[tau] = diff[tau] * tau as f32 / running_sum;
            }
        }

        // Find the first minimum below threshold
        let mut tau = min_period;
        while tau < max_period {
            if cmnd[tau] < self.threshold {
                while tau + 1 < max_period && cmnd[tau + 1] < cmnd[tau] {
                    tau += 1;
                }
                break;
            }
            tau += 1;
        }

        if tau >= max_period {
            return None;
        }

        // Parabolic interpolation for better accuracy
        let better_tau = self.parabolic_interpolation(&cmnd, tau);
        let frequency = self.sample_rate / better_tau;

        // Validate frequency is in guitar range
        if (MIN_FREQUENCY..=MAX_FREQUENCY).contains(&frequency) {
            debug!("Detected frequency: {:.2} Hz", frequency);
            Some(frequency)
        } else {
            None
        }
    }

    /// Parabolic interpolation for sub-sample accuracy
    fn parabolic_interpolation(&self, data: &[f32], index: usize) -> f32 {
        if index == 0 || index >= data.len() - 1 {
            return index as f32;
        }

        let s0 = data[index - 1];
        let s1 = data[index];
        let s2 = data[index + 1];

        let denom = s0 - 2.0 * s1 + s2;
        if denom.abs() < f32::EPSILON {
            return index as f32;
        }
        let adjustment = 0.5 * (s0 - s2) / denom;
        index as f32 + adjustment
    }

    /// Convert frequency to MIDI note number
    pub fn frequency_to_midi(frequency: f32) -> u8 {
        // MIDI note = 69 + 12 * log2(frequency / 440)
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
    fn test_frequency_to_midi() {
        // A4 = 440 Hz = MIDI note 69
        assert_eq!(PitchDetector::frequency_to_midi(440.0), 69);

        // E2 (low E on guitar) = 82.41 Hz = MIDI note 40
        assert_eq!(PitchDetector::frequency_to_midi(82.41), 40);

        // E4 = 329.63 Hz = MIDI note 64
        assert_eq!(PitchDetector::frequency_to_midi(329.63), 64);
    }

    #[test]
    fn test_midi_to_note_name() {
        assert_eq!(PitchDetector::midi_to_note_name(69), "A4");
        assert_eq!(PitchDetector::midi_to_note_name(40), "E2");
        assert_eq!(PitchDetector::midi_to_note_name(60), "C4");
        assert_eq!(PitchDetector::midi_to_note_name(64), "E4");
    }

    #[test]
    fn test_pitch_detector_creation() {
        let detector = PitchDetector::new(44100, 2048, 0.15);
        assert_eq!(detector.sample_rate, 44100.0);
        assert_eq!(detector.buffer_size, 2048);
        assert_eq!(detector.threshold, 0.15);
    }

    #[test]
    fn test_detect_pitch_with_sine_wave() {
        let sample_rate = 44100;
        let detector = PitchDetector::new(sample_rate, 2048, 0.15);

        // Generate a 440 Hz sine wave
        let frequency = 440.0;
        let duration = 0.1; // 100ms
        let num_samples = (sample_rate as f32 * duration) as usize;
        let mut samples = vec![0.0; num_samples];

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / sample_rate as f32;
            *sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
        }

        let detected = detector.detect_pitch(&samples);
        assert!(detected.is_some());

        // Allow 5% error in frequency detection
        if let Some(freq) = detected {
            assert_relative_eq!(freq, frequency, epsilon = frequency * 0.05);
        }
    }
}
