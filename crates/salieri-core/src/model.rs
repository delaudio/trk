use std::{collections::HashSet, fmt};

use serde::{Deserialize, Serialize};

pub const DEFAULT_BPM: u16 = 120;
pub const DEFAULT_LINES_PER_BEAT: u8 = 4;
pub const DEFAULT_PATTERN_LEN: usize = 64;
pub const DEFAULT_TRACK_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TrackId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PatternId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SampleId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstrumentId(pub u32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub metadata: SongMetadata,
    pub transport: TransportSettings,
    pub tracks: Vec<Track>,
    pub patterns: Vec<Pattern>,
    pub sequence: Vec<PatternId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<SampleReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sample_assignments: Vec<TrackSampleAssignment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instruments: Vec<Instrument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub track_instrument_assignments: Vec<TrackInstrumentAssignment>,
}

impl Song {
    #[must_use]
    pub fn empty() -> Self {
        let tracks = (0..DEFAULT_TRACK_COUNT)
            .map(|index| Track {
                id: TrackId(index as u32 + 1),
                name: default_track_name(index),
                midi_channel: default_midi_channel(index),
                muted: false,
                solo: false,
                armed: index == 0,
            })
            .collect::<Vec<_>>();

        let pattern = Pattern::empty(
            PatternId(1),
            "Pattern 01",
            DEFAULT_PATTERN_LEN,
            tracks.len(),
        );

        Self {
            metadata: SongMetadata {
                title: "Untitled".to_string(),
                author: None,
                created_at: None,
            },
            transport: TransportSettings {
                bpm: DEFAULT_BPM,
                lines_per_beat: DEFAULT_LINES_PER_BEAT,
                swing: 0.0,
            },
            tracks,
            patterns: vec![pattern],
            sequence: vec![PatternId(1)],
            samples: Vec::new(),
            sample_assignments: Vec::new(),
            instruments: Vec::new(),
            track_instrument_assignments: Vec::new(),
        }
    }

    #[must_use]
    pub fn current_pattern(&self) -> Option<&Pattern> {
        self.patterns.first()
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.metadata.title.trim().is_empty() {
            return Err(ValidationError::EmptySongTitle);
        }
        if self.transport.bpm == 0 {
            return Err(ValidationError::InvalidBpm {
                bpm: self.transport.bpm,
            });
        }
        if self.transport.lines_per_beat == 0 {
            return Err(ValidationError::InvalidLinesPerBeat {
                lines_per_beat: self.transport.lines_per_beat,
            });
        }
        if !self.transport.swing.is_finite() {
            return Err(ValidationError::InvalidSwing);
        }
        if self.tracks.is_empty() {
            return Err(ValidationError::NoTracks);
        }
        if self.patterns.is_empty() {
            return Err(ValidationError::NoPatterns);
        }
        if self.sequence.is_empty() {
            return Err(ValidationError::EmptySequence);
        }

        let mut track_ids = HashSet::new();
        for (track_index, track) in self.tracks.iter().enumerate() {
            if !track_ids.insert(track.id) {
                return Err(ValidationError::DuplicateTrackId { track_id: track.id });
            }
            if track.name.trim().is_empty() {
                return Err(ValidationError::EmptyTrackName { track_index });
            }
            if !(1..=16).contains(&track.midi_channel) {
                return Err(ValidationError::InvalidTrackMidiChannel {
                    track_index,
                    midi_channel: track.midi_channel,
                });
            }
        }

        let mut pattern_ids = HashSet::new();
        for (pattern_index, pattern) in self.patterns.iter().enumerate() {
            if !pattern_ids.insert(pattern.id) {
                return Err(ValidationError::DuplicatePatternId {
                    pattern_id: pattern.id,
                });
            }
            if pattern.name.trim().is_empty() {
                return Err(ValidationError::EmptyPatternName { pattern_index });
            }
            if pattern.rows.is_empty() {
                return Err(ValidationError::EmptyPattern { pattern_index });
            }
            for (row_index, row) in pattern.rows.iter().enumerate() {
                if row.cells.len() != self.tracks.len() {
                    return Err(ValidationError::PatternRowCellCountMismatch {
                        pattern_index,
                        row_index,
                        expected: self.tracks.len(),
                        actual: row.cells.len(),
                    });
                }
                for (track_index, cell) in row.cells.iter().enumerate() {
                    if let Some(NoteEvent::Note { pitch }) = cell.note {
                        if pitch > 127 {
                            return Err(ValidationError::InvalidNotePitch {
                                pattern_index,
                                row_index,
                                track_index,
                                pitch,
                            });
                        }
                    }
                    if let Some(velocity) = cell.velocity {
                        if velocity > 0x7f {
                            return Err(ValidationError::InvalidVelocity {
                                pattern_index,
                                row_index,
                                track_index,
                                velocity,
                            });
                        }
                    }
                    if let Some(gate) = cell.gate {
                        if gate > 0x7f {
                            return Err(ValidationError::InvalidGate {
                                pattern_index,
                                row_index,
                                track_index,
                                gate,
                            });
                        }
                    }
                }
            }
        }

        for (position, pattern_id) in self.sequence.iter().enumerate() {
            if !pattern_ids.contains(pattern_id) {
                return Err(ValidationError::SequencePatternNotFound {
                    position,
                    pattern_id: *pattern_id,
                });
            }
        }

        let mut sample_ids = HashSet::new();
        for (sample_index, sample) in self.samples.iter().enumerate() {
            if !sample_ids.insert(sample.id) {
                return Err(ValidationError::DuplicateSampleId {
                    sample_id: sample.id,
                });
            }
            if sample.name.trim().is_empty() {
                return Err(ValidationError::EmptySampleName { sample_index });
            }
            if sample.path.trim().is_empty() {
                return Err(ValidationError::EmptySamplePath { sample_index });
            }
            if sample.root_pitch > 127 {
                return Err(ValidationError::InvalidSampleRootPitch {
                    sample_index,
                    root_pitch: sample.root_pitch,
                });
            }
            if !sample.gain.is_finite() || sample.gain < 0.0 {
                return Err(ValidationError::InvalidSampleGain { sample_index });
            }
            validate_sample_playback_settings(sample_index, sample.playback)?;
        }

        let mut assigned_tracks = HashSet::new();
        for assignment in &self.sample_assignments {
            if !track_ids.contains(&assignment.track) {
                return Err(ValidationError::SampleAssignmentTrackNotFound {
                    track_id: assignment.track,
                });
            }
            if !sample_ids.contains(&assignment.sample) {
                return Err(ValidationError::SampleAssignmentSampleNotFound {
                    sample_id: assignment.sample,
                });
            }
            if !assigned_tracks.insert(assignment.track) {
                return Err(ValidationError::DuplicateSampleAssignment {
                    track_id: assignment.track,
                });
            }
        }

        let mut instrument_ids = HashSet::new();
        for (instrument_index, instrument) in self.instruments.iter().enumerate() {
            if !instrument_ids.insert(instrument.id) {
                return Err(ValidationError::DuplicateInstrumentId {
                    instrument_id: instrument.id,
                });
            }
            if instrument.name.trim().is_empty() {
                return Err(ValidationError::EmptyInstrumentName { instrument_index });
            }
            if let Some(sample) = instrument.sample {
                if !sample_ids.contains(&sample) {
                    return Err(ValidationError::InstrumentSampleNotFound {
                        instrument_id: instrument.id,
                        sample_id: sample,
                    });
                }
            }
        }

        let mut instrument_assigned_tracks = HashSet::new();
        for assignment in &self.track_instrument_assignments {
            if !track_ids.contains(&assignment.track) {
                return Err(ValidationError::InstrumentAssignmentTrackNotFound {
                    track_id: assignment.track,
                });
            }
            if !instrument_ids.contains(&assignment.instrument) {
                return Err(ValidationError::InstrumentAssignmentInstrumentNotFound {
                    instrument_id: assignment.instrument,
                });
            }
            if !instrument_assigned_tracks.insert(assignment.track) {
                return Err(ValidationError::DuplicateInstrumentAssignment {
                    track_id: assignment.track,
                });
            }
        }

        Ok(())
    }

    pub fn current_pattern_mut(&mut self) -> Option<&mut Pattern> {
        self.patterns.first_mut()
    }

    #[must_use]
    pub fn pattern(&self, index: usize) -> Option<&Pattern> {
        self.patterns.get(index)
    }

