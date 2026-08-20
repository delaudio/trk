mod opener;
mod page;
mod server;

use std::sync::{
    mpsc::{self, Receiver},
    Arc, RwLock, TryLockError,
};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use trk_core::{AutomationTarget, NoteEvent, Pattern, PatternId};

use crate::{history::TransactionSpec, App, DEFAULT_NOTE_VELOCITY};

pub(crate) use opener::{open_browser, BrowserOpenMonitor};
use server::WebServer;

const STATE_VERSION: u8 = 2;
const ACTION_QUEUE_CAPACITY: usize = 32;
const PUBLISH_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebAction {
    TogglePlayback {
        revision: u64,
    },
    Stop {
        revision: u64,
    },
    SelectPattern {
        revision: u64,
        index: usize,
    },
    SelectTrack {
        revision: u64,
        index: usize,
    },
    ToggleTrackMute {
        revision: u64,
        index: usize,
    },
    ToggleTrackSolo {
        revision: u64,
        index: usize,
    },
    CreateNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        pitch: u8,
    },
    MoveNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        to_row: usize,
        pitch: u8,
    },
    ResizeNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        gate: u8,
    },
    DeleteNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
    },
    SetNoteVelocity {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        velocity: u8,
    },
    SetCcPoint {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        controller: u8,
        value: u8,
    },
    ClearCcPoint {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        controller: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum WebActionRequest {
    TogglePlayback {
        revision: u64,
    },
    Stop {
        revision: u64,
    },
    SelectPattern {
        revision: u64,
        index: usize,
    },
    SelectTrack {
        revision: u64,
        index: usize,
    },
    ToggleTrackMute {
        revision: u64,
        index: usize,
    },
    ToggleTrackSolo {
        revision: u64,
        index: usize,
    },
    CreateNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        pitch: u8,
    },
    MoveNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        to_row: usize,
        pitch: u8,
    },
    ResizeNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        gate: u8,
    },
    DeleteNote {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
    },
    SetNoteVelocity {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        velocity: u8,
    },
    SetCcPoint {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        controller: u8,
        value: u8,
    },
    ClearCcPoint {
        revision: u64,
        pattern: usize,
        row: usize,
        track: usize,
        controller: u8,
    },
}

impl WebActionRequest {
    fn into_action(self) -> WebAction {
        match self {
            Self::TogglePlayback { revision } => WebAction::TogglePlayback { revision },
            Self::Stop { revision } => WebAction::Stop { revision },
            Self::SelectPattern { revision, index } => WebAction::SelectPattern { revision, index },
            Self::SelectTrack { revision, index } => WebAction::SelectTrack { revision, index },
            Self::ToggleTrackMute { revision, index } => {
                WebAction::ToggleTrackMute { revision, index }
            }
            Self::ToggleTrackSolo { revision, index } => {
                WebAction::ToggleTrackSolo { revision, index }
            }
            Self::CreateNote {
                revision,
                pattern,
                row,
                track,
                pitch,
            } => WebAction::CreateNote {
                revision,
                pattern,
                row,
                track,
                pitch,
            },
            Self::MoveNote {
                revision,
                pattern,
                row,
                track,
                to_row,
                pitch,
            } => WebAction::MoveNote {
                revision,
                pattern,
                row,
                track,
                to_row,
                pitch,
            },
            Self::ResizeNote {
                revision,
                pattern,
                row,
                track,
                gate,
            } => WebAction::ResizeNote {
                revision,
                pattern,
                row,
                track,
                gate,
            },
            Self::DeleteNote {
                revision,
                pattern,
                row,
                track,
            } => WebAction::DeleteNote {
                revision,
                pattern,
                row,
                track,
            },
            Self::SetNoteVelocity {
                revision,
                pattern,
                row,
                track,
                velocity,
            } => WebAction::SetNoteVelocity {
                revision,
                pattern,
                row,
                track,
                velocity,
            },
            Self::SetCcPoint {
                revision,
                pattern,
                row,
                track,
                controller,
                value,
            } => WebAction::SetCcPoint {
                revision,
                pattern,
                row,
                track,
                controller,
                value,
            },
            Self::ClearCcPoint {
                revision,
                pattern,
                row,
                track,
                controller,
            } => WebAction::ClearCcPoint {
                revision,
                pattern,
                row,
                track,
                controller,
            },
        }
    }
}

