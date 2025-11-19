use std::collections::HashMap;

/// Represents a note detection with confidence level
#[derive(Debug, Clone, Copy)]
pub struct NoteDetection {
    pub note: u8,
    pub frequency: f32,
    pub confidence: f32, // 0.0 (low) to 1.0 (high)
}

/// Tracks note history for learning during the session
pub struct NoteHistory {
    /// Count of how many times each note was detected with high confidence
    note_counts: HashMap<u8, u32>,
    /// Recent note sequence for pattern detection
    recent_notes: Vec<u8>,
    /// Maximum number of recent notes to track
    max_recent: usize,
    /// Minimum confidence threshold for a note to be considered "clear"
    clear_threshold: f32,
}

impl NoteHistory {
    /// Create a new NoteHistory tracker
    pub fn new(max_recent: usize, clear_threshold: f32) -> Self {
        Self {
            note_counts: HashMap::new(),
            recent_notes: Vec::new(),
            max_recent,
            clear_threshold,
        }
    }

    /// Record a note detection
    pub fn record(&mut self, detection: &NoteDetection) {
        // Only track notes with high confidence
        if detection.confidence >= self.clear_threshold {
            *self.note_counts.entry(detection.note).or_insert(0) += 1;

            self.recent_notes.push(detection.note);
            if self.recent_notes.len() > self.max_recent {
                self.recent_notes.remove(0);
            }
        }
    }

    /// Get the frequency of a note in the history (0.0 to 1.0)
    pub fn note_frequency(&self, note: u8) -> f32 {
        let count = self.note_counts.get(&note).copied().unwrap_or(0);
        let total: u32 = self.note_counts.values().sum();

        if total == 0 {
            0.0
        } else {
            count as f32 / total as f32
        }
    }

    /// Check if a note has been seen recently
    pub fn is_recent(&self, note: u8, window: usize) -> bool {
        let start = self.recent_notes.len().saturating_sub(window);
        self.recent_notes[start..].contains(&note)
    }

    /// Get the most common note from history
    #[allow(dead_code)]
    pub fn most_common_note(&self) -> Option<u8> {
        self.note_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&note, _)| note)
    }

    /// Get neighboring notes from the most recent detection
    pub fn get_recent_neighbors(&self) -> Vec<u8> {
        if let Some(&last_note) = self.recent_notes.last() {
            // Return notes within ±2 semitones
            vec![
                last_note.saturating_sub(2),
                last_note.saturating_sub(1),
                last_note,
                last_note.saturating_add(1).min(127),
                last_note.saturating_add(2).min(127),
            ]
        } else {
            vec![]
        }
    }
}

/// Fuzzy logic resolver for ambiguous note detections
pub struct FuzzyNoteResolver {
    history: NoteHistory,
    /// Confidence threshold below which fuzzy logic is applied
    fuzzy_threshold: f32,
}

impl FuzzyNoteResolver {
    /// Create a new fuzzy note resolver
    pub fn new(max_recent: usize, clear_threshold: f32, fuzzy_threshold: f32) -> Self {
        Self {
            history: NoteHistory::new(max_recent, clear_threshold),
            fuzzy_threshold,
        }
    }

    /// Process a note detection and return the resolved note
    pub fn resolve(&mut self, detection: NoteDetection) -> NoteDetection {
        // Record the detection for learning
        self.history.record(&detection);

        // If confidence is high enough, return as-is
        if detection.confidence >= self.fuzzy_threshold {
            return detection;
        }

        // Apply fuzzy logic to resolve ambiguous note
        let resolved_note = self.apply_fuzzy_logic(&detection);

        NoteDetection {
            note: resolved_note,
            frequency: detection.frequency,
            confidence: detection.confidence,
        }
    }