    pub fn pattern_mut(&mut self, index: usize) -> Option<&mut Pattern> {
        self.patterns.get_mut(index)
    }

    pub fn create_pattern(&mut self, row_count: usize) -> PatternId {
        let id = self.next_pattern_id();
        let name = format!("Pattern {:02}", id.0);
        self.patterns
            .push(Pattern::empty(id, name, row_count, self.tracks.len()));
        id
    }

    pub fn duplicate_pattern(&mut self, pattern_index: usize) -> Result<PatternId, EditError> {
        let mut pattern =
            self.patterns
                .get(pattern_index)
                .cloned()
                .ok_or(EditError::PatternOutOfBounds {
                    pattern: pattern_index,
                })?;
        let id = self.next_pattern_id();
        pattern.id = id;
        pattern.name = format!("Pattern {:02}", id.0);
        self.patterns.push(pattern);
        Ok(id)
    }

    pub fn delete_pattern(&mut self, pattern_index: usize) -> Result<Pattern, EditError> {
        if self.patterns.len() <= 1 {
            return Err(EditError::CannotDeleteLastPattern);
        }

        if pattern_index >= self.patterns.len() {
            return Err(EditError::PatternOutOfBounds {
                pattern: pattern_index,
            });
        }

        let removed = self.patterns.remove(pattern_index);
        self.sequence.retain(|id| *id != removed.id);
        if self.sequence.is_empty() {
            self.sequence.push(self.patterns[0].id);
        }
        Ok(removed)
    }

    pub fn resize_pattern(
        &mut self,
        pattern_index: usize,
        row_count: usize,
    ) -> Result<(), EditError> {
        if row_count == 0 {
            return Err(EditError::InvalidPatternLength { row_count });
        }

        let pattern =
            self.patterns
                .get_mut(pattern_index)
                .ok_or(EditError::PatternOutOfBounds {
                    pattern: pattern_index,
                })?;
        pattern.resize_rows(row_count, self.tracks.len());
        Ok(())
    }

    pub fn rename_pattern(
        &mut self,
        pattern_index: usize,
        name: impl Into<String>,
    ) -> Result<(), EditError> {
        let name = clean_name(name.into())?;
        let pattern =
            self.patterns
                .get_mut(pattern_index)
                .ok_or(EditError::PatternOutOfBounds {
                    pattern: pattern_index,
                })?;
        pattern.name = name;
        Ok(())
    }

    pub fn insert_pattern_row(
        &mut self,
        pattern_index: usize,
        row: usize,
    ) -> Result<(), EditError> {
        let pattern =
            self.patterns
                .get_mut(pattern_index)
                .ok_or(EditError::PatternOutOfBounds {
                    pattern: pattern_index,
                })?;
        pattern.insert_row(row, self.tracks.len())
    }

    pub fn delete_pattern_row(
        &mut self,
        pattern_index: usize,
        row: usize,
    ) -> Result<PatternRow, EditError> {
        let pattern =
            self.patterns
                .get_mut(pattern_index)
                .ok_or(EditError::PatternOutOfBounds {
                    pattern: pattern_index,
                })?;
        pattern.delete_row(row)
    }

    pub fn push_sequence_pattern(&mut self, pattern_id: PatternId) -> Result<(), EditError> {
        if !self.patterns.iter().any(|pattern| pattern.id == pattern_id) {
            return Err(EditError::PatternNotFound { pattern_id });
        }
        self.sequence.push(pattern_id);
        Ok(())
    }

    pub fn remove_sequence_position(&mut self, position: usize) -> Result<PatternId, EditError> {
        if position >= self.sequence.len() {
            return Err(EditError::SequenceOutOfBounds { position });
        }
        Ok(self.sequence.remove(position))
    }

    pub fn duplicate_sequence_position(&mut self, position: usize) -> Result<(), EditError> {
        let pattern_id = *self
            .sequence
            .get(position)
            .ok_or(EditError::SequenceOutOfBounds { position })?;
        self.sequence.insert(position.saturating_add(1), pattern_id);
        Ok(())
    }

    pub fn set_sequence_pattern(
        &mut self,
        position: usize,
        pattern_id: PatternId,
    ) -> Result<(), EditError> {
        if !self.patterns.iter().any(|pattern| pattern.id == pattern_id) {
            return Err(EditError::PatternNotFound { pattern_id });
        }

        let sequence_pattern = self
            .sequence
            .get_mut(position)
            .ok_or(EditError::SequenceOutOfBounds { position })?;
        *sequence_pattern = pattern_id;
        Ok(())
    }

    pub fn move_sequence_position(&mut self, from: usize, to: usize) -> Result<(), EditError> {
        if from >= self.sequence.len() {
            return Err(EditError::SequenceOutOfBounds { position: from });
        }
        if to >= self.sequence.len() {
            return Err(EditError::SequenceOutOfBounds { position: to });
        }

        if from == to {
            return Ok(());
        }

        let pattern_id = self.sequence.remove(from);
        self.sequence.insert(to, pattern_id);
        Ok(())
    }

    pub fn create_track(&mut self) -> TrackId {
        let index = self.tracks.len();
        let id = self.next_track_id();
        self.tracks.push(Track {
            id,
            name: default_track_name(index),
            midi_channel: default_midi_channel(index),
            muted: false,
            solo: false,
            armed: false,
        });

        for pattern in &mut self.patterns {
            pattern.append_track();
        }

        id
    }

    pub fn duplicate_track(&mut self, track_index: usize) -> Result<TrackId, EditError> {
        let source = self
            .tracks
            .get(track_index)
            .cloned()
            .ok_or(EditError::TrackOutOfBounds { track: track_index })?;
        let id = self.next_track_id();
        self.tracks.push(Track {
            id,
            name: format!("{} Copy", source.name),
            midi_channel: source.midi_channel,
            muted: source.muted,
            solo: false,
            armed: false,
        });

        for pattern in &mut self.patterns {
            pattern.duplicate_track(track_index)?;
        }

        if let Some(source_assignment) = self.sample_assignment_for_track(source.id).cloned() {
            self.assign_sample_to_track(id, source_assignment.sample)?;
        }
        if let Some(source_assignment) = self.instrument_assignment_for_track(source.id).cloned() {
            self.assign_instrument_to_track(id, source_assignment.instrument)?;
        }

        Ok(id)
    }

    pub fn delete_track(&mut self, track_index: usize) -> Result<Track, EditError> {
        if self.tracks.len() <= 1 {
            return Err(EditError::CannotDeleteLastTrack);
        }

        if track_index >= self.tracks.len() {
            return Err(EditError::TrackOutOfBounds { track: track_index });
        }

        for pattern in &mut self.patterns {
            pattern.remove_track(track_index)?;
        }

        let removed = self.tracks.remove(track_index);
        self.unassign_sample_from_track(removed.id);
        self.unassign_instrument_from_track(removed.id);
        Ok(removed)
    }

    pub fn move_track(&mut self, from: usize, to: usize) -> Result<(), EditError> {
        if from >= self.tracks.len() {
            return Err(EditError::TrackOutOfBounds { track: from });
        }
        if to >= self.tracks.len() {
            return Err(EditError::TrackOutOfBounds { track: to });
        }
        if from == to {
            return Ok(());
        }

        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);

        for pattern in &mut self.patterns {
            pattern.move_track(from, to)?;
        }

