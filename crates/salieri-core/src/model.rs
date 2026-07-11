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
pub struct ClipId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneId(pub u32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub metadata: SongMetadata,
    pub transport: TransportSettings,
    pub tracks: Vec<Track>,
    pub patterns: Vec<Pattern>,
    pub sequence: Vec<PatternId>,
    #[serde(default)]
    pub session: Session,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stem_manifest: Option<StemManifestReference>,
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
                stem: None,
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
            session: Session::default(),
            stem_manifest: None,
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
            if let Some(stem) = &track.stem {
                if stem.entry_id.trim().is_empty() {
                    return Err(ValidationError::EmptyStemEntryId { track_index });
                }
            }
        }

        if let Some(stem_manifest) = &self.stem_manifest {
            if stem_manifest.path.trim().is_empty() {
                return Err(ValidationError::EmptyStemManifestPath);
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

        let mut clip_ids = HashSet::new();
        for (clip_index, clip) in self.session.clips.iter().enumerate() {
            if !clip_ids.insert(clip.id) {
                return Err(ValidationError::DuplicateClipId { clip_id: clip.id });
            }
            if clip.name.trim().is_empty() {
                return Err(ValidationError::EmptyClipName { clip_index });
            }
            let ClipSource::Pattern {
                pattern_id,
                row_start,
                row_count,
            } = clip.source;
            let pattern = self
                .patterns
                .iter()
                .find(|pattern| pattern.id == pattern_id)
                .ok_or(ValidationError::ClipPatternNotFound {
                    clip_index,
                    pattern_id,
                })?;
            if row_count == 0 {
                return Err(ValidationError::InvalidClipRowRange {
                    clip_index,
                    row_start,
                    row_count,
                });
            }
            if row_start >= pattern.row_count()
                || row_start.saturating_add(row_count) > pattern.row_count()
            {
                return Err(ValidationError::InvalidClipRowRange {
                    clip_index,
                    row_start,
                    row_count,
                });
            }
        }

        let mut scene_ids = HashSet::new();
        for (scene_index, scene) in self.session.scenes.iter().enumerate() {
            if !scene_ids.insert(scene.id) {
                return Err(ValidationError::DuplicateSceneId { scene_id: scene.id });
            }
            if scene.name.trim().is_empty() {
                return Err(ValidationError::EmptySceneName { scene_index });
            }
            let mut scene_tracks = HashSet::new();
            for (slot_index, slot) in scene.slots.iter().enumerate() {
                if !track_ids.contains(&slot.track) {
                    return Err(ValidationError::SceneSlotTrackNotFound {
                        scene_index,
                        slot_index,
                        track_id: slot.track,
                    });
                }
                if !scene_tracks.insert(slot.track) {
                    return Err(ValidationError::DuplicateSceneTrackSlot {
                        scene_index,
                        track_id: slot.track,
                    });
                }
                if let Some(clip_id) = slot.clip {
                    if !clip_ids.contains(&clip_id) {
                        return Err(ValidationError::SceneSlotClipNotFound {
                            scene_index,
                            slot_index,
                            clip_id,
                        });
                    }
                }
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

        let pattern_id = self.patterns[pattern_index].id;
        if self
            .session
            .clips
            .iter()
            .any(|clip| clip.source.pattern_id() == pattern_id)
        {
            return Err(EditError::PatternInUseByClip { pattern_id });
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
            stem: None,
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
            stem: source.stem,
        });

        for pattern in &mut self.patterns {
            pattern.duplicate_track(track_index)?;
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
        for scene in &mut self.session.scenes {
            scene.slots.retain(|slot| slot.track != removed.id);
        }
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

    pub fn create_clip(
        &mut self,
        pattern_id: PatternId,
        name: impl Into<String>,
        row_start: usize,
        row_count: usize,
    ) -> Result<ClipId, EditError> {
        let name = clean_name(name.into())?;
        let pattern = self
            .patterns
            .iter()
            .find(|pattern| pattern.id == pattern_id)
            .ok_or(EditError::PatternNotFound { pattern_id })?;
        if row_count == 0
            || row_start >= pattern.row_count()
            || row_start.saturating_add(row_count) > pattern.row_count()
        {
            return Err(EditError::InvalidClipRowRange {
                row_start,
                row_count,
            });
        }

        let id = self.next_clip_id();
        self.session.clips.push(Clip {
            id,
            name,
            source: ClipSource::Pattern {
                pattern_id,
                row_start,
                row_count,
            },
            loop_enabled: true,
            launch_quantization: ClipLaunchQuantization::Pattern,
        });
        Ok(id)
    }

    pub fn delete_clip(&mut self, clip_id: ClipId) -> Result<Clip, EditError> {
        let index = self
            .session
            .clips
            .iter()
            .position(|clip| clip.id == clip_id)
            .ok_or(EditError::ClipNotFound { clip_id })?;
        for scene in &mut self.session.scenes {
            for slot in &mut scene.slots {
                if slot.clip == Some(clip_id) {
                    slot.clip = None;
                }
            }
        }
        Ok(self.session.clips.remove(index))
    }

    pub fn move_clip(&mut self, from: usize, to: usize) -> Result<(), EditError> {
        if from >= self.session.clips.len() {
            return Err(EditError::ClipOutOfBounds { clip: from });
        }
        if to >= self.session.clips.len() {
            return Err(EditError::ClipOutOfBounds { clip: to });
        }
        if from == to {
            return Ok(());
        }

        let clip = self.session.clips.remove(from);
        self.session.clips.insert(to, clip);
        Ok(())
    }

    pub fn create_scene(&mut self, name: impl Into<String>) -> Result<SceneId, EditError> {
        let name = clean_name(name.into())?;
        let id = self.next_scene_id();
        self.session.scenes.push(Scene {
            id,
            name,
            slots: Vec::new(),
        });
        Ok(id)
    }

    pub fn delete_scene(&mut self, scene_id: SceneId) -> Result<Scene, EditError> {
        let index = self
            .session
            .scenes
            .iter()
            .position(|scene| scene.id == scene_id)
            .ok_or(EditError::SceneNotFound { scene_id })?;
        Ok(self.session.scenes.remove(index))
    }

    pub fn move_scene(&mut self, from: usize, to: usize) -> Result<(), EditError> {
        if from >= self.session.scenes.len() {
            return Err(EditError::SceneOutOfBounds { scene: from });
        }
        if to >= self.session.scenes.len() {
            return Err(EditError::SceneOutOfBounds { scene: to });
        }
        if from == to {
            return Ok(());
        }

        let scene = self.session.scenes.remove(from);
        self.session.scenes.insert(to, scene);
        Ok(())
    }

    pub fn set_scene_clip(
        &mut self,
        scene_id: SceneId,
        track_id: TrackId,
        clip_id: Option<ClipId>,
    ) -> Result<(), EditError> {
        if !self.tracks.iter().any(|track| track.id == track_id) {
            return Err(EditError::TrackNotFound { track_id });
        }
        if let Some(clip_id) = clip_id {
            if !self.session.clips.iter().any(|clip| clip.id == clip_id) {
                return Err(EditError::ClipNotFound { clip_id });
            }
        }

        let scene = self
            .session
            .scenes
            .iter_mut()
            .find(|scene| scene.id == scene_id)
            .ok_or(EditError::SceneNotFound { scene_id })?;
        if let Some(slot) = scene.slots.iter_mut().find(|slot| slot.track == track_id) {
            slot.clip = clip_id;
        } else {
            scene.slots.push(ClipSlot {
                track: track_id,
                clip: clip_id,
            });
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

    fn next_clip_id(&self) -> ClipId {
        let next = self
            .session
            .clips
            .iter()
            .map(|clip| clip.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        ClipId(next)
    }

    fn next_scene_id(&self) -> SceneId {
        let next = self
            .session
            .scenes
            .iter()
            .map(|scene| scene.id.0)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        SceneId(next)
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
    #[error("stem manifest path cannot be empty")]
    EmptyStemManifestPath,
    #[error("track {track_index} stem entry id cannot be empty")]
    EmptyStemEntryId { track_index: usize },
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
    #[error("duplicate clip id: {clip_id:?}")]
    DuplicateClipId { clip_id: ClipId },
    #[error("clip {clip_index} name cannot be empty")]
    EmptyClipName { clip_index: usize },
    #[error("clip {clip_index} references missing pattern {pattern_id:?}")]
    ClipPatternNotFound {
        clip_index: usize,
        pattern_id: PatternId,
    },
    #[error("clip {clip_index} has invalid row range {row_start}..+{row_count}")]
    InvalidClipRowRange {
        clip_index: usize,
        row_start: usize,
        row_count: usize,
    },
    #[error("duplicate scene id: {scene_id:?}")]
    DuplicateSceneId { scene_id: SceneId },
    #[error("scene {scene_index} name cannot be empty")]
    EmptySceneName { scene_index: usize },
    #[error("scene {scene_index} slot {slot_index} references missing track {track_id:?}")]
    SceneSlotTrackNotFound {
        scene_index: usize,
        slot_index: usize,
        track_id: TrackId,
    },
    #[error("scene {scene_index} has duplicate slot for track {track_id:?}")]
    DuplicateSceneTrackSlot {
        scene_index: usize,
        track_id: TrackId,
    },
    #[error("scene {scene_index} slot {slot_index} references missing clip {clip_id:?}")]
    SceneSlotClipNotFound {
        scene_index: usize,
        slot_index: usize,
        clip_id: ClipId,
    },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stem: Option<StemTrackReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StemManifestReference {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StemTrackReference {
    pub entry_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    #[serde(default)]
    pub clips: Vec<Clip>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: ClipId,
    pub name: String,
    pub source: ClipSource,
    pub loop_enabled: bool,
    #[serde(default)]
    pub launch_quantization: ClipLaunchQuantization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClipSource {
    Pattern {
        pattern_id: PatternId,
        row_start: usize,
        row_count: usize,
    },
}

impl ClipSource {
    #[must_use]
    pub const fn pattern_id(self) -> PatternId {
        match self {
            Self::Pattern { pattern_id, .. } => pattern_id,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipLaunchQuantization {
    None,
    Row,
    Beat,
    #[default]
    Pattern,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: SceneId,
    pub name: String,
    #[serde(default)]
    pub slots: Vec<ClipSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSlot {
    pub track: TrackId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<ClipId>,
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
    #[error("cannot delete the last track")]
    CannotDeleteLastTrack,
    #[error("pattern out of bounds: pattern {pattern}")]
    PatternOutOfBounds { pattern: usize },
    #[error("pattern not found: pattern id {pattern_id:?}")]
    PatternNotFound { pattern_id: PatternId },
    #[error("pattern {pattern_id:?} is used by a clip")]
    PatternInUseByClip { pattern_id: PatternId },
    #[error("cannot delete the last pattern")]
    CannotDeleteLastPattern,
    #[error("clip not found: clip id {clip_id:?}")]
    ClipNotFound { clip_id: ClipId },
    #[error("clip out of bounds: clip {clip}")]
    ClipOutOfBounds { clip: usize },
    #[error("invalid clip row range {row_start}..+{row_count}")]
    InvalidClipRowRange { row_start: usize, row_count: usize },
    #[error("scene not found: scene id {scene_id:?}")]
    SceneNotFound { scene_id: SceneId },
    #[error("scene out of bounds: scene {scene}")]
    SceneOutOfBounds { scene: usize },
    #[error("track not found: track id {track_id:?}")]
    TrackNotFound { track_id: TrackId },
    #[error("sequence out of bounds: position {position}")]
    SequenceOutOfBounds { position: usize },
    #[error("invalid pattern length: {row_count}")]
    InvalidPatternLength { row_count: usize },
    #[error("cannot delete the last pattern row")]
    CannotDeleteLastPatternRow,
    #[error("invalid MIDI channel: {midi_channel}")]
    InvalidMidiChannel { midi_channel: u8 },
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
    fn clips_can_reference_pattern_ranges() {
        let mut song = Song::empty();

        let clip_id = song
            .create_clip(PatternId(1), " Intro Loop ", 0, 16)
            .expect("create clip");

        assert_eq!(clip_id, ClipId(1));
        assert_eq!(song.session.clips.len(), 1);
        assert_eq!(song.session.clips[0].name, "Intro Loop");
        assert_eq!(
            song.session.clips[0].source,
            ClipSource::Pattern {
                pattern_id: PatternId(1),
                row_start: 0,
                row_count: 16,
            }
        );
        assert!(song.session.clips[0].loop_enabled);
        assert_eq!(
            song.session.clips[0].launch_quantization,
            ClipLaunchQuantization::Pattern
        );
    }

    #[test]
    fn clip_operations_validate_pattern_references_and_ranges() {
        let mut song = Song::empty();

        assert_eq!(
            song.create_clip(PatternId(99), "Missing", 0, 16)
                .expect_err("missing pattern"),
            EditError::PatternNotFound {
                pattern_id: PatternId(99)
            }
        );
        assert_eq!(
            song.create_clip(PatternId(1), "Empty", 0, 0)
                .expect_err("empty row range"),
            EditError::InvalidClipRowRange {
                row_start: 0,
                row_count: 0,
            }
        );
        assert_eq!(
            song.create_clip(PatternId(1), "Too Long", 60, 8)
                .expect_err("range extends past pattern"),
            EditError::InvalidClipRowRange {
                row_start: 60,
                row_count: 8,
            }
        );
    }

    #[test]
    fn clips_and_scenes_can_be_deleted_and_moved() {
        let mut song = Song::empty();
        let clip_1 = song
            .create_clip(PatternId(1), "Clip 1", 0, 16)
            .expect("clip 1");
        let clip_2 = song
            .create_clip(PatternId(1), "Clip 2", 16, 16)
            .expect("clip 2");
        let scene_1 = song.create_scene("Scene 1").expect("scene 1");
        let scene_2 = song.create_scene("Scene 2").expect("scene 2");

        song.set_scene_clip(scene_1, TrackId(1), Some(clip_1))
            .expect("set scene slot");
        song.move_clip(0, 1).expect("move clip");
        song.move_scene(0, 1).expect("move scene");

        assert_eq!(song.session.clips[0].id, clip_2);
        assert_eq!(song.session.clips[1].id, clip_1);
        assert_eq!(song.session.scenes[0].id, scene_2);
        assert_eq!(song.session.scenes[1].id, scene_1);

        let removed = song.delete_clip(clip_1).expect("delete clip");
        assert_eq!(removed.id, clip_1);
        assert_eq!(song.session.scenes[1].slots[0].clip, None);

        let removed_scene = song.delete_scene(scene_2).expect("delete scene");
        assert_eq!(removed_scene.id, scene_2);
        assert_eq!(song.session.scenes.len(), 1);
    }

    #[test]
    fn scene_slots_validate_tracks_and_clips() {
        let mut song = Song::empty();
        let scene = song.create_scene("Scene").expect("scene");

        assert_eq!(
            song.set_scene_clip(scene, TrackId(99), None)
                .expect_err("missing track"),
            EditError::TrackNotFound {
                track_id: TrackId(99)
            }
        );
        assert_eq!(
            song.set_scene_clip(scene, TrackId(1), Some(ClipId(99)))
                .expect_err("missing clip"),
            EditError::ClipNotFound {
                clip_id: ClipId(99)
            }
        );
        assert_eq!(
            song.set_scene_clip(SceneId(99), TrackId(1), None)
                .expect_err("missing scene"),
            EditError::SceneNotFound {
                scene_id: SceneId(99)
            }
        );
    }

    #[test]
    fn deleting_track_removes_session_slots_for_that_track() {
        let mut song = Song::empty();
        let clip = song.create_clip(PatternId(1), "Clip", 0, 16).expect("clip");
        let scene = song.create_scene("Scene").expect("scene");
        song.set_scene_clip(scene, TrackId(2), Some(clip))
            .expect("set scene slot");

        let removed = song.delete_track(1).expect("delete bass track");

        assert_eq!(removed.id, TrackId(2));
        assert!(song.session.scenes[0].slots.is_empty());
    }

    #[test]
    fn deleting_pattern_referenced_by_clip_is_rejected() {
        let mut song = Song::empty();
        let pattern_id = song.create_pattern(16);
        song.create_clip(pattern_id, "Clip", 0, 16).expect("clip");

        assert_eq!(
            song.delete_pattern(1).expect_err("pattern in use"),
            EditError::PatternInUseByClip { pattern_id }
        );
    }

    #[test]
    fn validation_rejects_invalid_session_references() {
        let mut song = Song::empty();
        song.session.clips.push(Clip {
            id: ClipId(1),
            name: "Dangling".to_string(),
            source: ClipSource::Pattern {
                pattern_id: PatternId(99),
                row_start: 0,
                row_count: 16,
            },
            loop_enabled: true,
            launch_quantization: ClipLaunchQuantization::Pattern,
        });

        assert_eq!(
            song.validate().expect_err("missing clip pattern"),
            ValidationError::ClipPatternNotFound {
                clip_index: 0,
                pattern_id: PatternId(99),
            }
        );

        let mut song = Song::empty();
        song.session.scenes.push(Scene {
            id: SceneId(1),
            name: "Scene".to_string(),
            slots: vec![ClipSlot {
                track: TrackId(1),
                clip: Some(ClipId(99)),
            }],
        });

        assert_eq!(
            song.validate().expect_err("missing scene clip"),
            ValidationError::SceneSlotClipNotFound {
                scene_index: 0,
                slot_index: 0,
                clip_id: ClipId(99),
            }
        );
    }

    #[test]
    fn song_deserialization_defaults_missing_session() {
        let json = r#"{
            "metadata": { "title": "Legacy" },
            "transport": { "bpm": 120, "linesPerBeat": 4, "swing": 0.0 },
            "tracks": [
                { "id": 1, "name": "Drums", "midiChannel": 10, "muted": false, "solo": false, "armed": true }
            ],
            "patterns": [
                {
                    "id": 1,
                    "name": "Pattern 01",
                    "rows": [
                        { "cells": [ {} ] }
                    ]
                }
            ],
            "sequence": [1]
        }"#;

        let song: Song = serde_json::from_str(json).expect("deserialize legacy song");

        assert!(song.session.clips.is_empty());
        assert!(song.session.scenes.is_empty());
        assert!(song.stem_manifest.is_none());
        assert!(song.tracks[0].stem.is_none());
        song.validate().expect("legacy song validates");
    }

    #[test]
    fn stem_references_are_optional_but_not_empty() {
        let mut song = Song::empty();
        song.stem_manifest = Some(StemManifestReference {
            path: "stems.json".to_string(),
        });
        song.tracks[0].stem = Some(StemTrackReference {
            entry_id: "stem_000_kick".to_string(),
        });

        song.validate()
            .expect("stem manifest references do not require files at load time");

        song.tracks[0].stem = Some(StemTrackReference {
            entry_id: " ".to_string(),
        });

        assert_eq!(
            song.validate().expect_err("empty stem entry id"),
            ValidationError::EmptyStemEntryId { track_index: 0 }
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