    /// Apply fuzzy logic rules to determine the most likely note
    fn apply_fuzzy_logic(&self, detection: &NoteDetection) -> u8 {
        let mut scores: HashMap<u8, f32> = HashMap::new();

        // Rule 1: Base score from detected note
        scores.insert(detection.note, 1.0);

        // Rule 2: Boost score for recently played notes (temporal locality)
        let recent_window = 5;
        if self.history.is_recent(detection.note, recent_window) {
            *scores.entry(detection.note).or_insert(0.0) += 0.5;
        }

        // Rule 3: Consider neighboring notes from recent history
        for neighbor in self.history.get_recent_neighbors() {
            // Check if the detected note is close to a neighbor
            let semitone_diff = (detection.note as i16 - neighbor as i16).abs();
            if semitone_diff <= 2 {
                let proximity_score = 1.0 - (semitone_diff as f32 * 0.2);
                *scores.entry(neighbor).or_insert(0.0) += proximity_score * 0.3;
            }
        }

        // Rule 4: Boost score based on historical frequency
        let freq_score = self.history.note_frequency(detection.note);
        *scores.entry(detection.note).or_insert(0.0) += freq_score * 0.8;

        // Rule 5: Check for alternative notes within ±1 semitone
        for offset in [-1, 1] {
            let alt_note = (detection.note as i16 + offset).clamp(0, 127) as u8;
            let alt_freq = self.history.note_frequency(alt_note);

            // If alternative has been played significantly more, consider it
            if alt_freq > 0.1 && self.history.is_recent(alt_note, recent_window * 2) {
                *scores.entry(alt_note).or_insert(0.0) += alt_freq * 0.6;
            }
        }

        // Return the note with the highest score
        scores
            .into_iter()
            .max_by(|(_, score_a), (_, score_b)| {
                score_a
                    .partial_cmp(score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(note, _)| note)
            .unwrap_or(detection.note)
    }

    /// Get a reference to the history for testing/debugging
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn history(&self) -> &NoteHistory {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_history_tracking() {
        let mut history = NoteHistory::new(10, 0.8);

        // Record some high confidence notes
        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.9,
        });
        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.85,
        });
        history.record(&NoteDetection {
            note: 62,
            frequency: 293.66,
            confidence: 0.95,
        });

        assert_eq!(history.note_counts.get(&60), Some(&2));
        assert_eq!(history.note_counts.get(&62), Some(&1));
        assert_eq!(history.recent_notes.len(), 3);
    }

    #[test]
    fn test_note_history_ignores_low_confidence() {
        let mut history = NoteHistory::new(10, 0.8);

        // Record a low confidence note - should be ignored
        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.5,
        });

        assert_eq!(history.note_counts.get(&60), None);
        assert_eq!(history.recent_notes.len(), 0);
    }

    #[test]
    fn test_note_frequency_calculation() {
        let mut history = NoteHistory::new(10, 0.8);

        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.9,
        });
        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.9,
        });
        history.record(&NoteDetection {
            note: 62,
            frequency: 293.66,
            confidence: 0.9,
        });

        // 60 appears 2 out of 3 times = ~0.67
        let freq = history.note_frequency(60);
        assert!((freq - 0.666).abs() < 0.01);

        // 62 appears 1 out of 3 times = ~0.33
        let freq = history.note_frequency(62);
        assert!((freq - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_is_recent() {
        let mut history = NoteHistory::new(10, 0.8);

        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.9,
        });
        history.record(&NoteDetection {
            note: 62,
            frequency: 293.66,
            confidence: 0.9,
        });
        history.record(&NoteDetection {
            note: 64,
            frequency: 329.63,
            confidence: 0.9,
        });

        assert!(history.is_recent(64, 1));
        assert!(history.is_recent(62, 2));
        assert!(history.is_recent(60, 3));
        assert!(!history.is_recent(60, 1));
    }

    #[test]
    fn test_most_common_note() {
        let mut history = NoteHistory::new(10, 0.8);

        for _ in 0..5 {
            history.record(&NoteDetection {
                note: 60,
                frequency: 261.63,
                confidence: 0.9,
            });
        }

        for _ in 0..2 {
            history.record(&NoteDetection {
                note: 62,
                frequency: 293.66,
                confidence: 0.9,
            });
        }

        assert_eq!(history.most_common_note(), Some(60));
    }

    #[test]
    fn test_fuzzy_resolver_high_confidence() {
        let mut resolver = FuzzyNoteResolver::new(10, 0.8, 0.7);

        let detection = NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.9,
        };

        let resolved = resolver.resolve(detection);
        assert_eq!(resolved.note, 60);
        assert_eq!(resolved.confidence, 0.9);
    }

    #[test]
    fn test_fuzzy_resolver_with_history() {
        let mut resolver = FuzzyNoteResolver::new(10, 0.8, 0.7);

        // Build up history with note 60
        for _ in 0..5 {
            resolver.resolve(NoteDetection {
                note: 60,
                frequency: 261.63,
                confidence: 0.9,
            });
        }

        // Now detect an ambiguous note (low confidence)
        let detection = NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.5,
        };

        let resolved = resolver.resolve(detection);
        // Should resolve to 60 based on history
        assert_eq!(resolved.note, 60);
    }

    #[test]
    fn test_get_recent_neighbors() {
        let mut history = NoteHistory::new(10, 0.8);

        history.record(&NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.9,
        });

        let neighbors = history.get_recent_neighbors();
        assert_eq!(neighbors, vec![58, 59, 60, 61, 62]);
    }

    #[test]
    fn test_fuzzy_resolver_empty_history() {
        let mut resolver = FuzzyNoteResolver::new(10, 0.8, 0.7);

        let detection = NoteDetection {
            note: 60,
            frequency: 261.63,
            confidence: 0.5,
        };

        let resolved = resolver.resolve(detection);
        // With no history, should return the detected note
        assert_eq!(resolved.note, 60);
    }
}