        Ok(())
    }

    pub fn rename_track(
        &mut self,
        track_index: usize,
        name: impl Into<String>,
    ) -> Result<(), EditError> {
        let name = clean_name(name.into())?;
        let track = self
            .tracks
            .get_mut(track_index)
            .ok_or(EditError::TrackOutOfBounds { track: track_index })?;
        track.name = name;
        Ok(())
    }

    pub fn toggle_mute(&mut self, track_index: usize) -> Result<(), EditError> {
        let track = self
            .tracks
            .get_mut(track_index)
            .ok_or(EditError::TrackOutOfBounds { track: track_index })?;
        track.muted = !track.muted;
        Ok(())
    }

    pub fn toggle_solo(&mut self, track_index: usize) -> Result<(), EditError> {
        let track = self
            .tracks
            .get_mut(track_index)
            .ok_or(EditError::TrackOutOfBounds { track: track_index })?;
        track.solo = !track.solo;
        Ok(())
    }

    pub fn set_track_midi_channel(
        &mut self,
        track_index: usize,
        midi_channel: u8,
    ) -> Result<(), EditError> {
        if !(1..=16).contains(&midi_channel) {
            return Err(EditError::InvalidMidiChannel { midi_channel });
        }

        let track = self
            .tracks
            .get_mut(track_index)
            .ok_or(EditError::TrackOutOfBounds { track: track_index })?;
        track.midi_channel = midi_channel;
        Ok(())
    }

    pub fn upsert_sample_reference(
        &mut self,
        path: impl Into<String>,
        name: impl Into<String>,
    ) -> SampleId {
        let path = path.into();
        let name = name.into();
        if let Some(sample) = self.samples.iter_mut().find(|sample| sample.path == path) {
            sample.name = name;
            return sample.id;
        }

        let id = self.next_sample_id();
        self.samples.push(SampleReference {
            id,
            name,
            path,
            root_pitch: 60,
            gain: 1.0,
            playback: SamplePlaybackSettings::default(),
        });
        id
    }

    pub fn assign_sample_to_track(
        &mut self,
        track: TrackId,
        sample: SampleId,
    ) -> Result<(), EditError> {
        if !self.tracks.iter().any(|existing| existing.id == track) {
            return Err(EditError::TrackIdNotFound { track_id: track });
        }
        if !self.samples.iter().any(|existing| existing.id == sample) {
            return Err(EditError::SampleNotFound { sample_id: sample });
        }

        if let Some(existing) = self
            .sample_assignments
            .iter_mut()
            .find(|assignment| assignment.track == track)
        {
            existing.sample = sample;
        } else {
            self.sample_assignments
                .push(TrackSampleAssignment { track, sample });
        }
        let instrument = self.upsert_sample_instrument(sample)?;
        self.assign_instrument_to_track(track, instrument)?;
        Ok(())
    }

    pub fn replace_track_sample(
        &mut self,
        track: TrackId,
        path: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<SampleId, EditError> {
        let sample = self.upsert_sample_reference(path, name);
        self.assign_sample_to_track(track, sample)?;
        Ok(sample)
    }

    pub fn set_sample_frame_window(
        &mut self,
        sample: SampleId,
        start_frame: Option<usize>,
        end_frame: Option<usize>,
    ) -> Result<(), EditError> {
        let reference = self
            .samples
            .iter_mut()
            .find(|reference| reference.id == sample)
            .ok_or(EditError::SampleNotFound { sample_id: sample })?;
        let mut playback = reference.playback;
        playback.start_frame = start_frame;
        playback.end_frame = end_frame;
        if let Err(ValidationError::InvalidSampleFrameWindow { .. }) =
            validate_sample_playback_settings(0, playback)
        {
            return Err(EditError::InvalidSampleFrameWindow);
        }
        reference.playback.start_frame = start_frame;
        reference.playback.end_frame = end_frame;
        Ok(())
    }

    pub fn set_sample_loop(
        &mut self,
        sample: SampleId,
        mode: SamplePlaybackMode,
        loop_start_frame: Option<usize>,
        loop_end_frame: Option<usize>,
    ) -> Result<(), EditError> {
        let reference = self
            .samples
            .iter_mut()
            .find(|reference| reference.id == sample)
            .ok_or(EditError::SampleNotFound { sample_id: sample })?;
        let mut playback = reference.playback;
        playback.mode = mode;
        playback.loop_start_frame = loop_start_frame;
        playback.loop_end_frame = loop_end_frame;
        if let Err(ValidationError::InvalidSampleLoopWindow { .. }) =
            validate_sample_playback_settings(0, playback)
        {
            return Err(EditError::InvalidSampleLoopWindow);
        }
        reference.playback.mode = mode;
        reference.playback.loop_start_frame = loop_start_frame;
        reference.playback.loop_end_frame = loop_end_frame;
        Ok(())
    }

    pub fn set_sample_envelope(
        &mut self,
        sample: SampleId,
        envelope: SampleEnvelope,
    ) -> Result<(), EditError> {
        let reference = self
            .samples
            .iter_mut()
            .find(|reference| reference.id == sample)
            .ok_or(EditError::SampleNotFound { sample_id: sample })?;
        let mut playback = reference.playback;
        playback.envelope = envelope;
        if let Err(ValidationError::InvalidSampleEnvelope { .. }) =
            validate_sample_playback_settings(0, playback)
        {
            return Err(EditError::InvalidSampleEnvelope);
        }
        reference.playback.envelope = envelope;
        Ok(())
    }

    pub fn unassign_sample_from_track(&mut self, track: TrackId) {
        self.sample_assignments
            .retain(|assignment| assignment.track != track);
        self.unassign_instrument_from_track(track);
    }

    pub fn remove_sample_reference(&mut self, sample: SampleId) -> Result<(), EditError> {
        if !self.samples.iter().any(|existing| existing.id == sample) {
            return Err(EditError::SampleNotFound { sample_id: sample });
        }
        if self.is_sample_assigned(sample) {
            return Err(EditError::SampleInUse { sample_id: sample });
        }

        self.samples.retain(|reference| reference.id != sample);
        self.instruments
            .retain(|instrument| instrument.sample != Some(sample));
        Ok(())
    }

    pub fn prune_unused_sample_references(&mut self) -> usize {
        let assigned_instruments = self
            .track_instrument_assignments
            .iter()
            .map(|assignment| assignment.instrument)
            .collect::<HashSet<_>>();
        self.instruments.retain(|instrument| {
            instrument.sample.is_none() || assigned_instruments.contains(&instrument.id)
        });
        let assigned_samples = self
            .sample_assignments
            .iter()
            .map(|assignment| assignment.sample)
            .chain(
                self.instruments
                    .iter()
                    .filter_map(|instrument| instrument.sample),
            )
            .collect::<HashSet<_>>();
        let before = self.samples.len();
        self.samples
            .retain(|sample| assigned_samples.contains(&sample.id));
        before - self.samples.len()
    }

    #[must_use]
    pub fn is_sample_assigned(&self, sample: SampleId) -> bool {
        self.sample_assignments
            .iter()
            .any(|assignment| assignment.sample == sample)
            || self
                .track_instrument_assignments
                .iter()
                .filter_map(|assignment| self.instrument_for_id(assignment.instrument))
                .any(|instrument| instrument.sample == Some(sample))
    }

    #[must_use]
    pub fn sample_assignment_for_track(&self, track: TrackId) -> Option<&TrackSampleAssignment> {
        self.sample_assignments
            .iter()
            .find(|assignment| assignment.track == track)
    }

    #[must_use]
    pub fn sample_for_id(&self, sample: SampleId) -> Option<&SampleReference> {
        self.samples.iter().find(|reference| reference.id == sample)
    }

    pub fn sample_for_id_mut(&mut self, sample: SampleId) -> Option<&mut SampleReference> {
        self.samples
            .iter_mut()
            .find(|reference| reference.id == sample)
    }

    #[must_use]
    pub fn sample_for_track(&self, track: TrackId) -> Option<&SampleReference> {
        self.sample_assignment_for_track(track)
            .and_then(|assignment| self.sample_for_id(assignment.sample))
            .or_else(|| {
                self.instrument_for_track(track)
                    .and_then(|instrument| instrument.sample)
                    .and_then(|sample| self.sample_for_id(sample))
            })
    }

    pub fn upsert_sample_instrument(
        &mut self,
        sample: SampleId,
    ) -> Result<InstrumentId, EditError> {
        if !self.samples.iter().any(|existing| existing.id == sample) {
            return Err(EditError::SampleNotFound { sample_id: sample });
        }
        if let Some(instrument) = self
            .instruments
            .iter()
            .find(|instrument| instrument.sample == Some(sample))
        {
            return Ok(instrument.id);
        }
        let sample_name = self.sample_for_id(sample).map_or_else(
            || format!("Sample {}", sample.0),
            |sample| sample.name.clone(),
        );
        let id = self.next_instrument_id();
        self.instruments.push(Instrument {
            id,
            name: sample_name,
            sample: Some(sample),
        });
        Ok(id)
    }

    pub fn assign_instrument_to_track(
        &mut self,
        track: TrackId,
        instrument: InstrumentId,
    ) -> Result<(), EditError> {
        if !self.tracks.iter().any(|existing| existing.id == track) {
            return Err(EditError::TrackIdNotFound { track_id: track });
        }
        if !self
            .instruments
            .iter()
            .any(|existing| existing.id == instrument)
        {
            return Err(EditError::InstrumentNotFound {
                instrument_id: instrument,
            });
        }
        if let Some(existing) = self
            .track_instrument_assignments
            .iter_mut()
            .find(|assignment| assignment.track == track)
        {
            existing.instrument = instrument;
        } else {
            self.track_instrument_assignments
                .push(TrackInstrumentAssignment { track, instrument });
        }
        Ok(())
    }

    pub fn unassign_instrument_from_track(&mut self, track: TrackId) {
        self.track_instrument_assignments
            .retain(|assignment| assignment.track != track);
    }

    #[must_use]
    pub fn instrument_for_id(&self, instrument: InstrumentId) -> Option<&Instrument> {
        self.instruments
            .iter()
            .find(|reference| reference.id == instrument)
    }

    #[must_use]
    pub fn instrument_assignment_for_track(
        &self,
        track: TrackId,
    ) -> Option<&TrackInstrumentAssignment> {
        self.track_instrument_assignments
            .iter()
            .find(|assignment| assignment.track == track)
    }

    #[must_use]
    pub fn instrument_for_track(&self, track: TrackId) -> Option<&Instrument> {
        self.instrument_assignment_for_track(track)
            .and_then(|assignment| self.instrument_for_id(assignment.instrument))
    }

    pub fn ensure_instruments_for_sample_assignments(&mut self) -> Result<(), EditError> {
        let assignments = self.sample_assignments.clone();
        for assignment in assignments {
            let instrument = self.upsert_sample_instrument(assignment.sample)?;
            self.assign_instrument_to_track(assignment.track, instrument)?;
        }
        Ok(())
    }

    fn next_track_id(&self) -> TrackId {
        let next = self
            .tracks
            .iter()
            .map(|track| track.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        TrackId(next)
    }

    fn next_pattern_id(&self) -> PatternId {
        let next = self
            .patterns
            .iter()
            .map(|pattern| pattern.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        PatternId(next)
    }

    fn next_sample_id(&self) -> SampleId {
        let next = self
            .samples
            .iter()
            .map(|sample| sample.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        SampleId(next)
    }

    fn next_instrument_id(&self) -> InstrumentId {
        let next = self
            .instruments
            .iter()
            .map(|instrument| instrument.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        InstrumentId(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("song title cannot be empty")]
    EmptySongTitle,
    #[error("invalid BPM: {bpm}")]
    InvalidBpm { bpm: u16 },
    #[error("invalid lines per beat: {lines_per_beat}")]
    InvalidLinesPerBeat { lines_per_beat: u8 },
    #[error("invalid swing value")]
    InvalidSwing,
    #[error("song must contain at least one track")]
    NoTracks,
    #[error("song must contain at least one pattern")]
    NoPatterns,
    #[error("sequence must contain at least one pattern reference")]
    EmptySequence,
    #[error("duplicate track id: {track_id:?}")]
    DuplicateTrackId { track_id: TrackId },
    #[error("track {track_index} name cannot be empty")]
    EmptyTrackName { track_index: usize },
    #[error("track {track_index} has invalid MIDI channel {midi_channel}")]
    InvalidTrackMidiChannel {
        track_index: usize,
        midi_channel: u8,
    },
    #[error("duplicate pattern id: {pattern_id:?}")]
    DuplicatePatternId { pattern_id: PatternId },
    #[error("pattern {pattern_index} name cannot be empty")]
    EmptyPatternName { pattern_index: usize },
    #[error("pattern {pattern_index} must contain at least one row")]
    EmptyPattern { pattern_index: usize },
    #[error("pattern {pattern_index} row {row_index} has {actual} cells, expected {expected}")]
    PatternRowCellCountMismatch {
        pattern_index: usize,
        row_index: usize,
        expected: usize,
        actual: usize,
    },
    #[error("pattern {pattern_index} row {row_index} track {track_index} has invalid note pitch {pitch}")]
    InvalidNotePitch {
        pattern_index: usize,
        row_index: usize,
        track_index: usize,
        pitch: u8,
    },
    #[error("pattern {pattern_index} row {row_index} track {track_index} has invalid velocity {velocity}")]
    InvalidVelocity {
        pattern_index: usize,
        row_index: usize,
        track_index: usize,
        velocity: u8,
    },
    #[error("pattern {pattern_index} row {row_index} track {track_index} has invalid gate {gate}")]
    InvalidGate {
        pattern_index: usize,
        row_index: usize,
        track_index: usize,
        gate: u8,
    },
    #[error("sequence position {position} references missing pattern {pattern_id:?}")]
    SequencePatternNotFound {
        position: usize,
        pattern_id: PatternId,
    },
    #[error("duplicate sample id {sample_id:?}")]
    DuplicateSampleId { sample_id: SampleId },
    #[error("sample {sample_index} name cannot be empty")]
    EmptySampleName { sample_index: usize },
    #[error("sample {sample_index} path cannot be empty")]
    EmptySamplePath { sample_index: usize },
    #[error("sample {sample_index} has invalid root pitch {root_pitch}")]
    InvalidSampleRootPitch { sample_index: usize, root_pitch: u8 },
    #[error("sample {sample_index} has invalid gain")]
    InvalidSampleGain { sample_index: usize },
    #[error("sample {sample_index} has invalid frame window")]
    InvalidSampleFrameWindow { sample_index: usize },
    #[error("sample {sample_index} has invalid loop window")]
    InvalidSampleLoopWindow { sample_index: usize },
    #[error("sample {sample_index} has invalid envelope")]
    InvalidSampleEnvelope { sample_index: usize },
    #[error("sample assignment references missing track {track_id:?}")]
    SampleAssignmentTrackNotFound { track_id: TrackId },
    #[error("sample assignment references missing sample {sample_id:?}")]
    SampleAssignmentSampleNotFound { sample_id: SampleId },
    #[error("track {track_id:?} has multiple sample assignments")]
    DuplicateSampleAssignment { track_id: TrackId },
    #[error("duplicate instrument id {instrument_id:?}")]
    DuplicateInstrumentId { instrument_id: InstrumentId },
    #[error("instrument {instrument_index} name cannot be empty")]
    EmptyInstrumentName { instrument_index: usize },
    #[error("instrument {instrument_id:?} references missing sample {sample_id:?}")]
    InstrumentSampleNotFound {
        instrument_id: InstrumentId,
        sample_id: SampleId,
    },
    #[error("instrument assignment references missing track {track_id:?}")]
    InstrumentAssignmentTrackNotFound { track_id: TrackId },
    #[error("instrument assignment references missing instrument {instrument_id:?}")]
    InstrumentAssignmentInstrumentNotFound { instrument_id: InstrumentId },
    #[error("track {track_id:?} has multiple instrument assignments")]
    DuplicateInstrumentAssignment { track_id: TrackId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongMetadata {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportSettings {
    pub bpm: u16,
    pub lines_per_beat: u8,
    pub swing: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub midi_channel: u8,
    pub muted: bool,
    pub solo: bool,
    pub armed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleReference {
    pub id: SampleId,
    pub name: String,
    pub path: String,
    pub root_pitch: u8,
    pub gain: f32,
    #[serde(default, skip_serializing_if = "SamplePlaybackSettings::is_default")]
    pub playback: SamplePlaybackSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SamplePlaybackMode {
    #[default]
    OneShot,
    Loop,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleEnvelope {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain: f32,
    pub release_seconds: f32,
}

impl Default for SampleEnvelope {
    fn default() -> Self {
        Self {
            attack_seconds: 0.0,
            decay_seconds: 0.0,
            sustain: 1.0,
            release_seconds: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplePlaybackSettings {
    pub mode: SamplePlaybackMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_start_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_end_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "SampleEnvelope::is_default")]
    pub envelope: SampleEnvelope,
}

impl Default for SamplePlaybackSettings {
    fn default() -> Self {
        Self {
            mode: SamplePlaybackMode::OneShot,
            start_frame: None,
            end_frame: None,
            loop_start_frame: None,
            loop_end_frame: None,
            envelope: SampleEnvelope::default(),
        }
    }
}

impl SampleEnvelope {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl SamplePlaybackSettings {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSampleAssignment {
    pub track: TrackId,
    pub sample: SampleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: InstrumentId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<SampleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInstrumentAssignment {
    pub track: TrackId,
    pub instrument: InstrumentId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
    pub id: PatternId,
    pub name: String,
    pub rows: Vec<PatternRow>,
}

impl Pattern {
    #[must_use]
    pub fn empty(
        id: PatternId,
        name: impl Into<String>,
        row_count: usize,
        track_count: usize,
    ) -> Self {
        let rows = (0..row_count)
            .map(|_| PatternRow {
                cells: vec![PatternCell::default(); track_count],
            })
            .collect();

        Self {
            id,
            name: name.into(),
            rows,
        }
    }

    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn cell(&self, row: usize, track: usize) -> Option<&PatternCell> {
        self.rows
            .get(row)
            .and_then(|pattern_row| pattern_row.cells.get(track))
    }

    pub fn cell_mut(&mut self, row: usize, track: usize) -> Option<&mut PatternCell> {
        self.rows
            .get_mut(row)
            .and_then(|pattern_row| pattern_row.cells.get_mut(track))
    }

    pub fn set_note(
        &mut self,
        row: usize,
        track: usize,
        note: NoteEvent,
        velocity: u8,
    ) -> Result<(), EditError> {
        self.set_note_event(row, track, note, Some(velocity))
    }

    pub fn set_note_event(
        &mut self,
        row: usize,
        track: usize,
        note: NoteEvent,
        velocity: Option<u8>,
    ) -> Result<(), EditError> {
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        cell.note = Some(note);
        cell.velocity = velocity.map(|value| value.min(0x7f));
        Ok(())
    }

    pub fn set_velocity(
        &mut self,
        row: usize,
        track: usize,
        velocity: u8,
    ) -> Result<(), EditError> {
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        cell.velocity = Some(velocity.min(0x7f));
        Ok(())
    }

    pub fn clear_cell(&mut self, row: usize, track: usize) -> Result<(), EditError> {
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        *cell = PatternCell::default();
        Ok(())
    }

    pub fn set_cell(
        &mut self,
        row: usize,
        track: usize,
        cell: PatternCell,
    ) -> Result<(), EditError> {
        let target = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        *target = cell;
        Ok(())
    }

    pub fn resize_rows(&mut self, row_count: usize, track_count: usize) {
        self.rows
            .resize_with(row_count, || PatternRow::empty(track_count));
    }

    pub fn insert_row(&mut self, row: usize, track_count: usize) -> Result<(), EditError> {
        if row > self.rows.len() {
            return Err(EditError::RowOutOfBounds { row });
        }
        self.rows.insert(row, PatternRow::empty(track_count));
        Ok(())
    }

    pub fn delete_row(&mut self, row: usize) -> Result<PatternRow, EditError> {
        if self.rows.len() <= 1 {
            return Err(EditError::CannotDeleteLastPatternRow);
        }
        if row >= self.rows.len() {
            return Err(EditError::RowOutOfBounds { row });
        }
        Ok(self.rows.remove(row))
    }

    fn append_track(&mut self) {
        for row in &mut self.rows {
            row.cells.push(PatternCell::default());
        }
    }

    fn duplicate_track(&mut self, track: usize) -> Result<(), EditError> {
        for row in &mut self.rows {
            let cell = row
                .cells
                .get(track)
                .cloned()
                .ok_or(EditError::TrackOutOfBounds { track })?;
            row.cells.push(cell);
        }
        Ok(())
    }

    fn remove_track(&mut self, track: usize) -> Result<(), EditError> {
        for row in &mut self.rows {
            if track >= row.cells.len() {
                return Err(EditError::TrackOutOfBounds { track });
            }
            row.cells.remove(track);
        }
        Ok(())
    }

    fn move_track(&mut self, from: usize, to: usize) -> Result<(), EditError> {
        for row in &mut self.rows {
            if from >= row.cells.len() {
                return Err(EditError::TrackOutOfBounds { track: from });
            }
            if to >= row.cells.len() {
                return Err(EditError::TrackOutOfBounds { track: to });
            }

            let cell = row.cells.remove(from);
            row.cells.insert(to, cell);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("cell out of bounds: row {row}, track {track}")]
    CellOutOfBounds { row: usize, track: usize },
    #[error("row out of bounds: row {row}")]
    RowOutOfBounds { row: usize },
    #[error("track out of bounds: track {track}")]
    TrackOutOfBounds { track: usize },
    #[error("track not found: track id {track_id:?}")]
    TrackIdNotFound { track_id: TrackId },
    #[error("cannot delete the last track")]
    CannotDeleteLastTrack,
    #[error("pattern out of bounds: pattern {pattern}")]
    PatternOutOfBounds { pattern: usize },
    #[error("pattern not found: pattern id {pattern_id:?}")]
    PatternNotFound { pattern_id: PatternId },
    #[error("cannot delete the last pattern")]
    CannotDeleteLastPattern,
    #[error("sequence out of bounds: position {position}")]
    SequenceOutOfBounds { position: usize },
    #[error("invalid pattern length: {row_count}")]
    InvalidPatternLength { row_count: usize },
    #[error("cannot delete the last pattern row")]
    CannotDeleteLastPatternRow,
    #[error("invalid MIDI channel: {midi_channel}")]
    InvalidMidiChannel { midi_channel: u8 },
    #[error("sample not found: sample id {sample_id:?}")]
    SampleNotFound { sample_id: SampleId },
    #[error("sample is still assigned: sample id {sample_id:?}")]
    SampleInUse { sample_id: SampleId },
    #[error("invalid sample frame window")]
    InvalidSampleFrameWindow,
    #[error("invalid sample loop window")]
    InvalidSampleLoopWindow,
    #[error("invalid sample envelope")]
    InvalidSampleEnvelope,
    #[error("instrument not found: instrument id {instrument_id:?}")]
    InstrumentNotFound { instrument_id: InstrumentId },
    #[error("name cannot be empty")]
    EmptyName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternRow {
    pub cells: Vec<PatternCell>,
}

impl PatternRow {
    fn empty(track_count: usize) -> Self {
        Self {
            cells: vec![PatternCell::default(); track_count],
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternCell {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<NoteEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<TrackerCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NoteEvent {
    Note { pitch: u8 },
    NoteOff,
    NoteCut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackerCommand {
    pub code: u8,
    pub value: u8,
}

impl TrackerCommand {
    pub const DELAY_CODE: u8 = b'D';
    pub const RETRIGGER_CODE: u8 = b'R';

    #[must_use]
    pub const fn delay(value: u8) -> Self {
        Self {
            code: Self::DELAY_CODE,
            value,
        }
    }

    #[must_use]
    pub const fn retrigger(count: u8) -> Self {
        Self {
            code: Self::RETRIGGER_CODE,
            value: count,
        }
    }

    #[must_use]
    pub fn from_code_char(code: char, value: u8) -> Self {
        Self {
            code: code.to_ascii_uppercase() as u8,
            value,
        }
    }

    #[must_use]
    pub fn display_code(self) -> char {
        char::from(self.code).to_ascii_uppercase()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub track: usize,
    pub field: CellField,
    pub digit: usize,
}

impl Cursor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            row: 0,
            track: 0,
            field: CellField::Note,
            digit: 0,
        }
    }

    pub fn move_in(&mut self, direction: Direction, row_count: usize, track_count: usize) {
        match direction {
            Direction::Up => self.row = self.row.saturating_sub(1),
            Direction::Down => {
                self.row = self.row.saturating_add(1).min(row_count.saturating_sub(1));
            }
            Direction::Left => self.move_left(),
            Direction::Right => self.move_right(track_count),
        }

        self.clamp(row_count, track_count);
    }

    pub fn clamp(&mut self, row_count: usize, track_count: usize) {
        self.row = self.row.min(row_count.saturating_sub(1));
        self.track = self.track.min(track_count.saturating_sub(1));
    }

    fn move_left(&mut self) {
        match self.field {
            CellField::Velocity => self.field = CellField::Note,
            CellField::Note if self.track > 0 => {
                self.track -= 1;
                self.field = CellField::Velocity;
            }
            CellField::Note => {}
        }
    }

    fn move_right(&mut self, track_count: usize) {
        match self.field {
            CellField::Note => self.field = CellField::Velocity,
            CellField::Velocity if self.track + 1 < track_count => {
                self.track += 1;
                self.field = CellField::Note;
            }
            CellField::Velocity => {}
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellField {
    Note,
    Velocity,
}

impl fmt::Display for CellField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellField::Note => f.write_str("NOTE"),
            CellField::Velocity => f.write_str("VEL"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn default_track_name(index: usize) -> String {
    match index {
        0 => "Drums".to_string(),
        1 => "Bass".to_string(),
        2 => "Lead".to_string(),
        3 => "Pad".to_string(),
        _ => format!("Track {:02}", index + 1),
    }
}

fn default_midi_channel(index: usize) -> u8 {
    match index {
        0 => 10,
        _ => index as u8,
    }
}

fn clean_name(name: String) -> Result<String, EditError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        Err(EditError::EmptyName)
    } else {
        Ok(name)
    }
}

fn validate_sample_playback_settings(
    sample_index: usize,
    settings: SamplePlaybackSettings,
) -> Result<(), ValidationError> {
    if let (Some(start_frame), Some(end_frame)) = (settings.start_frame, settings.end_frame) {
        if start_frame >= end_frame {
            return Err(ValidationError::InvalidSampleFrameWindow { sample_index });
        }
    }
    match (
        settings.mode,
        settings.loop_start_frame,
        settings.loop_end_frame,
    ) {
        (SamplePlaybackMode::Loop, Some(loop_start), Some(loop_end)) if loop_start < loop_end => {}
        (SamplePlaybackMode::Loop, _, _) => {
            return Err(ValidationError::InvalidSampleLoopWindow { sample_index });
        }
        (_, Some(loop_start), Some(loop_end)) if loop_start < loop_end => {}
        (_, Some(_), Some(_)) | (_, Some(_), None) | (_, None, Some(_)) => {
            return Err(ValidationError::InvalidSampleLoopWindow { sample_index });
        }
        (_, None, None) => {}
    }
    let envelope = settings.envelope;
    if !envelope.attack_seconds.is_finite()
        || envelope.attack_seconds < 0.0
        || !envelope.decay_seconds.is_finite()
        || envelope.decay_seconds < 0.0
        || !envelope.release_seconds.is_finite()
        || envelope.release_seconds < 0.0
        || !envelope.sustain.is_finite()
        || !(0.0..=1.0).contains(&envelope.sustain)
    {
        return Err(ValidationError::InvalidSampleEnvelope { sample_index });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_song_matches_mvp_shape() {
        let song = Song::empty();

        assert_eq!(song.transport.bpm, DEFAULT_BPM);
        assert_eq!(song.transport.lines_per_beat, DEFAULT_LINES_PER_BEAT);
        assert_eq!(song.tracks.len(), DEFAULT_TRACK_COUNT);
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.patterns[0].rows.len(), DEFAULT_PATTERN_LEN);
        assert_eq!(song.patterns[0].rows[0].cells.len(), DEFAULT_TRACK_COUNT);
        assert_eq!(song.sequence, vec![PatternId(1)]);
    }

    #[test]
    fn cursor_navigation_is_clamped_to_pattern_bounds() {
        let mut cursor = Cursor::new();

        cursor.move_in(Direction::Up, 64, 4);
        cursor.move_in(Direction::Left, 64, 4);
        assert_eq!(cursor, Cursor::new());

        for _ in 0..100 {
            cursor.move_in(Direction::Down, 64, 4);
        }
        assert_eq!(cursor.row, 63);

        for _ in 0..20 {
            cursor.move_in(Direction::Right, 64, 4);
        }
        assert_eq!(cursor.track, 3);
        assert_eq!(cursor.field, CellField::Velocity);
    }

    #[test]
    fn cursor_moves_between_note_and_velocity_fields() {
        let mut cursor = Cursor::new();

        cursor.move_in(Direction::Right, 64, 4);
        assert_eq!(cursor.field, CellField::Velocity);
        assert_eq!(cursor.track, 0);

        cursor.move_in(Direction::Right, 64, 4);
        assert_eq!(cursor.field, CellField::Note);
        assert_eq!(cursor.track, 1);

        cursor.move_in(Direction::Left, 64, 4);
        assert_eq!(cursor.field, CellField::Velocity);
        assert_eq!(cursor.track, 0);
    }

    #[test]
    fn pattern_cell_edits_are_addressed_by_row_and_track() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern 01", 64, 4);

        pattern
            .set_note(2, 1, NoteEvent::Note { pitch: 60 }, 0x70)
            .expect("set note");
        assert_eq!(
            pattern.cell(2, 1).expect("cell").note,
            Some(NoteEvent::Note { pitch: 60 })
        );
        assert_eq!(pattern.cell(2, 1).expect("cell").velocity, Some(0x70));

        pattern.set_velocity(2, 1, 0xff).expect("set velocity");
        assert_eq!(pattern.cell(2, 1).expect("cell").velocity, Some(0x7f));

        pattern.clear_cell(2, 1).expect("clear cell");
        assert_eq!(pattern.cell(2, 1), Some(&PatternCell::default()));
    }

    #[test]
    fn pattern_cells_can_be_replaced_whole() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern 01", 64, 4);
        let cell = PatternCell {
            note: Some(NoteEvent::Note { pitch: 72 }),
            velocity: Some(0x40),
            gate: None,
            command: None,
        };

        pattern.set_cell(4, 2, cell.clone()).expect("set cell");

        assert_eq!(pattern.cell(4, 2), Some(&cell));
    }

    #[test]
    fn note_events_can_be_set_without_velocity() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern 01", 64, 4);

        pattern
            .set_note_event(0, 0, NoteEvent::NoteOff, None)
            .expect("set note off");

        let cell = pattern.cell(0, 0).expect("cell");
        assert_eq!(cell.note, Some(NoteEvent::NoteOff));
        assert_eq!(cell.velocity, None);
    }

    #[test]
    fn editing_outside_pattern_returns_error() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern 01", 64, 4);

        let error = pattern
            .set_note(100, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect_err("out of bounds");

        assert_eq!(error, EditError::CellOutOfBounds { row: 100, track: 0 });
    }

    #[test]
    fn creating_track_updates_every_pattern_row() {
        let mut song = Song::empty();

        let id = song.create_track();

        assert_eq!(id, TrackId(5));
        assert_eq!(song.tracks.len(), 5);
        assert!(song.patterns.iter().all(|pattern| {
            pattern
                .rows
                .iter()
                .all(|row| row.cells.len() == song.tracks.len())
        }));
    }

    #[test]
    fn duplicating_track_copies_cells_and_track_settings() {
        let mut song = Song::empty();
        song.tracks[1].midi_channel = 12;
        song.tracks[1].muted = true;
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x64)
            .expect("set note");

        let id = song.duplicate_track(1).expect("duplicate track");

        assert_eq!(id, TrackId(5));
        assert_eq!(song.tracks.len(), 5);
        assert_eq!(song.tracks[4].name, "Bass Copy");
        assert_eq!(song.tracks[4].midi_channel, 12);
        assert!(song.tracks[4].muted);
        assert!(!song.tracks[4].solo);
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(0, 4)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );
        assert_eq!(
            song.duplicate_track(99).expect_err("track out of bounds"),
            EditError::TrackOutOfBounds { track: 99 }
        );
    }

    #[test]
    fn deleting_track_updates_every_pattern_row() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x70)
            .expect("set note");

        let removed = song.delete_track(1).expect("delete track");

        assert_eq!(removed.name, "Bass");
        assert_eq!(song.tracks.len(), 3);
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("shifted cell")
                .note,
            Some(NoteEvent::Note { pitch: 64 })
        );
        assert!(song.patterns.iter().all(|pattern| {
            pattern
                .rows
                .iter()
                .all(|row| row.cells.len() == song.tracks.len())
        }));
    }

    #[test]
    fn cannot_delete_last_track() {
        let mut song = Song::empty();

        while song.tracks.len() > 1 {
            song.delete_track(0).expect("delete track");
        }

        let error = song.delete_track(0).expect_err("last track remains");
        assert_eq!(error, EditError::CannotDeleteLastTrack);
    }

    #[test]
    fn moving_track_reorders_tracks_and_pattern_cells() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
            .expect("set bass note");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x70)
            .expect("set lead note");

        song.move_track(1, 2).expect("move track");

        assert_eq!(song.tracks[1].name, "Lead");
        assert_eq!(song.tracks[2].name, "Bass");
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("lead cell")
                .note,
            Some(NoteEvent::Note { pitch: 64 })
        );
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(0, 2)
                .expect("bass cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );
        assert_eq!(
            song.move_track(99, 0).expect_err("source out of range"),
            EditError::TrackOutOfBounds { track: 99 }
        );
        assert_eq!(
            song.move_track(0, 99).expect_err("target out of range"),
            EditError::TrackOutOfBounds { track: 99 }
        );
    }

    #[test]
    fn mute_and_solo_toggle_track_flags() {
        let mut song = Song::empty();

        song.toggle_mute(0).expect("mute");
        song.toggle_solo(1).expect("solo");

        assert!(song.tracks[0].muted);
        assert!(song.tracks[1].solo);
    }

    #[test]
    fn track_midi_channel_can_be_changed_with_validation() {
        let mut song = Song::empty();

        song.set_track_midi_channel(1, 12)
            .expect("set MIDI channel");

        assert_eq!(song.tracks[1].midi_channel, 12);
        assert_eq!(
            song.set_track_midi_channel(1, 0)
                .expect_err("channel out of range"),
            EditError::InvalidMidiChannel { midi_channel: 0 }
        );
        assert_eq!(
            song.set_track_midi_channel(99, 1)
                .expect_err("track out of range"),
            EditError::TrackOutOfBounds { track: 99 }
        );
    }

    #[test]
    fn tracks_can_be_renamed_with_validation() {
        let mut song = Song::empty();

        song.rename_track(1, " Acid Bass ").expect("rename track");

        assert_eq!(song.tracks[1].name, "Acid Bass");
        assert_eq!(
            song.rename_track(1, "   ").expect_err("empty name"),
            EditError::EmptyName
        );
        assert_eq!(
            song.rename_track(99, "Missing")
                .expect_err("track out of range"),
            EditError::TrackOutOfBounds { track: 99 }
        );
    }

    #[test]
    fn creating_pattern_uses_current_track_shape() {
        let mut song = Song::empty();
        song.create_track();

        let id = song.create_pattern(32);

        assert_eq!(id, PatternId(2));
        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.patterns[1].rows.len(), 32);
        assert_eq!(song.patterns[1].rows[0].cells.len(), song.tracks.len());
    }

    #[test]
    fn duplicating_pattern_copies_cells_with_new_identity() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        let id = song.duplicate_pattern(0).expect("duplicate pattern");

        assert_eq!(id, PatternId(2));
        assert_eq!(song.patterns[1].id, PatternId(2));
        assert_eq!(song.patterns[1].name, "Pattern 02");
        assert_eq!(
            song.patterns[1].cell(0, 0).expect("cell").note,
            Some(NoteEvent::Note { pitch: 60 })
        );
    }

    #[test]
    fn deleting_pattern_removes_sequence_references() {
        let mut song = Song::empty();
        let id = song.create_pattern(64);
        song.push_sequence_pattern(id).expect("push sequence");

        let removed = song.delete_pattern(1).expect("delete pattern");

        assert_eq!(removed.id, id);
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.sequence, vec![PatternId(1)]);
    }

    #[test]
    fn cannot_delete_last_pattern() {
        let mut song = Song::empty();

        let error = song.delete_pattern(0).expect_err("last pattern remains");

        assert_eq!(error, EditError::CannotDeleteLastPattern);
    }

    #[test]
    fn resizing_pattern_preserves_existing_rows_and_adds_track_shaped_rows() {
        let mut song = Song::empty();
        song.create_track();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        song.resize_pattern(0, 80).expect("resize pattern");

        let pattern = song.current_pattern().expect("pattern");
        assert_eq!(pattern.row_count(), 80);
        assert_eq!(
            pattern.cell(0, 0).expect("cell").note,
            Some(NoteEvent::Note { pitch: 60 })
        );
        assert_eq!(pattern.rows[79].cells.len(), song.tracks.len());
    }

    #[test]
    fn resizing_pattern_can_truncate_rows() {
        let mut song = Song::empty();

        song.resize_pattern(0, 16).expect("resize pattern");

        assert_eq!(song.current_pattern().expect("pattern").row_count(), 16);
        assert_eq!(
            song.resize_pattern(0, 0).expect_err("invalid length"),
            EditError::InvalidPatternLength { row_count: 0 }
        );
    }

    #[test]
    fn pattern_rows_can_be_inserted_and_deleted() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        song.insert_pattern_row(0, 0).expect("insert row");

        let pattern = song.current_pattern().expect("pattern");
        assert_eq!(pattern.row_count(), DEFAULT_PATTERN_LEN + 1);
        assert_eq!(pattern.rows[0].cells.len(), song.tracks.len());
        assert_eq!(pattern.cell(0, 0), Some(&PatternCell::default()));
        assert_eq!(
            pattern.cell(1, 0).expect("cell").note,
            Some(NoteEvent::Note { pitch: 60 })
        );

        let removed = song.delete_pattern_row(0, 0).expect("delete row");
        assert_eq!(removed.cells.len(), song.tracks.len());
        assert_eq!(
            song.current_pattern().expect("pattern").row_count(),
            DEFAULT_PATTERN_LEN
        );
    }

    #[test]
    fn deleting_rows_validates_bounds_and_keeps_one_row() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern 01", 1, 4);

        assert_eq!(
            pattern.delete_row(0).expect_err("last row remains"),
            EditError::CannotDeleteLastPatternRow
        );
        assert_eq!(
            pattern.insert_row(2, 4).expect_err("row out of range"),
            EditError::RowOutOfBounds { row: 2 }
        );
    }

    #[test]
    fn patterns_can_be_renamed_with_validation() {
        let mut song = Song::empty();

        song.rename_pattern(0, " Intro ").expect("rename pattern");

        assert_eq!(song.patterns[0].name, "Intro");
        assert_eq!(
            song.rename_pattern(0, "").expect_err("empty name"),
            EditError::EmptyName
        );
        assert_eq!(
            song.rename_pattern(99, "Missing")
                .expect_err("pattern out of range"),
            EditError::PatternOutOfBounds { pattern: 99 }
        );
    }

    #[test]
    fn sequence_positions_can_be_added_and_removed_without_deleting_patterns() {
        let mut song = Song::empty();
        let id = song.create_pattern(64);

        song.push_sequence_pattern(id).expect("push sequence");
        let removed = song.remove_sequence_position(0).expect("remove sequence");

        assert_eq!(removed, PatternId(1));
        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.sequence, vec![id]);
    }

    #[test]
    fn sequence_positions_can_be_duplicated_changed_and_moved() {
        let mut song = Song::empty();
        let pattern_2 = song.create_pattern(64);
        let pattern_3 = song.create_pattern(64);
        song.push_sequence_pattern(pattern_2)
            .expect("push pattern 2");
        song.push_sequence_pattern(pattern_3)
            .expect("push pattern 3");

        song.duplicate_sequence_position(1)
            .expect("duplicate sequence position");
        assert_eq!(
            song.sequence,
            vec![PatternId(1), pattern_2, pattern_2, pattern_3]
        );

        song.set_sequence_pattern(0, pattern_3)
            .expect("change sequence pattern");
        assert_eq!(song.sequence[0], pattern_3);

        song.move_sequence_position(3, 1)
            .expect("move sequence position");
        assert_eq!(
            song.sequence,
            vec![pattern_3, pattern_3, pattern_2, pattern_2]
        );
    }

    #[test]
    fn sequence_operations_validate_bounds_and_pattern_identity() {
        let mut song = Song::empty();

        assert_eq!(
            song.duplicate_sequence_position(10)
                .expect_err("position out of bounds"),
            EditError::SequenceOutOfBounds { position: 10 }
        );
        assert_eq!(
            song.set_sequence_pattern(0, PatternId(99))
                .expect_err("pattern missing"),
            EditError::PatternNotFound {
                pattern_id: PatternId(99)
            }
        );
        assert_eq!(
            song.move_sequence_position(0, 10)
                .expect_err("target out of bounds"),
            EditError::SequenceOutOfBounds { position: 10 }
        );
    }

    #[test]
    fn sample_references_can_be_assigned_replaced_and_removed() {
        let mut song = Song::empty();
        let drums = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let replacement = song.upsert_sample_reference("samples/snare.wav", "snare.wav");
        let track = song.tracks[0].id;

        song.assign_sample_to_track(track, drums)
            .expect("assign drums");
        song.assign_sample_to_track(track, replacement)
            .expect("replace assignment");

        assert_eq!(
            song.sample_assignment_for_track(track),
            Some(&TrackSampleAssignment {
                track,
                sample: replacement
            })
        );
        assert_eq!(
            song.sample_for_track(track).expect("sample").name,
            "snare.wav"
        );

        song.unassign_sample_from_track(track);

        assert!(song.sample_assignment_for_track(track).is_none());
    }

    #[test]
    fn sample_assignment_creates_track_instrument_assignment() {
        let mut song = Song::empty();
        let track = song.tracks[0].id;
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");

        song.assign_sample_to_track(track, sample)
            .expect("assign sample");

        let instrument = song.instrument_for_track(track).expect("instrument");
        assert_eq!(instrument.name, "kick.wav");
        assert_eq!(instrument.sample, Some(sample));
        assert_eq!(
            song.sample_for_track(track).expect("sample").path,
            "samples/kick.wav"
        );
    }

    #[test]
    fn sample_playback_settings_validate_and_edit() {
        let mut song = Song::empty();
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");

        song.set_sample_frame_window(sample, Some(10), Some(100))
            .expect("set frame window");
        song.set_sample_loop(sample, SamplePlaybackMode::Loop, Some(20), Some(80))
            .expect("set loop");
        song.set_sample_envelope(
            sample,
            SampleEnvelope {
                attack_seconds: 0.01,
                decay_seconds: 0.02,
                sustain: 0.75,
                release_seconds: 0.03,
            },
        )
        .expect("set envelope");

        let playback = song.sample_for_id(sample).expect("sample").playback;
        assert_eq!(playback.start_frame, Some(10));
        assert_eq!(playback.end_frame, Some(100));
        assert_eq!(playback.mode, SamplePlaybackMode::Loop);
        assert_eq!(playback.loop_start_frame, Some(20));
        assert_eq!(playback.loop_end_frame, Some(80));
        assert_eq!(playback.envelope.sustain, 0.75);
        song.validate().expect("valid playback settings");

        assert_eq!(
            song.set_sample_frame_window(sample, Some(100), Some(10))
                .expect_err("invalid frame window"),
            EditError::InvalidSampleFrameWindow
        );
        assert_eq!(
            song.set_sample_loop(sample, SamplePlaybackMode::Loop, Some(80), Some(20))
                .expect_err("invalid loop window"),
            EditError::InvalidSampleLoopWindow
        );
        assert_eq!(
            song.set_sample_envelope(
                sample,
                SampleEnvelope {
                    attack_seconds: 0.0,
                    decay_seconds: 0.0,
                    sustain: 1.5,
                    release_seconds: 0.0,
                },
            )
            .expect_err("invalid envelope"),
            EditError::InvalidSampleEnvelope
        );
    }

    #[test]
    fn validation_rejects_invalid_sample_playback_settings() {
        let mut song = Song::empty();
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");

        song.sample_for_id_mut(sample).expect("sample").playback = SamplePlaybackSettings {
            mode: SamplePlaybackMode::Loop,
            loop_start_frame: Some(12),
            loop_end_frame: None,
            ..SamplePlaybackSettings::default()
        };

        assert_eq!(
            song.validate().expect_err("partial loop is invalid"),
            ValidationError::InvalidSampleLoopWindow { sample_index: 0 }
        );
    }

    #[test]
    fn instrument_assignments_can_drive_sample_lookup_without_legacy_assignment() {
        let mut song = Song::empty();
        let track = song.tracks[0].id;
        let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let instrument = song.upsert_sample_instrument(sample).expect("instrument");
        song.assign_instrument_to_track(track, instrument)
            .expect("assign instrument");

        assert_eq!(
            song.sample_for_track(track).expect("sample").path,
            "samples/kick.wav"
        );
        assert!(song.sample_assignment_for_track(track).is_none());
    }

    #[test]
    fn sample_references_are_removed_only_when_unused() {
        let mut song = Song::empty();
        let assigned = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let unused = song.upsert_sample_reference("samples/snare.wav", "snare.wav");
        let track = song.tracks[0].id;

        song.assign_sample_to_track(track, assigned)
            .expect("assign sample");

        assert_eq!(
            song.remove_sample_reference(assigned)
                .expect_err("assigned sample is protected"),
            EditError::SampleInUse {
                sample_id: assigned
            }
        );

        song.remove_sample_reference(unused)
            .expect("remove unused sample");

        assert!(song.sample_for_id(assigned).is_some());
        assert!(song.sample_for_id(unused).is_none());
    }

    #[test]
    fn unused_sample_references_can_be_pruned() {
        let mut song = Song::empty();
        let assigned = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
        let first_unused = song.upsert_sample_reference("samples/snare.wav", "snare.wav");
        let second_unused = song.upsert_sample_reference("samples/hat.wav", "hat.wav");
        let track = song.tracks[0].id;

        song.assign_sample_to_track(track, assigned)
            .expect("assign sample");

        assert_eq!(song.prune_unused_sample_references(), 2);
        assert!(song.sample_for_id(assigned).is_some());
        assert!(song.sample_for_id(first_unused).is_none());
        assert!(song.sample_for_id(second_unused).is_none());
    }

    #[test]
    fn sample_assignments_follow_track_lifecycle() {
        let mut song = Song::empty();
        let sample = song.upsert_sample_reference("samples/bass.wav", "bass.wav");
        let source_track = song.tracks[0].id;
        song.assign_sample_to_track(source_track, sample)
            .expect("assign source");

        let duplicated_track = song.duplicate_track(0).expect("duplicate track");

        assert_eq!(
            song.sample_assignment_for_track(duplicated_track)
                .expect("duplicated assignment")
                .sample,
            sample
        );

        song.delete_track(0).expect("delete source");

        assert!(song.sample_assignment_for_track(source_track).is_none());
        assert!(song.sample_assignment_for_track(duplicated_track).is_some());
        assert!(song.instrument_assignment_for_track(source_track).is_none());
        assert!(song
            .instrument_assignment_for_track(duplicated_track)
            .is_some());
    }

    #[test]
    fn validation_rejects_invalid_sample_assignments() {
        let mut song = Song::empty();
        song.sample_assignments.push(TrackSampleAssignment {
            track: song.tracks[0].id,
            sample: SampleId(99),
        });

        assert_eq!(
            song.validate().expect_err("missing sample"),
            ValidationError::SampleAssignmentSampleNotFound {
                sample_id: SampleId(99)
            }
        );
    }

    #[test]
    fn validation_rejects_invalid_instrument_assignments() {
        let mut song = Song::empty();
        song.track_instrument_assignments
            .push(TrackInstrumentAssignment {
                track: song.tracks[0].id,
                instrument: InstrumentId(99),
            });

        assert_eq!(
            song.validate().expect_err("missing instrument"),
            ValidationError::InstrumentAssignmentInstrumentNotFound {
                instrument_id: InstrumentId(99)
            }
        );
    }

    #[test]
    fn default_song_validates() {
        Song::empty().validate().expect("default song is valid");
    }

    #[test]
    fn validation_rejects_missing_sequence_pattern() {
        let mut song = Song::empty();
        song.sequence[0] = PatternId(99);

        assert_eq!(
            song.validate().expect_err("missing sequence pattern"),
            ValidationError::SequencePatternNotFound {
                position: 0,
                pattern_id: PatternId(99),
            }
        );
    }

    #[test]
    fn validation_rejects_row_cell_count_mismatch() {
        let mut song = Song::empty();
        song.patterns[0].rows[0].cells.pop();

        assert_eq!(
            song.validate().expect_err("cell count mismatch"),
            ValidationError::PatternRowCellCountMismatch {
                pattern_index: 0,
                row_index: 0,
                expected: 4,
                actual: 3,
            }
        );
    }

    #[test]
    fn validation_rejects_invalid_cell_values() {
        let mut song = Song::empty();
        song.patterns[0].rows[0].cells[0].velocity = Some(0x80);

        assert_eq!(
            song.validate().expect_err("invalid velocity"),
            ValidationError::InvalidVelocity {
                pattern_index: 0,
                row_index: 0,
                track_index: 0,
                velocity: 0x80,
            }
        );
    }
}
