use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Audio buffer size (samples per processing chunk)
    pub buffer_size: usize,

    /// Minimum duration for a note to be considered valid (in seconds)
    pub min_note_duration: f32,

    /// Threshold for pitch detection confidence
    pub pitch_threshold: f32,

    /// MIDI output port name (None for virtual port)
    pub midi_port: Option<String>,

    /// Velocity for MIDI notes (0-127)
    pub velocity: u8,

    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            buffer_size: 2048,
            min_note_duration: 0.05, // 50ms
            pitch_threshold: 0.15,
            midi_port: None,
            velocity: 80,
            verbose: false,
        }
    }
}

impl Config {
    /// Load configuration from JSON file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn to_file(&self, path: &str) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.buffer_size, 2048);
        assert_eq!(config.velocity, 80);
        assert!(!config.verbose);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buffer_size, config.buffer_size);
    }
}