impl WebAction {
    const fn revision(self) -> u64 {
        match self {
            Self::TogglePlayback { revision }
            | Self::Stop { revision }
            | Self::SelectPattern { revision, .. }
            | Self::SelectTrack { revision, .. }
            | Self::ToggleTrackMute { revision, .. }
            | Self::ToggleTrackSolo { revision, .. }
            | Self::CreateNote { revision, .. }
            | Self::MoveNote { revision, .. }
            | Self::ResizeNote { revision, .. }
            | Self::DeleteNote { revision, .. }
            | Self::SetNoteVelocity { revision, .. }
            | Self::SetCcPoint { revision, .. } => revision,
            Self::ClearCcPoint { revision, .. } => revision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebBridgeState {
    version: u8,
    revision: u64,
    song_title: String,
    transport: WebTransportState,
    tracks: Vec<WebTrackState>,
    patterns: Vec<WebPatternSummary>,
    sequence: Vec<WebSequenceSlot>,
    active_pattern: Option<WebPatternState>,
    selected_track: usize,
    meters: WebMasterMeters,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebTransportState {
    playing: bool,
    bpm: u16,
    lines_per_beat: u8,
    loop_pattern: bool,
    sequence_position: Option<usize>,
    pattern_index: usize,
    current_row: usize,
    current_tick: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebTrackState {
    index: usize,
    id: u32,
    name: String,
    midi_channel: u8,
    muted: bool,
    solo: bool,
    armed: bool,
    activity: f32,
    active_note: Option<WebActiveNote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebActiveNote {
    pitch: u8,
    velocity: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebPatternSummary {
    index: usize,
    id: u32,
    name: String,
    rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebSequenceSlot {
    index: usize,
    pattern_index: Option<usize>,
    pattern_id: u32,
    name: String,
    rows: usize,
    active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebPatternState {
    index: usize,
    id: u32,
    name: String,
    rows: usize,
    notes: Vec<WebPatternNote>,
    cc_lanes: Vec<WebCcLane>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebPatternNote {
    row: usize,
    track: usize,
    pitch: u8,
    velocity: u8,
    gate: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebCcLane {
    track: usize,
    controller: u8,
    points: Vec<WebCcPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct WebCcPoint {
    row: usize,
    value: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
struct WebMasterMeters {
    low: f32,
    mid: f32,
    high: f32,
    rms: f32,
    peak: f32,
}

pub(crate) struct WebCompanion {
    server: WebServer,
    state: Arc<RwLock<WebBridgeState>>,
    action_rx: Receiver<WebAction>,
    revision: u64,
    last_publish: Instant,
}

impl WebCompanion {
    pub(crate) fn start(mut initial_state: WebBridgeState) -> std::io::Result<Self> {
        initial_state.revision = 1;
        let state = Arc::new(RwLock::new(initial_state));
        let (action_tx, action_rx) = mpsc::sync_channel(ACTION_QUEUE_CAPACITY);
        let server = WebServer::start(Arc::clone(&state), action_tx)?;
        Ok(Self {
            server,
            state,
            action_rx,
            revision: 1,
            last_publish: Instant::now(),
        })
    }

    pub(crate) fn url(&self) -> &str {
        self.server.url()
    }

    pub(crate) fn publish_if_due(&mut self, build_state: impl FnOnce() -> WebBridgeState) -> bool {
        if self.last_publish.elapsed() < PUBLISH_INTERVAL {
            return false;
        }
        let mut state = build_state();
        self.revision = self.revision.wrapping_add(1).max(1);
        state.revision = self.revision;
        match self.state.try_write() {
            Ok(mut current) => {
                *current = state;
                self.last_publish = Instant::now();
                true
            }
            Err(TryLockError::Poisoned(error)) => {
                *error.into_inner() = state;
                self.last_publish = Instant::now();
                true
            }
            Err(TryLockError::WouldBlock) => false,
        }
    }

    pub(crate) fn try_recv_action(&self) -> Option<WebAction> {
        self.action_rx.try_recv().ok()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn action_matches_current_state(
        &self,
        action: WebAction,
        current: &WebBridgeState,
    ) -> bool {
        if action.revision() != self.revision {
            return false;
        }
        match self.state.read() {
            Ok(published) => same_action_surface(&published, current),
            Err(error) => same_action_surface(&error.into_inner(), current),
        }
    }
}

fn same_action_surface(published: &WebBridgeState, current: &WebBridgeState) -> bool {
    published.song_title == current.song_title
        && published.transport.playing == current.transport.playing
        && published.transport.bpm == current.transport.bpm
        && published.transport.lines_per_beat == current.transport.lines_per_beat
        && published.transport.loop_pattern == current.transport.loop_pattern
        && published.transport.pattern_index == current.transport.pattern_index
        && published.patterns == current.patterns
        && published.active_pattern == current.active_pattern
        && published.selected_track == current.selected_track
        && published.tracks.len() == current.tracks.len()
        && published
            .tracks
            .iter()
            .zip(&current.tracks)
            .all(|(left, right)| {
                left.index == right.index
                    && left.id == right.id
                    && left.name == right.name
                    && left.midi_channel == right.midi_channel
                    && left.muted == right.muted
                    && left.solo == right.solo
                    && left.armed == right.armed
            })
        && published.sequence.len() == current.sequence.len()
        && published
            .sequence
            .iter()
            .zip(&current.sequence)
            .all(|(left, right)| {
                left.index == right.index
                    && left.pattern_index == right.pattern_index
                    && left.pattern_id == right.pattern_id
                    && left.name == right.name
                    && left.rows == right.rows
            })
}

impl App {
    pub(crate) fn request_web_companion(&mut self) {
        self.web_companion_requested = true;
        self.notify_info("Opening web companion");
    }

    pub(crate) fn take_web_companion_request(&mut self) -> bool {
        std::mem::take(&mut self.web_companion_requested)
    }

    pub(crate) fn apply_web_action(&mut self, action: WebAction, expected_revision: u64) {
        if action.revision() != expected_revision {
            return;
        }
        match action {
            WebAction::TogglePlayback { .. } => self.toggle_playback(),
            WebAction::Stop { .. } => self.stop_playback(),
            WebAction::SelectPattern { index, .. } => self.select_pattern(index),
            WebAction::SelectTrack { index, .. } if index < self.song.tracks.len() => {
                self.cursor.track = index;
            }
            WebAction::ToggleTrackMute { index, .. } => self.toggle_track_mute(index),
            WebAction::ToggleTrackSolo { index, .. } => self.toggle_track_solo(index),
            WebAction::CreateNote {
                pattern,
                row,
                track,
                pitch,
                ..
            } => {
                self.mutate_song_with(
                    TransactionSpec::new("Create Web Piano Roll note"),
                    move |song, _| {
                        let Some(pattern) = song.pattern_mut(pattern) else {
                            return;
                        };
                        if row >= pattern.row_count()
                            || track >= pattern.rows.first().map_or(0, |row| row.cells.len())
                        {
                            return;
                        }
                        let _ = pattern.set_note(
                            row,
                            track,
                            NoteEvent::Note { pitch },
                            DEFAULT_NOTE_VELOCITY,
                        );
                        let _ = pattern.set_gate(row, track, Some(1));
                    },
                );
            }
            WebAction::MoveNote {
                pattern,
                row,
                track,
                to_row,
                pitch,
                ..
            } => {
                self.mutate_song_with(
                    TransactionSpec::new("Move Web Piano Roll note"),
                    move |song, _| {
                        let Some(pattern) = song.pattern_mut(pattern) else {
                            return;
                        };
                        if row >= pattern.row_count()
                            || to_row >= pattern.row_count()
                            || track >= pattern.rows.first().map_or(0, |row| row.cells.len())
                        {
                            return;
                        }
                        let Some(mut source) = pattern.cell(row, track).cloned() else {
                            return;
                        };
                        if !matches!(source.note, Some(NoteEvent::Note { .. })) {
                            return;
                        }
                        if to_row != row
                            && pattern
                                .cell(to_row, track)
                                .is_some_and(|cell| *cell != trk_core::PatternCell::default())
                        {
                            return;
                        }
                        source.note = Some(NoteEvent::Note { pitch });
                        if to_row != row {
                            let _ = pattern.clear_cell(row, track);
                        }
                        if let Some(destination) = pattern.cell_mut(to_row, track) {
                            *destination = source;
                        }
                    },
                );
            }
            WebAction::ResizeNote {
                pattern,
                row,
                track,
                gate,
                ..
            } => {
                self.mutate_song_with(
                    TransactionSpec::new("Resize Web Piano Roll note"),
                    move |song, _| {
                        let Some(pattern) = song.pattern_mut(pattern) else {
                            return;
                        };
                        if pattern
                            .cell(row, track)
                            .is_some_and(|cell| matches!(cell.note, Some(NoteEvent::Note { .. })))
                        {
                            let _ = pattern.set_gate(row, track, Some(gate));
                        }
                    },
                );
            }
            WebAction::DeleteNote {
                pattern,
                row,
                track,
                ..
            } => {
                self.mutate_song_with(
                    TransactionSpec::new("Delete Web Piano Roll note"),
                    move |song, _| {
                        if let Some(pattern) = song.pattern_mut(pattern) {
                            let _ = pattern.clear_cell(row, track);
                        }
                    },
                );
            }
            WebAction::SetNoteVelocity {
                pattern,
                row,
                track,
                velocity,
                ..
            } => {
                self.mutate_song_with(
                    TransactionSpec::new("Set Web Piano Roll velocity"),
                    move |song, _| {
                        if let Some(cell) = song
                            .pattern_mut(pattern)
                            .and_then(|pattern| pattern.cell_mut(row, track))
                        {
                            if matches!(cell.note, Some(NoteEvent::Note { .. })) {
                                cell.velocity = Some(velocity.min(127));
                            }
                        }
                    },
                );
            }
            WebAction::SetCcPoint {
                pattern,
                row,
                track,
                controller,
                value,
                ..
            } => {
                let Some(track_id) = self.song.tracks.get(track).map(|track| track.id) else {
                    return;
                };
                self.mutate_song_with(
                    TransactionSpec::new("Set Web MIDI CC point"),
                    move |song, _| {
                        if let Some(pattern) = song.pattern_mut(pattern) {
                            let _ = pattern.set_automation_point(
                                AutomationTarget::MidiCc {
                                    track: track_id,
                                    controller,
                                },
                                row,
                                f32::from(value) / 127.0,
                            );
                        }
                    },
                );
            }
            WebAction::ClearCcPoint {
                pattern,
                row,
                track,
                controller,
                ..
            } => {
                let Some(track_id) = self.song.tracks.get(track).map(|track| track.id) else {
                    return;
                };
                self.mutate_song_with(
                    TransactionSpec::new("Clear Web MIDI CC point"),
                    move |song, _| {
                        if let Some(pattern) = song.pattern_mut(pattern) {
                            let _ = pattern.clear_automation_point(
                                AutomationTarget::MidiCc {
                                    track: track_id,
                                    controller,
                                },
                                row,
                            );
                        }
                    },
                );
            }
            WebAction::SelectTrack { .. } => {}
        }
    }

    pub(crate) fn web_bridge_state(&self) -> WebBridgeState {
        let current_row = self.playhead_row.unwrap_or(self.cursor.row);
        let active_pattern = self.song.patterns.get(self.pattern_index);
        let current_tick = current_sequence_tick(
            &self.song.sequence,
            &self.song.patterns,
            self.sequence_position,
            current_row,
        );
        let row_events = active_pattern
            .and_then(|pattern| pattern.rows.get(current_row))
            .map(|row| row.cells.as_slice())
            .unwrap_or_default();
        let tracks = self
            .song
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let active_note =
                    active_pattern.and_then(|pattern| active_note_at(pattern, current_row, index));
                let activity = row_events
                    .get(index)
                    .and_then(|cell| match cell.note {
                        Some(NoteEvent::Note { .. }) => Some(cell.velocity.unwrap_or(0x7f)),
                        Some(NoteEvent::NoteOff | NoteEvent::NoteCut) | None => None,
                    })
                    .map_or(0.0, |velocity| f32::from(velocity) / 127.0);
                WebTrackState {
                    index,
                    id: track.id.0,
                    name: track.name.clone(),
                    midi_channel: track.midi_channel,
                    muted: track.muted,
                    solo: track.solo,
                    armed: track.armed,
                    activity,
                    active_note,
                }
            })
            .collect();
        let patterns = self
            .song
            .patterns
            .iter()
            .enumerate()
            .map(|(index, pattern)| WebPatternSummary {
                index,
                id: pattern.id.0,
                name: pattern.name.clone(),
                rows: pattern.rows.len(),
            })
            .collect();
        let sequence = self
            .song
            .sequence
            .iter()
            .enumerate()
            .map(|(index, pattern_id)| {
                let pattern_index = pattern_index(&self.song.patterns, *pattern_id);
                let pattern = pattern_index.and_then(|index| self.song.patterns.get(index));
                WebSequenceSlot {
                    index,
                    pattern_index,
                    pattern_id: pattern_id.0,
                    name: pattern.map_or_else(
                        || format!("Missing pattern {}", pattern_id.0),
                        |pattern| pattern.name.clone(),
                    ),
                    rows: pattern.map_or(0, |pattern| pattern.rows.len()),
                    active: self.sequence_position == Some(index),
                }
            })
            .collect();
        let meters = self.playback.calibration_meters();
        WebBridgeState {
            version: STATE_VERSION,
            revision: 0,
            song_title: self.song.metadata.title.clone(),
            transport: WebTransportState {
                playing: self.is_playing,
                bpm: self.song.transport.bpm,
                lines_per_beat: self.song.transport.lines_per_beat,
                loop_pattern: self.loop_pattern,
                sequence_position: self.sequence_position,
                pattern_index: self.pattern_index,
                current_row,
                current_tick,
            },
            tracks,
            patterns,
            sequence,
            active_pattern: active_pattern
                .map(|pattern| web_pattern_state(self.pattern_index, pattern, &self.song)),
            selected_track: self.cursor.track,
            meters: WebMasterMeters {
                low: finite_meter(meters.low),
                mid: finite_meter(meters.mid),
                high: finite_meter(meters.high),
                rms: finite_meter(meters.rms),
                peak: finite_meter(meters.peak),
            },
        }
    }
}

fn pattern_index(patterns: &[Pattern], id: PatternId) -> Option<usize> {
    patterns.iter().position(|pattern| pattern.id == id)
}

fn current_sequence_tick(
    sequence: &[PatternId],
    patterns: &[Pattern],
    sequence_position: Option<usize>,
    current_row: usize,
) -> usize {
    let preceding_rows = sequence_position.map_or(0, |position| {
        sequence
            .iter()
            .take(position)
            .filter_map(|id| pattern_index(patterns, *id))
            .filter_map(|index| patterns.get(index))
            .map(|pattern| pattern.rows.len())
            .fold(0_usize, usize::saturating_add)
    });
    preceding_rows.saturating_add(current_row)
}

fn web_pattern_state(index: usize, pattern: &Pattern, song: &trk_core::Song) -> WebPatternState {
    let notes = pattern
        .rows
        .iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            row.cells
                .iter()
                .enumerate()
                .filter_map(move |(track, cell)| match cell.note {
                    Some(NoteEvent::Note { pitch }) => Some(WebPatternNote {
                        row: row_index,
                        track,
                        pitch,
                        velocity: cell.velocity.unwrap_or(0x7f),
                        gate: pattern.note_gate_rows(row_index, track).unwrap_or(1),
                    }),
                    Some(NoteEvent::NoteOff | NoteEvent::NoteCut) | None => None,
                })
        })
        .collect();
    let cc_lanes = pattern
        .automation
        .iter()
        .filter_map(|lane| {
            let AutomationTarget::MidiCc { track, controller } = lane.target else {
                return None;
            };
            Some(WebCcLane {
                track: song
                    .tracks
                    .iter()
                    .position(|candidate| candidate.id == track)?,
                controller,
                points: lane
                    .points
                    .iter()
                    .map(|point| WebCcPoint {
                        row: point.row,
                        value: (point.value.clamp(0.0, 1.0) * 127.0).round() as u8,
                    })
                    .collect(),
            })
        })
        .collect();
    WebPatternState {
        index,
        id: pattern.id.0,
        name: pattern.name.clone(),
        rows: pattern.rows.len(),
        notes,
        cc_lanes,
    }
}

fn active_note_at(pattern: &Pattern, row: usize, track: usize) -> Option<WebActiveNote> {
    pattern
        .rows
        .iter()
        .enumerate()
        .take(row.saturating_add(1))
        .rev()
        .filter_map(|(start, row_cells)| row_cells.cells.get(track).map(|cell| (start, cell)))
        .find_map(|(start, cell)| match cell.note {
            Some(NoteEvent::Note { pitch }) => Some(
                (row < start.saturating_add(pattern.note_gate_rows(start, track).unwrap_or(1)))
                    .then_some(WebActiveNote {
                        pitch,
                        velocity: cell.velocity.unwrap_or(0x7f),
                    }),
            ),
            Some(NoteEvent::NoteOff | NoteEvent::NoteCut) => Some(None),
            None => None,
        })
        .flatten()
}

fn finite_meter(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests;
