use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{Pattern, Song};

pub const MAX_PATTERN_VARIATIONS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PatternVariationId(pub u64);

impl std::fmt::Display for PatternVariationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "v{:03}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatternVariationSource {
    AiProposal,
    EuclideanTransform,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatternVariation {
    pub id: PatternVariationId,
    pub timestamp: u64,
    pub description: String,
    pub source: PatternVariationSource,
    pub pattern_index: usize,
    pub track_index: Option<usize>,
    pub snapshot: Pattern,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatternVariationHistory {
    #[serde(default = "default_next_id")]
    next_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_id: Option<PatternVariationId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    entries: Vec<PatternVariation>,
}

impl Default for PatternVariationHistory {
    fn default() -> Self {
        Self {
            next_id: default_next_id(),
            active_id: None,
            entries: Vec::new(),
        }
    }
}

impl PatternVariationHistory {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn entries(&self) -> &[PatternVariation] {
        &self.entries
    }

    #[must_use]
    pub const fn active_id(&self) -> Option<PatternVariationId> {
        self.active_id
    }

    #[must_use]
    pub fn entry(&self, id: PatternVariationId) -> Option<&PatternVariation> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    pub fn record_now(
        &mut self,
        description: impl Into<String>,
        source: PatternVariationSource,
        pattern_index: usize,
        track_index: Option<usize>,
        snapshot: Pattern,
    ) -> Result<PatternVariationId, PatternVariationError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        self.record_at(
            timestamp,
            description,
            source,
            pattern_index,
            track_index,
            snapshot,
        )
    }

    pub fn record_at(
        &mut self,
        timestamp: u64,
        description: impl Into<String>,
        source: PatternVariationSource,
        pattern_index: usize,
        track_index: Option<usize>,
        snapshot: Pattern,
    ) -> Result<PatternVariationId, PatternVariationError> {
        let description = description.into();
        validate_entry_fields(&description, pattern_index, track_index, &snapshot)?;
        let id = PatternVariationId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(PatternVariationError::IdExhausted)?;
        self.entries.push(PatternVariation {
            id,
            timestamp,
            description,
            source,
            pattern_index,
            track_index,
            snapshot,
        });
        if self.entries.len() > MAX_PATTERN_VARIATIONS {
            let excess = self.entries.len() - MAX_PATTERN_VARIATIONS;
            self.entries.drain(..excess);
        }
        self.active_id = Some(id);
        Ok(id)
    }

    pub fn set_active(&mut self, id: PatternVariationId) -> Result<(), PatternVariationError> {
        if self.entry(id).is_none() {
            return Err(PatternVariationError::ActiveVersionMissing(id));
        }
        self.active_id = Some(id);
        Ok(())
    }

    pub fn reconcile(&mut self, song: &Song) {
        self.entries
            .retain(|entry| entry_is_compatible_with_song(entry, song).is_ok());
        self.active_id = self
            .entries
            .iter()
            .rev()
            .find(|entry| {
                song.patterns
                    .get(entry.pattern_index)
                    .is_some_and(|pattern| pattern == &entry.snapshot)
            })
            .map(|entry| entry.id);
    }

    pub fn validate_for_song(&self, song: &Song) -> Result<(), PatternVariationError> {
        self.validate()?;
        for entry in &self.entries {
            entry_is_compatible_with_song(entry, song)?;
        }
        if let Some(active) = self.active_id {
            let entry = self
                .entry(active)
                .ok_or(PatternVariationError::ActiveVersionMissing(active))?;
            if song.patterns.get(entry.pattern_index) != Some(&entry.snapshot) {
                return Err(PatternVariationError::ActiveVersionMismatch(active));
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), PatternVariationError> {
        if self.entries.len() > MAX_PATTERN_VARIATIONS {
            return Err(PatternVariationError::TooManyEntries(self.entries.len()));
        }
        let mut previous = None;
        for entry in &self.entries {
            validate_entry_fields(
                &entry.description,
                entry.pattern_index,
                entry.track_index,
                &entry.snapshot,
            )?;
            if previous.is_some_and(|id| entry.id.0 <= id) {
                return Err(PatternVariationError::IdsNotIncreasing);
            }
            previous = Some(entry.id.0);
        }
        if previous.is_some_and(|id| self.next_id <= id) || self.next_id == 0 {
            return Err(PatternVariationError::InvalidNextId(self.next_id));
        }
        if let Some(active) = self.active_id {
            if self.entry(active).is_none() {
                return Err(PatternVariationError::ActiveVersionMissing(active));
            }
        }
        Ok(())
    }
}

fn entry_is_compatible_with_song(
    entry: &PatternVariation,
    song: &Song,
) -> Result<(), PatternVariationError> {
    if entry.pattern_index >= song.patterns.len() {
        return Err(PatternVariationError::PatternOutOfBounds {
            pattern_index: entry.pattern_index,
            pattern_count: song.patterns.len(),
        });
    }
    let mut candidate = song.clone();
    candidate.patterns[entry.pattern_index] = entry.snapshot.clone();
    candidate
        .validate()
        .map_err(|error| PatternVariationError::SnapshotIncompatible {
            pattern_index: entry.pattern_index,
            reason: error.to_string(),
        })
}

fn default_next_id() -> u64 {
    1
}

fn validate_entry_fields(
    description: &str,
    pattern_index: usize,
    track_index: Option<usize>,
    snapshot: &Pattern,
) -> Result<(), PatternVariationError> {
    if description.trim().is_empty() {
        return Err(PatternVariationError::EmptyDescription);
    }
    if snapshot.name.trim().is_empty() || snapshot.rows.is_empty() {
        return Err(PatternVariationError::InvalidSnapshot(pattern_index));
    }
    let Some(width) = snapshot.rows.first().map(|row| row.cells.len()) else {
        return Err(PatternVariationError::InvalidSnapshot(pattern_index));
    };
    if width == 0 || snapshot.rows.iter().any(|row| row.cells.len() != width) {
        return Err(PatternVariationError::InvalidSnapshot(pattern_index));
    }
    if track_index.is_some_and(|track| track >= width) {
        return Err(PatternVariationError::TrackOutOfBounds {
            pattern_index,
            track_index: track_index.expect("checked as some"),
            track_count: width,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternVariationError {
    #[error("variation description cannot be empty")]
    EmptyDescription,
    #[error("variation snapshot for pattern {0} is structurally invalid")]
    InvalidSnapshot(usize),
    #[error("variation pattern {pattern_index} is outside project with {pattern_count} patterns")]
    PatternOutOfBounds {
        pattern_index: usize,
        pattern_count: usize,
    },
    #[error("variation snapshot for pattern {pattern_index} is incompatible: {reason}")]
    SnapshotIncompatible {
        pattern_index: usize,
        reason: String,
    },
    #[error(
        "variation track {track_index} is outside pattern {pattern_index} with {track_count} tracks"
    )]
    TrackOutOfBounds {
        pattern_index: usize,
        track_index: usize,
        track_count: usize,
    },
    #[error("variation ids must be strictly increasing")]
    IdsNotIncreasing,
    #[error("variation next id {0} is invalid")]
    InvalidNextId(u64),
    #[error("variation id space is exhausted")]
    IdExhausted,
    #[error("active variation {0} is not retained")]
    ActiveVersionMissing(PatternVariationId),
    #[error("active variation {0} does not match the live pattern")]
    ActiveVersionMismatch(PatternVariationId),
    #[error("variation history has {0} entries; maximum is {MAX_PATTERN_VARIATIONS}")]
    TooManyEntries(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_is_monotonic_bounded_and_reconciles_active_snapshot() {
        let song = Song::empty();
        let snapshot = song.patterns[0].clone();
        let mut history = PatternVariationHistory::default();
        for index in 0..=MAX_PATTERN_VARIATIONS {
            let id = history
                .record_at(
                    index as u64,
                    format!("take {index}"),
                    PatternVariationSource::AiProposal,
                    0,
                    Some(0),
                    snapshot.clone(),
                )
                .expect("record variation");
            assert_eq!(id.0, index as u64 + 1);
        }

        assert_eq!(history.entries().len(), MAX_PATTERN_VARIATIONS);
        assert_eq!(history.entries()[0].id, PatternVariationId(2));
        assert_eq!(history.active_id(), Some(PatternVariationId(65)));
        history.reconcile(&song);
        assert_eq!(history.active_id(), Some(PatternVariationId(65)));
        history.validate().expect("valid history");
    }

    #[test]
    fn validation_rejects_empty_descriptions_and_dangling_active_ids() {
        let song = Song::empty();
        let mut history = PatternVariationHistory::default();
        assert_eq!(
            history.record_at(
                0,
                " ",
                PatternVariationSource::EuclideanTransform,
                0,
                Some(0),
                song.patterns[0].clone(),
            ),
            Err(PatternVariationError::EmptyDescription)
        );
        history.active_id = Some(PatternVariationId(42));
        assert_eq!(
            history.validate(),
            Err(PatternVariationError::ActiveVersionMissing(
                PatternVariationId(42)
            ))
        );
    }

    #[test]
    fn project_validation_rejects_stale_targets_and_reconcile_prunes_incompatible_entries() {
        let mut song = Song::empty();
        let mut history = PatternVariationHistory::default();
        history
            .record_at(
                0,
                "valid take",
                PatternVariationSource::AiProposal,
                0,
                Some(0),
                song.patterns[0].clone(),
            )
            .expect("record take");
        history
            .validate_for_song(&song)
            .expect("history matches song");

        song.patterns[0]
            .set_note(0, 0, crate::NoteEvent::Note { pitch: 60 }, 100)
            .expect("change live pattern");
        assert!(matches!(
            history.validate_for_song(&song),
            Err(PatternVariationError::ActiveVersionMismatch(_))
        ));
        history.reconcile(&song);
        assert_eq!(history.active_id(), None);

        song.create_track();
        assert!(matches!(
            history.validate_for_song(&song),
            Err(PatternVariationError::SnapshotIncompatible { .. })
        ));
        history.reconcile(&song);
        assert!(history.is_empty());
        assert_eq!(history.active_id(), None);

        let mut stale = PatternVariationHistory::default();
        stale
            .record_at(
                1,
                "missing pattern",
                PatternVariationSource::EuclideanTransform,
                99,
                None,
                song.patterns[0].clone(),
            )
            .expect("record structurally valid stale take");
        assert!(matches!(
            stale.validate_for_song(&song),
            Err(PatternVariationError::PatternOutOfBounds { .. })
        ));
    }
}
