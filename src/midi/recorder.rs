use anyhow::{Context, Result};
use log::{debug, info};
use midly::{
    num::{u15, u24, u28, u4, u7},
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

const DEFAULT_TICKS_PER_BEAT: u16 = 480;
const MICROSECONDS_PER_MINUTE: u32 = 60_000_000;

pub struct MidiRecorder {
    events: Vec<(u64, MidiMessage)>,
    start_time: Instant,
    tempo: u32, // Microseconds per quarter note
    ticks_per_beat: u16,
    is_recording: bool,
}

impl Default for MidiRecorder {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            start_time: Instant::now(),
            tempo: MICROSECONDS_PER_MINUTE / 120, // 120 BPM default
            ticks_per_beat: DEFAULT_TICKS_PER_BEAT,
            is_recording: false,
        }
    }
}

impl MidiRecorder {
    /// Create a new MIDI recorder
    pub fn new() -> Self {
        Self::default()
    }

    /// Start recording
    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.events.clear();
        self.is_recording = true;
        info!("MIDI recording started");
    }

    /// Stop recording
    pub fn stop(&mut self) {
        self.is_recording = false;
        info!(
            "MIDI recording stopped, {} events recorded",
            self.events.len()
        );
    }

    /// Check if currently recording
    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Record a note on event
    pub fn record_note_on(&mut self, note: u8, velocity: u8) {
        if !self.is_recording {
            return;
        }

        let timestamp = self.start_time.elapsed().as_micros() as u64;
        let message = MidiMessage::NoteOn {
            key: u7::new(note),
            vel: u7::new(velocity),
        };
        self.events.push((timestamp, message));
        debug!("Recorded note ON: {} at {}μs", note, timestamp);
    }

    /// Record a note off event
    pub fn record_note_off(&mut self, note: u8) {
        if !self.is_recording {
            return;
        }

        let timestamp = self.start_time.elapsed().as_micros() as u64;
        let message = MidiMessage::NoteOff {
            key: u7::new(note),
            vel: u7::new(0),
        };
        self.events.push((timestamp, message));
        debug!("Recorded note OFF: {} at {}μs", note, timestamp);
    }

    /// Save recorded MIDI events to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if self.events.is_empty() {
            anyhow::bail!("No MIDI events to save");
        }

        let path = path.as_ref();
        info!("Saving {} MIDI events to {:?}", self.events.len(), path);

        // Convert events to MIDI track events with delta times
        let mut track_events = Vec::new();

        // Add tempo meta event at the beginning
        track_events.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(self.tempo))),
        });

        // Convert recorded events to track events
        let mut last_timestamp = 0u64;
        for (timestamp, message) in &self.events {
            // Calculate delta time in ticks
            let delta_micros = timestamp.saturating_sub(last_timestamp);
            let delta_ticks = self.micros_to_ticks(delta_micros);

            track_events.push(TrackEvent {
                delta: u28::new(delta_ticks),
                kind: TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: *message,
                },
            });

            last_timestamp = *timestamp;
        }

        // Add end of track meta event
        track_events.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        // Create SMF structure
        let header = Header {
            format: Format::SingleTrack,
            timing: Timing::Metrical(u15::new(self.ticks_per_beat)),
        };

        let track = Track::from(track_events);
        let smf = Smf {
            header,
            tracks: vec![track],
        };

        // Write to file
        let mut file = File::create(path).context("Failed to create MIDI file")?;
        smf.write_std(&mut file)
            .context("Failed to write MIDI data")?;
        file.flush().context("Failed to flush MIDI file")?;

        info!("MIDI file saved successfully to {:?}", path);
        Ok(())
    }

    /// Convert microseconds to MIDI ticks
    fn micros_to_ticks(&self, micros: u64) -> u32 {
        // ticks = (microseconds * ticks_per_beat) / tempo
        let ticks = (micros * self.ticks_per_beat as u64) / self.tempo as u64;
        ticks.min(u32::MAX as u64) as u32
    }

    /// Get the number of recorded events
    #[allow(dead_code)]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Clear all recorded events
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.events.clear();
        debug!("Cleared all recorded MIDI events");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_recorder_creation() {
        let recorder = MidiRecorder::new();
        assert!(!recorder.is_recording());
        assert_eq!(recorder.event_count(), 0);
    }

    #[test]
    fn test_start_stop_recording() {
        let mut recorder = MidiRecorder::new();
        assert!(!recorder.is_recording());

        recorder.start();
        assert!(recorder.is_recording());

        recorder.stop();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_record_events() {
        let mut recorder = MidiRecorder::new();
        recorder.start();

        recorder.record_note_on(60, 80);
        thread::sleep(Duration::from_millis(10));
        recorder.record_note_off(60);

        recorder.stop();
        assert_eq!(recorder.event_count(), 2);
    }

    #[test]
    fn test_no_recording_when_stopped() {
        let mut recorder = MidiRecorder::new();
        recorder.record_note_on(60, 80);
        recorder.record_note_off(60);
        assert_eq!(recorder.event_count(), 0);
    }

    #[test]
    fn test_clear_events() {
        let mut recorder = MidiRecorder::new();
        recorder.start();
        recorder.record_note_on(60, 80);
        recorder.record_note_off(60);
        assert_eq!(recorder.event_count(), 2);

        recorder.clear();
        assert_eq!(recorder.event_count(), 0);
    }

    #[test]
    fn test_save_empty_recording() {
        let recorder = MidiRecorder::new();
        let result = recorder.save("/tmp/test_empty.mid");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_recording() {
        let mut recorder = MidiRecorder::new();
        recorder.start();
        recorder.record_note_on(60, 80);
        thread::sleep(Duration::from_millis(10));
        recorder.record_note_off(60);
        recorder.stop();

        let path = "/tmp/test_recording.mid";
        let result = recorder.save(path);
        assert!(result.is_ok());

        // Verify file was created
        assert!(std::path::Path::new(path).exists());
    }

    #[test]
    fn test_micros_to_ticks() {
        let recorder = MidiRecorder::new();
        // With default tempo (500000 μs/beat) and 480 ticks/beat:
        // 1 tick = 500000 / 480 = ~1041.67 microseconds
        let ticks = recorder.micros_to_ticks(500000);
        assert_eq!(ticks, 480); // Should be exactly one beat
    }
}
