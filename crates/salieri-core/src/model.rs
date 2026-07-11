use std::fmt;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub metadata: SongMetadata,
    pub transport: TransportSettings,
    pub tracks: Vec<Track>,
    pub patterns: Vec<Pattern>,
    pub sequence: Vec<PatternId>,
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
        }
    }

    #[must_use]
    pub fn current_pattern(&self) -> Option<&Pattern> {
        self.patterns.first()
    }

    pub fn current_pattern_mut(&mut self) -> Option<&mut Pattern> {
        self.patterns.first_mut()
    }
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
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        cell.note = Some(note);
        cell.velocity = Some(velocity.min(0x7f));
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    #[error("cell out of bounds: row {row}, track {track}")]
    CellOutOfBounds { row: usize, track: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternRow {
    pub cells: Vec<PatternCell>,
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
    fn editing_outside_pattern_returns_error() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern 01", 64, 4);

        let error = pattern
            .set_note(100, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect_err("out of bounds");

        assert_eq!(error, EditError::CellOutOfBounds { row: 100, track: 0 });
    }
}
