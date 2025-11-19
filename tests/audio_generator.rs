use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::PathBuf;

/// Generate a sine wave audio file for testing
pub fn generate_test_audio(
    frequency: f32,
    duration_secs: f32,
    sample_rate: u32,
    filename: &str,
) -> PathBuf {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let amplitude = i16::MAX as f32 * 0.5;

    let path = PathBuf::from(filename);
    let mut writer = WavWriter::create(&path, spec).unwrap();

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        let sample_int = (sample * amplitude) as i16;
        writer.write_sample(sample_int).unwrap();
    }

    writer.finalize().unwrap();
    path
}

/// Generate a guitar-like plucked note using Karplus-Strong algorithm
#[allow(dead_code)]
pub fn generate_guitar_note(
    frequency: f32,
    duration_secs: f32,
    sample_rate: u32,
    filename: &str,
) -> PathBuf {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let period = (sample_rate as f32 / frequency) as usize;
    let amplitude = i16::MAX as f32 * 0.5;

    let path = PathBuf::from(filename);
    let mut writer = WavWriter::create(&path, spec).unwrap();

    // Initialize delay line with noise
    let mut delay_line: Vec<f32> = (0..period)
        .map(|_| (rand::random::<f32>() - 0.5) * 2.0)
        .collect();

    let decay = 0.996; // Decay factor for realistic sound
    let mut index = 0;

    for _ in 0..num_samples {
        let current = delay_line[index];
        let next = delay_line[(index + 1) % period];
        let new_sample = (current + next) * 0.5 * decay;

        delay_line[index] = new_sample;
        let sample_int = (new_sample * amplitude) as i16;
        writer.write_sample(sample_int).unwrap();

        index = (index + 1) % period;
    }

    writer.finalize().unwrap();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_test_audio() {
        let path = generate_test_audio(440.0, 0.5, 44100, "/tmp/test_audio.wav");
        assert!(path.exists());
        std::fs::remove_file(path).unwrap();
    }
}
