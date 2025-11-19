use anyhow::Result;
use crossbeam_channel::{bounded, Receiver};
use log::{debug, info};
use std::time::{Duration, Instant};

use crate::audio::AudioInput;
use crate::config::Config;
use crate::fuzzy::{FuzzyNoteResolver, NoteDetection};
use crate::midi::MidiOutputHandler;
use crate::pitch::PitchDetector;

pub struct StreamProcessor {
    config: Config,
    audio_input: AudioInput,
    pitch_detector: PitchDetector,
    midi_output: MidiOutputHandler,
    fuzzy_resolver: Option<FuzzyNoteResolver>,
    current_note: Option<u8>,
    note_start_time: Option<Instant>,
}

impl StreamProcessor {
    pub fn new(config: Config) -> Result<Self> {
        let audio_input = AudioInput::new()?;
        let sample_rate = audio_input.sample_rate();

        let pitch_detector =
            PitchDetector::new(sample_rate, config.buffer_size, config.pitch_threshold);
        let mut midi_output = MidiOutputHandler::new()?;
        midi_output.connect(config.midi_port.as_deref())?;

        // Initialize fuzzy note resolver if enabled
        let fuzzy_resolver = if config.fuzzy_enabled {
            info!("Fuzzy note detection enabled");
            Some(FuzzyNoteResolver::new(
                config.max_recent_notes,
                config.clear_threshold,
                config.fuzzy_threshold,
            ))
        } else {
            None
        };

        info!(
            "Stream processor initialized with sample rate: {} Hz",
            sample_rate
        );

        Ok(Self {
            config,
            audio_input,
            pitch_detector,
            midi_output,
            fuzzy_resolver,
            current_note: None,
            note_start_time: None,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        info!("Starting real-time audio processing...");

        let (tx, rx) = bounded(10);
        let _stream = self.audio_input.start_stream(tx)?;

        self.process_audio_stream(rx)?;

        Ok(())
    }

    fn process_audio_stream(&mut self, rx: Receiver<Vec<f32>>) -> Result<()> {
        let mut buffer = Vec::new();

        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(samples) => {
                    buffer.extend_from_slice(&samples);

                    // Process buffer when we have enough samples
                    while buffer.len() >= self.config.buffer_size {
                        let chunk: Vec<f32> = buffer.drain(..self.config.buffer_size).collect();
                        self.process_chunk(&chunk)?;
                    }
                }
                Err(_) => {
                    // Timeout - continue processing
                    continue;
                }
            }
        }
    }

    fn process_chunk(&mut self, samples: &[f32]) -> Result<()> {
        // Detect pitch with confidence
        if let Some((frequency, confidence)) =
            self.pitch_detector.detect_pitch_with_confidence(samples)
        {
            let detected_note = PitchDetector::frequency_to_midi(frequency);

            // Create note detection
            let detection = NoteDetection {
                note: detected_note,
                frequency,
                confidence,
            };

            // Apply fuzzy resolution if enabled
            let resolved_detection = if let Some(resolver) = &mut self.fuzzy_resolver {
                resolver.resolve(detection)
            } else {
                detection
            };

            let note = resolved_detection.note;
            let note_name = PitchDetector::midi_to_note_name(note);

            // Handle note change
            if Some(note) != self.current_note {
                // Turn off previous note if it exists
                if let Some(prev_note) = self.current_note {
                    self.midi_output.note_off(prev_note)?;
                    debug!("Note changed from {} to {}", prev_note, note_name);
                }

                // Start new note
                self.midi_output.note_on(note, self.config.velocity)?;
                self.current_note = Some(note);
                self.note_start_time = Some(Instant::now());

                if confidence < self.config.fuzzy_threshold && self.config.fuzzy_enabled {
                    // For fuzzy-resolved notes, show the expected frequency of the resolved note
                    let resolved_frequency = PitchDetector::midi_to_frequency(note);
                    info!(
                        "Playing note: {} ({:.2} Hz) [fuzzy resolved from {:.2} Hz, confidence: {:.2}]",
                        note_name, resolved_frequency, frequency, confidence
                    );
                } else {
                    info!("Playing note: {} ({:.2} Hz)", note_name, frequency);
                }
            }
        } else {
            // No pitch detected - turn off current note if minimum duration met
            if let Some(note) = self.current_note {
                if let Some(start_time) = self.note_start_time {
                    let duration = start_time.elapsed().as_secs_f32();
                    if duration >= self.config.min_note_duration {
                        self.midi_output.note_off(note)?;
                        debug!("Note off after {:.2}s", duration);
                        self.current_note = None;
                        self.note_start_time = None;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping stream processor...");
        self.midi_output.all_notes_off()?;
        Ok(())
    }
}

impl Drop for StreamProcessor {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
