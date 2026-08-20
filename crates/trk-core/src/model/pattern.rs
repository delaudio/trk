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

    pub fn set_gate(
        &mut self,
        row: usize,
        track: usize,
        gate: Option<u8>,
    ) -> Result<(), EditError> {
        let remaining = self.row_count().saturating_sub(row).clamp(1, 0x7f) as u8;
        let cell = self
            .cell_mut(row, track)
            .ok_or(EditError::CellOutOfBounds { row, track })?;
        cell.gate = gate.map(|value| value.clamp(1, remaining));
        Ok(())
    }

    #[must_use]
    pub fn note_gate_rows(&self, row: usize, track: usize) -> Option<usize> {
        let cell = self.cell(row, track)?;
        if !matches!(cell.note, Some(NoteEvent::Note { .. })) {
            return None;
        }
        let remaining = self.row_count().saturating_sub(row).max(1);
        let next_event = self
            .rows
            .iter()
            .enumerate()
            .skip(row.saturating_add(1))
            .find(|(_, candidate)| {
                candidate
                    .cells
                    .get(track)
                    .is_some_and(|candidate| candidate.note.is_some())
            })
            .map_or(self.row_count(), |(next_row, _)| next_row);
        let until_next = next_event.saturating_sub(row).max(1);
        Some(
            cell.gate
                .map_or(until_next, |gate| usize::from(gate.max(1)))
                .min(remaining)
                .min(until_next),
        )
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
        let valid = match target {
            AutomationTarget::SampleGain { .. } => sample_gain_descriptor().validate_f32(value),
            AutomationTarget::MidiCc { controller, .. } => {
                controller <= 0x7f && value.is_finite() && (0.0..=1.0).contains(&value)
            }
        };
        if !valid {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command2: Option<TrackerCommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_locks: Vec<ParameterLock>,
}

impl PatternCell {
    pub fn commands(&self) -> impl Iterator<Item = TrackerCommand> {
        self.command.into_iter().chain(self.command2)
    }
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
            CellField::Gate => self.field = CellField::Delay,
            CellField::Effect => self.field = CellField::Gate,
            CellField::Effect2 => self.field = CellField::Effect,
            CellField::Note if self.track > 0 => {
                self.track -= 1;
                self.field = CellField::Effect2;
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
            CellField::Delay => self.field = CellField::Gate,
            CellField::Gate => self.field = CellField::Effect,
            CellField::Effect => self.field = CellField::Effect2,
            CellField::Effect2 if self.track + 1 < track_count => {
                self.track += 1;
                self.field = CellField::Note;
            }
            CellField::Effect2 => {}
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
    Gate,
    Effect,
    Effect2,
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
            CellField::Gate => f.write_str("GATE"),
            CellField::Effect => f.write_str("FX1"),
            CellField::Effect2 => f.write_str("FX2"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_cell_command2_is_backward_compatible() {
        let legacy = r#"{"command":{"code":68,"value":32}}"#;
        let mut cell: PatternCell = serde_json::from_str(legacy).expect("legacy cell");
        assert_eq!(cell.command, Some(TrackerCommand::delay(32)));
        assert_eq!(cell.command2, None);

        let serialized = serde_json::to_string(&cell).expect("serialize");
        assert!(!serialized.contains("command2"));

        cell.command2 = Some(TrackerCommand::retrigger(4));
        let serialized = serde_json::to_string(&cell).expect("serialize command2");
        assert!(serialized.contains("command2"));
    }

    #[test]
    fn resolved_note_gates_preserve_legacy_sustain_and_stop_at_replacements() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern", 16, 1);
        pattern
            .set_note(2, 0, NoteEvent::Note { pitch: 60 }, 100)
            .expect("first note");
        pattern
            .set_note(8, 0, NoteEvent::Note { pitch: 64 }, 100)
            .expect("replacement");
        assert_eq!(pattern.note_gate_rows(2, 0), Some(6));

        pattern.set_gate(2, 0, Some(12)).expect("long gate");
        assert_eq!(pattern.note_gate_rows(2, 0), Some(6));
        pattern.set_gate(2, 0, Some(3)).expect("short gate");
        assert_eq!(pattern.note_gate_rows(2, 0), Some(3));

        pattern
            .set_note(14, 0, NoteEvent::Note { pitch: 67 }, 100)
            .expect("ending note");
        pattern.set_gate(14, 0, Some(12)).expect("ending gate");
        assert_eq!(pattern.cell(14, 0).expect("cell").gate, Some(2));
    }
}
