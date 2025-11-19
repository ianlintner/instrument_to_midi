use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use crossbeam_channel::Sender;
use log::{debug, info};

pub struct AudioInput {
    device: Device,
    config: StreamConfig,
}

impl AudioInput {
    /// Create a new AudioInput instance with the default input device
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device available")?;

        info!("Using input device: {}", device.name()?);

        let config = device
            .default_input_config()
            .context("Failed to get default input config")?
            .into();

        Ok(Self { device, config })
    }

    /// Start streaming audio samples to the provided channel
    pub fn start_stream(&self, tx: Sender<Vec<f32>>) -> Result<Stream> {
        let config = self.config.clone();
        debug!("Audio config: {:?}", config);

        let stream = self.device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Send audio samples through the channel
                let samples: Vec<f32> = data.to_vec();
                if let Err(e) = tx.send(samples) {
                    log::error!("Failed to send audio samples: {}", e);
                }
            },
            |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        info!("Audio stream started");
        Ok(stream)
    }

    /// Get the sample rate of the audio input
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    /// Get the number of channels
    #[allow(dead_code)]
    pub fn channels(&self) -> u16 {
        self.config.channels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_input_creation() {
        // This test might fail on systems without audio devices
        // but that's expected behavior
        let result = AudioInput::new();
        if let Ok(audio) = result {
            assert!(audio.sample_rate() > 0);
            assert!(audio.channels() > 0);
        }
    }
}
