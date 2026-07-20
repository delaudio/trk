use std::fmt;

use serde::{Deserialize, Serialize};

use crate::sample_gain_descriptor;

use super::{
    AutomationInterpolation, AutomationLane, AutomationPoint, AutomationTarget, EditError,
    InstrumentId, ParameterLock, PatternId,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pattern {
    pub id: PatternId,
    pub name: String,
    pub rows: Vec<PatternRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automation: Vec<AutomationLane>,
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
            automation: Vec::new(),
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
        for lane in &mut self.automation {
            lane.points.retain(|point| point.row < row_count);
        }
        self.automation.retain(|lane| !lane.points.is_empty());
    }

    pub fn insert_row(&mut self, row: usize, track_count: usize) -> Result<(), EditError> {
        if row > self.rows.len() {
            return Err(EditError::RowOutOfBounds { row });
        }
        self.rows.insert(row, PatternRow::empty(track_count));
        for lane in &mut self.automation {
            for point in &mut lane.points {
                if point.row >= row {
                    point.row = point.row.saturating_add(1);
                }
            }
        }
        Ok(())
    }

    pub fn delete_row(&mut self, row: usize) -> Result<PatternRow, EditError> {
        if self.rows.len() <= 1 {
            return Err(EditError::CannotDeleteLastPatternRow);
        }
        if row >= self.rows.len() {
            return Err(EditError::RowOutOfBounds { row });
        }
        for lane in &mut self.automation {
            lane.points.retain(|point| point.row != row);
            for point in &mut lane.points {
                if point.row > row {
                    point.row = point.row.saturating_sub(1);
                }
            }
        }
        self.automation.retain(|lane| !lane.points.is_empty());
        Ok(self.rows.remove(row))
    }

    pub fn set_automation_point(
        &mut self,
        target: AutomationTarget,
        row: usize,
        value: f32,
    ) -> Result<(), EditError> {
        if row >= self.rows.len() {
            return Err(EditError::RowOutOfBounds { row });
        }
        if !sample_gain_descriptor().validate_f32(value) {
            return Err(EditError::InvalidAutomationValue);
        }

        let lane = if let Some(lane) = self
            .automation
            .iter_mut()
            .find(|lane| lane.target == target)
        {
            lane
        } else {
            self.automation.push(AutomationLane {
                target,
                interpolation: AutomationInterpolation::Step,
                points: Vec::new(),
            });
            self.automation
                .last_mut()
                .expect("automation lane was just inserted")
        };

        if let Some(point) = lane.points.iter_mut().find(|point| point.row == row) {
            point.value = value;
        } else {
            lane.points.push(AutomationPoint { row, value });
            lane.points.sort_by_key(|point| point.row);
        }
        Ok(())
    }

    pub fn clear_automation_point(
        &mut self,
        target: AutomationTarget,
        row: usize,
    ) -> Result<(), EditError> {
        if row >= self.rows.len() {
            return Err(EditError::RowOutOfBounds { row });
        }
        if let Some(lane) = self
            .automation
            .iter_mut()
            .find(|lane| lane.target == target)
        {
            lane.points.retain(|point| point.row != row);
        }
        self.automation.retain(|lane| !lane.points.is_empty());
        Ok(())
    }

    #[must_use]
    pub fn automation_value_at(
        &self,
        target: AutomationTarget,
        row: usize,
        default_value: f32,
    ) -> f32 {
        self.automation
            .iter()
            .find(|lane| lane.target == target)
            .and_then(|lane| {
                lane.points
                    .iter()
                    .take_while(|point| point.row <= row)
                    .last()
                    .map(|point| point.value)
            })
            .unwrap_or(default_value)
    }

    pub(super) fn append_track(&mut self) {
        for row in &mut self.rows {
            row.cells.push(PatternCell::default());
        }
    }

    pub(super) fn duplicate_track(&mut self, track: usize) -> Result<(), EditError> {
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

    pub(super) fn remove_track(&mut self, track: usize) -> Result<(), EditError> {
        for row in &mut self.rows {
            if track >= row.cells.len() {
                return Err(EditError::TrackOutOfBounds { track });
            }
            row.cells.remove(track);
        }
        Ok(())
    }

    pub(super) fn move_track(&mut self, from: usize, to: usize) -> Result<(), EditError> {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternCell {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<NoteEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument: Option<InstrumentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pan: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<TrackerCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_locks: Vec<ParameterLock>,
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
            CellField::Instrument => self.field = CellField::Velocity,
            CellField::Volume => self.field = CellField::Instrument,
            CellField::Pan => self.field = CellField::Volume,
            CellField::Delay => self.field = CellField::Pan,
            CellField::Effect => self.field = CellField::Delay,
            CellField::Note if self.track > 0 => {
                self.track -= 1;
                self.field = CellField::Effect;
            }
            CellField::Note => {}
        }
    }

    fn move_right(&mut self, track_count: usize) {
        match self.field {
            CellField::Note => self.field = CellField::Velocity,
            CellField::Velocity => self.field = CellField::Instrument,
            CellField::Instrument => self.field = CellField::Volume,
            CellField::Volume => self.field = CellField::Pan,
            CellField::Pan => self.field = CellField::Delay,
            CellField::Delay => self.field = CellField::Effect,
            CellField::Effect if self.track + 1 < track_count => {
                self.track += 1;
                self.field = CellField::Note;
            }
            CellField::Effect => {}
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
    Instrument,
    Volume,
    Pan,
    Delay,
    Effect,
}

impl fmt::Display for CellField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellField::Note => f.write_str("NOTE"),
            CellField::Velocity => f.write_str("VEL"),
            CellField::Instrument => f.write_str("INST"),
            CellField::Volume => f.write_str("VOL"),
            CellField::Pan => f.write_str("PAN"),
            CellField::Delay => f.write_str("DLY"),
            CellField::Effect => f.write_str("FX"),
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
