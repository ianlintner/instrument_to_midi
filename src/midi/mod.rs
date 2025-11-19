use anyhow::{Context, Result};
use log::{debug, info};
use midir::{MidiOutput, MidiOutputConnection};
use std::collections::HashMap;

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const PITCH_BEND: u8 = 0xE0;
#[allow(dead_code)]
const DEFAULT_VELOCITY: u8 = 80;
const DEFAULT_CHANNEL: u8 = 0;

pub struct MidiOutputHandler {
    connection: Option<MidiOutputConnection>,
    active_notes: HashMap<u8, u64>,
    note_counter: u64,
}

impl MidiOutputHandler {
    /// Create a new MIDI output handler
    pub fn new() -> Result<Self> {
        Ok(Self {
            connection: None,
            active_notes: HashMap::new(),
            note_counter: 0,
        })
    }

    /// Connect to a MIDI output port by name or create a virtual port
    pub fn connect(&mut self, port_name: Option<&str>) -> Result<()> {
        let midi_out = MidiOutput::new("instrument_to_midi")?;

        let connection = if let Some(name) = port_name {
            // Find port by name
            let ports = midi_out.ports();
            let port = ports
                .iter()
                .find(|p| {
                    midi_out
                        .port_name(p)
                        .map(|n| n.contains(name))
                        .unwrap_or(false)
                })
                .context(format!("MIDI port '{}' not found", name))?;

            info!("Connecting to MIDI port: {}", midi_out.port_name(port)?);
            midi_out
                .connect(port, "instrument_to_midi_out")
                .map_err(|e| anyhow::anyhow!("Failed to connect to MIDI port: {:?}", e))?
        } else {
            // Try to create virtual port (Unix only), otherwise use first available port
            #[cfg(target_os = "linux")]
            {
                use midir::os::unix::VirtualOutput;
                info!("Creating virtual MIDI port: instrument_to_midi");
                midi_out
                    .create_virtual("instrument_to_midi")
                    .map_err(|e| anyhow::anyhow!("Failed to create virtual MIDI port: {:?}", e))?
            }
            #[cfg(not(target_os = "linux"))]
            {
                let ports = midi_out.ports();
                if ports.is_empty() {
                    anyhow::bail!("No MIDI output ports available");
                }
                let port = &ports[0];
                info!("Using MIDI port: {}", midi_out.port_name(port)?);
                midi_out
                    .connect(port, "instrument_to_midi_out")
                    .map_err(|e| anyhow::anyhow!("Failed to connect to MIDI port: {:?}", e))?
            }
        };

        self.connection = Some(connection);
        Ok(())
    }

    /// Send a note on message
    pub fn note_on(&mut self, note: u8, velocity: u8) -> Result<()> {
        if let Some(conn) = &mut self.connection {
            let message = [NOTE_ON | DEFAULT_CHANNEL, note, velocity];
            conn.send(&message)?;

            self.note_counter += 1;
            self.active_notes.insert(note, self.note_counter);

            debug!("Note ON: {} velocity: {}", note, velocity);
            Ok(())
        } else {
            anyhow::bail!("MIDI output not connected")
        }
    }

    /// Send a note off message
    pub fn note_off(&mut self, note: u8) -> Result<()> {
        if let Some(conn) = &mut self.connection {
            let message = [NOTE_OFF | DEFAULT_CHANNEL, note, 0];
            conn.send(&message)?;

            self.active_notes.remove(&note);

            debug!("Note OFF: {}", note);
            Ok(())
        } else {
            anyhow::bail!("MIDI output not connected")
        }
    }

    /// Stop all currently active notes
    pub fn all_notes_off(&mut self) -> Result<()> {
        let notes: Vec<u8> = self.active_notes.keys().copied().collect();
        for note in notes {
            self.note_off(note)?;
        }
        Ok(())
    }

    /// Send a pitch bend message
    ///
    /// # Arguments
    /// * `bend` - Pitch bend value from -1.0 to +1.0, where:
    ///   - -1.0 = maximum downward bend
    ///   - 0.0 = no bend (centered)
    ///   - +1.0 = maximum upward bend
    pub fn pitch_bend(&mut self, bend: f32) -> Result<()> {
        if let Some(conn) = &mut self.connection {
            // Clamp bend value to valid range
            let bend = bend.clamp(-1.0, 1.0);

            // Convert to 14-bit MIDI pitch bend value (0-16383, center is 8192)
            let bend_value = ((bend + 1.0) * 8192.0) as u16;
            let bend_value = bend_value.clamp(0, 16383);

            // Split into LSB and MSB (7 bits each)
            let lsb = (bend_value & 0x7F) as u8;
            let msb = ((bend_value >> 7) & 0x7F) as u8;

            let message = [PITCH_BEND | DEFAULT_CHANNEL, lsb, msb];
            conn.send(&message)?;

            debug!("Pitch bend: {:.3} (value: {})", bend, bend_value);
            Ok(())
        } else {
            anyhow::bail!("MIDI output not connected")
        }
    }

    /// Check if a note is currently active
    #[allow(dead_code)]
    pub fn is_note_active(&self, note: u8) -> bool {
        self.active_notes.contains_key(&note)
    }

    /// Get the number of active notes
    #[allow(dead_code)]
    pub fn active_note_count(&self) -> usize {
        self.active_notes.len()
    }
}

impl Drop for MidiOutputHandler {
    fn drop(&mut self) {
        // Send note off for all active notes when dropping
        let _ = self.all_notes_off();
    }
}

/// List available MIDI output ports
pub fn list_midi_ports() -> Result<Vec<String>> {
    let midi_out = MidiOutput::new("instrument_to_midi")?;
    let ports = midi_out.ports();

    let mut port_names = Vec::new();
    for port in ports.iter() {
        if let Ok(name) = midi_out.port_name(port) {
            port_names.push(name);
        }
    }

    Ok(port_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_output_creation() {
        let result = MidiOutputHandler::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_midi_ports() {
        // This test might fail on systems without MIDI devices
        // Just ensure it doesn't panic
        let result = list_midi_ports();
        // Accept both success and error cases
        let _ = result;
    }

    #[test]
    fn test_note_tracking() {
        let mut handler = MidiOutputHandler::new().unwrap();
        assert_eq!(handler.active_note_count(), 0);
        assert!(!handler.is_note_active(60));

        // Simulate note tracking (without actual MIDI connection)
        handler.active_notes.insert(60, 1);
        assert_eq!(handler.active_note_count(), 1);
        assert!(handler.is_note_active(60));
    }

    #[test]
    fn test_pitch_bend_calculation() {
        // Test center position (no bend)
        let bend_value = ((0.0 + 1.0) * 8192.0) as u16;
        assert_eq!(bend_value, 8192);

        // Test maximum upward bend
        let bend_value = ((1.0 + 1.0) * 8192.0) as u16;
        assert_eq!(bend_value, 16384);
        let clamped = bend_value.clamp(0, 16383);
        assert_eq!(clamped, 16383);

        // Test maximum downward bend
        let bend_value = ((-1.0 + 1.0) * 8192.0) as u16;
        assert_eq!(bend_value, 0);
    }
}
