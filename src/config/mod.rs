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

    /// Enable fuzzy note detection with learning
    #[serde(default = "default_fuzzy_enabled")]
    pub fuzzy_enabled: bool,

    /// Confidence threshold for fuzzy logic (notes below this use fuzzy resolution)
    #[serde(default = "default_fuzzy_threshold")]
    pub fuzzy_threshold: f32,

    /// Confidence threshold to consider a note "clear" for learning
    #[serde(default = "default_clear_threshold")]
    pub clear_threshold: f32,

    /// Maximum number of recent notes to track for pattern detection
    #[serde(default = "default_max_recent_notes")]
    pub max_recent_notes: usize,

    /// Enable MIDI recording to file
    #[serde(default)]
    pub record_enabled: bool,

    /// Output file path for MIDI recording (None = auto-generate based on timestamp)
    #[serde(default)]
    pub record_output: Option<String>,
    /// Enable pitch bend for vibrato, trills, and whammy effects
    #[serde(default = "default_pitch_bend_enabled")]
    pub pitch_bend_enabled: bool,

    /// Range in semitones for pitch bend (typically 2 or 12)
    #[serde(default = "default_pitch_bend_range")]
    pub pitch_bend_range: f32,

    /// Enable polyphonic pitch detection (detect multiple simultaneous notes)
    #[serde(default)]
    pub polyphonic_enabled: bool,

    /// Minimum peak magnitude for polyphonic detection (higher = less sensitive)
    #[serde(default = "default_polyphonic_threshold")]
    pub polyphonic_threshold: f32,
}

fn default_fuzzy_enabled() -> bool {
    true
}

fn default_fuzzy_threshold() -> f32 {
    0.7
}

fn default_clear_threshold() -> f32 {
    0.8
}

fn default_max_recent_notes() -> usize {
    20
}

fn default_pitch_bend_enabled() -> bool {
    true
}

fn default_pitch_bend_range() -> f32 {
    2.0
}

fn default_polyphonic_threshold() -> f32 {
    0.2
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
            fuzzy_enabled: default_fuzzy_enabled(),
            fuzzy_threshold: default_fuzzy_threshold(),
            clear_threshold: default_clear_threshold(),
            max_recent_notes: default_max_recent_notes(),
            record_enabled: false,
            record_output: None,
            pitch_bend_enabled: default_pitch_bend_enabled(),
            pitch_bend_range: default_pitch_bend_range(),
            polyphonic_enabled: false,
            polyphonic_threshold: default_polyphonic_threshold(),
        }
    }
}

impl Config {
    /// Load configuration from JSON file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn to_file(&self, path: &str) -> anyhow::Result<()> {
        self.validate()?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Validate configuration parameters
    fn validate(&self) -> anyhow::Result<()> {
        if self.fuzzy_enabled && self.clear_threshold < self.fuzzy_threshold {
            anyhow::bail!(
                "clear_threshold ({}) must be greater than or equal to fuzzy_threshold ({}). \
                 Clear notes (used for learning) should have higher confidence than the threshold for applying fuzzy logic.",
                self.clear_threshold,
                self.fuzzy_threshold
            );
        }
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

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_thresholds() {
        let config = Config {
            fuzzy_threshold: 0.8,
            clear_threshold: 0.7, // Invalid: clear < fuzzy
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_disabled_fuzzy() {
        let config = Config {
            fuzzy_enabled: false,
            fuzzy_threshold: 0.8,
            clear_threshold: 0.7, // Would be invalid if fuzzy enabled, but OK when disabled
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_pitch_bend_defaults() {
        let config = Config::default();
        assert!(config.pitch_bend_enabled);
        assert_eq!(config.pitch_bend_range, 2.0);
    }

    #[test]
    fn test_config_pitch_bend_serialization() {
        let config = Config {
            pitch_bend_enabled: false,
            pitch_bend_range: 12.0,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.pitch_bend_enabled);
        assert_eq!(deserialized.pitch_bend_range, 12.0);
    }
}
