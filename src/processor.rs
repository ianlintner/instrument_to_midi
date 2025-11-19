use anyhow::Result;
use crossbeam_channel::{bounded, Receiver};
use log::{debug, info};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::audio::AudioInput;
use crate::config::Config;
use crate::fuzzy::{FuzzyNoteResolver, NoteDetection};
use crate::midi::{MidiOutputHandler, MidiRecorder};
use crate::pitch::polyphonic::PolyphonicPitchDetector;
use crate::pitch::PitchDetector;
use crate::web::MonitoringEvent;

pub struct StreamProcessor {
    config: Config,
    audio_input: AudioInput,
    pitch_detector: PitchDetector,
    polyphonic_detector: Option<PolyphonicPitchDetector>,
    midi_output: MidiOutputHandler,
    midi_recorder: Option<MidiRecorder>,
    fuzzy_resolver: Option<FuzzyNoteResolver>,
    current_note: Option<u8>,
    active_notes: HashSet<u8>,
    note_start_time: Option<Instant>,
    web_event_tx: Option<broadcast::Sender<MonitoringEvent>>,
}

impl StreamProcessor {
    pub fn new(config: Config) -> Result<Self> {
        let audio_input = AudioInput::new()?;
        let sample_rate = audio_input.sample_rate();

        let pitch_detector =
            PitchDetector::new(sample_rate, config.buffer_size, config.pitch_threshold);

        // Initialize polyphonic detector if enabled
        let polyphonic_detector = if config.polyphonic_enabled {
            info!("Polyphonic pitch detection enabled");
            Some(PolyphonicPitchDetector::new(
                sample_rate,
                config.buffer_size,
                config.polyphonic_threshold,
            ))
        } else {
            None
        };

        let mut midi_output = MidiOutputHandler::new()?;
        midi_output.connect(config.midi_port.as_deref())?;

        // Initialize fuzzy note resolver if enabled (only for monophonic mode)
        let fuzzy_resolver = if config.fuzzy_enabled && !config.polyphonic_enabled {
            info!("Fuzzy note detection enabled");
            Some(FuzzyNoteResolver::new(
                config.max_recent_notes,
                config.clear_threshold,
                config.fuzzy_threshold,
            ))
        } else {
            None
        };

        // Initialize MIDI recorder if enabled
        let midi_recorder = if config.record_enabled {
            info!("MIDI recording enabled");
            Some(MidiRecorder::new())
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
            polyphonic_detector,
            midi_output,
            midi_recorder,
            fuzzy_resolver,
            current_note: None,
            active_notes: HashSet::new(),
            note_start_time: None,
            web_event_tx: None,
        })
    }

    /// Set the web event sender for broadcasting monitoring events
    pub fn set_web_event_sender(&mut self, tx: broadcast::Sender<MonitoringEvent>) {
        self.web_event_tx = Some(tx);
    }

    pub fn start(&mut self) -> Result<()> {
        info!("Starting real-time audio processing...");

        // Start MIDI recording if enabled
        if let Some(recorder) = &mut self.midi_recorder {
            recorder.start();
        }

        // Broadcast recording status to web UI
        if let Some(tx) = &self.web_event_tx {
            let _ = tx.send(MonitoringEvent::RecordingStatus {
                recording: self.midi_recorder.is_some(),
            });
        }

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
        // Use polyphonic detection if enabled
        if self.polyphonic_detector.is_some() {
            // Extract the detector temporarily to avoid borrow checker issues
            let mut poly_detector = self.polyphonic_detector.take().unwrap();
            self.process_polyphonic(samples, &mut poly_detector)?;
            self.polyphonic_detector = Some(poly_detector);
        } else {
            self.process_monophonic(samples)?;
        }
        Ok(())
    }

    fn process_polyphonic(
        &mut self,
        samples: &[f32],
        poly_detector: &mut PolyphonicPitchDetector,
    ) -> Result<()> {
        let candidates = poly_detector.detect_pitches(samples);

        // Get current detected notes
        let detected_notes: HashSet<u8> = candidates.iter().map(|c| c.midi_note).collect();

        // Turn off notes that are no longer detected
        let notes_to_turn_off: Vec<u8> = self
            .active_notes
            .difference(&detected_notes)
            .copied()
            .collect();

        for &note in &notes_to_turn_off {
            self.midi_output.note_off(note)?;
            if let Some(recorder) = &mut self.midi_recorder {
                recorder.record_note_off(note);
            }

            // Broadcast note off event
            if let Some(tx) = &self.web_event_tx {
                let note_name = PolyphonicPitchDetector::midi_to_note_name(note);
                let _ = tx.send(MonitoringEvent::NoteOff { note, note_name });
            }

            self.active_notes.remove(&note);
            debug!("Note off (polyphonic): {}", note);
        }

        // Turn on new notes
        let notes_to_turn_on: Vec<u8> = detected_notes
            .difference(&self.active_notes)
            .copied()
            .collect();

        for &note in &notes_to_turn_on {
            self.midi_output.note_on(note, self.config.velocity)?;
            if let Some(recorder) = &mut self.midi_recorder {
                recorder.record_note_on(note, self.config.velocity);
            }

            self.active_notes.insert(note);

            // Broadcast note on event
            if let Some(tx) = &self.web_event_tx {
                let note_name = PolyphonicPitchDetector::midi_to_note_name(note);
                if let Some(candidate) = candidates.iter().find(|c| c.midi_note == note) {
                    let _ = tx.send(MonitoringEvent::NoteOn {
                        note,
                        note_name: note_name.clone(),
                        frequency: candidate.frequency,
                        velocity: self.config.velocity,
                        confidence: candidate.magnitude,
                    });
                }
            }

            debug!("Note on (polyphonic): {}", note);
        }

        // Log active notes if changed
        if !notes_to_turn_off.is_empty() || !notes_to_turn_on.is_empty() {
            let note_names: Vec<String> = self
                .active_notes
                .iter()
                .map(|&n| PolyphonicPitchDetector::midi_to_note_name(n))
                .collect();
            info!("Active notes: {}", note_names.join(", "));
        }

        Ok(())
    }

    fn process_monophonic(&mut self, samples: &[f32]) -> Result<()> {
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
                    if let Some(recorder) = &mut self.midi_recorder {
                        recorder.record_note_off(prev_note);
                    }

                    // Broadcast note off event
                    if let Some(tx) = &self.web_event_tx {
                        let prev_note_name = PitchDetector::midi_to_note_name(prev_note);
                        let _ = tx.send(MonitoringEvent::NoteOff {
                            note: prev_note,
                            note_name: prev_note_name,
                        });
                    }

                    debug!("Note changed from {} to {}", prev_note, note_name);
                }

                // Start new note
                self.midi_output.note_on(note, self.config.velocity)?;
                if let Some(recorder) = &mut self.midi_recorder {
                    recorder.record_note_on(note, self.config.velocity);
                }
                self.current_note = Some(note);
                self.note_start_time = Some(Instant::now());

                // Broadcast note on event
                if let Some(tx) = &self.web_event_tx {
                    let _ = tx.send(MonitoringEvent::NoteOn {
                        note,
                        note_name: note_name.clone(),
                        frequency,
                        velocity: self.config.velocity,
                        confidence,
                    });
                }

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

            // Apply pitch bend if enabled and we have an active note
            if self.config.pitch_bend_enabled && self.current_note.is_some() {
                let bend = PitchDetector::calculate_pitch_bend(
                    frequency,
                    note,
                    self.config.pitch_bend_range,
                );
                self.midi_output.pitch_bend(bend)?;

                // Broadcast pitch bend event
                if let Some(tx) = &self.web_event_tx {
                    let _ = tx.send(MonitoringEvent::PitchBend {
                        note,
                        bend_value: bend,
                    });
                }
            }
        } else {
            // No pitch detected - turn off current note if minimum duration met
            if let Some(note) = self.current_note {
                if let Some(start_time) = self.note_start_time {
                    let duration = start_time.elapsed().as_secs_f32();
                    if duration >= self.config.min_note_duration {
                        self.midi_output.note_off(note)?;
                        if let Some(recorder) = &mut self.midi_recorder {
                            recorder.record_note_off(note);
                        }

                        // Broadcast note off event
                        if let Some(tx) = &self.web_event_tx {
                            let note_name = PitchDetector::midi_to_note_name(note);
                            let _ = tx.send(MonitoringEvent::NoteOff { note, note_name });
                        }

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

        // Turn off all active notes
        self.midi_output.all_notes_off()?;
        self.active_notes.clear();

        // Save MIDI recording if enabled
        if let Some(recorder) = &mut self.midi_recorder {
            recorder.stop();
            if recorder.event_count() > 0 {
                let default_path;
                let output_path = if let Some(ref path) = self.config.record_output {
                    path.as_str()
                } else {
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    default_path = format!("recording_{}.mid", timestamp);
                    &default_path
                };
                recorder.save(output_path)?;
                info!("MIDI recording saved to: {}", output_path);
            } else {
                info!("No MIDI events recorded");
            }
        }

        Ok(())
    }
}

impl Drop for StreamProcessor {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
