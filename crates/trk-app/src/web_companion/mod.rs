mod opener;
mod page;
mod server;

use std::sync::{
    mpsc::{self, Receiver},
    Arc, RwLock, TryLockError,
};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use trk_core::{NoteEvent, Pattern, PatternId};

use crate::App;

pub(crate) use opener::{open_browser, BrowserOpenMonitor};
use server::WebServer;

const STATE_VERSION: u8 = 1;
const ACTION_QUEUE_CAPACITY: usize = 32;
const PUBLISH_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebAction {
    TogglePlayback,
    Stop,
    SelectPattern { index: usize },
    ToggleTrackMute { index: usize },
    ToggleTrackSolo { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct WebActionRequest {
    #[serde(rename = "type")]
    kind: WebActionKind,
    index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum WebActionKind {
    TogglePlayback,
    Stop,
    SelectPattern,
    ToggleTrackMute,
    ToggleTrackSolo,
}

impl WebActionRequest {
    fn into_action(self) -> Option<WebAction> {
        match (self.kind, self.index) {
            (WebActionKind::TogglePlayback, None) => Some(WebAction::TogglePlayback),
            (WebActionKind::Stop, None) => Some(WebAction::Stop),
            (WebActionKind::SelectPattern, Some(index)) => Some(WebAction::SelectPattern { index }),
            (WebActionKind::ToggleTrackMute, Some(index)) => {
                Some(WebAction::ToggleTrackMute { index })
            }
            (WebActionKind::ToggleTrackSolo, Some(index)) => {
                Some(WebAction::ToggleTrackSolo { index })
            }
            (WebActionKind::TogglePlayback | WebActionKind::Stop, Some(_))
            | (
                WebActionKind::SelectPattern
                | WebActionKind::ToggleTrackMute
                | WebActionKind::ToggleTrackSolo,
                None,
            ) => None,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebPatternNote {
    row: usize,
    track: usize,
    pitch: u8,
    velocity: u8,
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
}

impl App {
    pub(crate) fn request_web_companion(&mut self) {
        self.web_companion_requested = true;
        self.notify_info("Opening web companion");
    }

    pub(crate) fn take_web_companion_request(&mut self) -> bool {
        std::mem::take(&mut self.web_companion_requested)
    }

    pub(crate) fn apply_web_action(&mut self, action: WebAction) {
        match action {
            WebAction::TogglePlayback => self.toggle_playback(),
            WebAction::Stop => self.stop_playback(),
            WebAction::SelectPattern { index } => self.select_pattern(index),
            WebAction::ToggleTrackMute { index } => self.toggle_track_mute(index),
            WebAction::ToggleTrackSolo { index } => self.toggle_track_solo(index),
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
                .map(|pattern| web_pattern_state(self.pattern_index, pattern)),
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

fn web_pattern_state(index: usize, pattern: &Pattern) -> WebPatternState {
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
                    }),
                    Some(NoteEvent::NoteOff | NoteEvent::NoteCut) | None => None,
                })
        })
        .collect();
    WebPatternState {
        index,
        id: pattern.id.0,
        name: pattern.name.clone(),
        rows: pattern.rows.len(),
        notes,
    }
}

fn active_note_at(pattern: &Pattern, row: usize, track: usize) -> Option<WebActiveNote> {
    pattern
        .rows
        .iter()
        .take(row.saturating_add(1))
        .rev()
        .filter_map(|row| row.cells.get(track))
        .find_map(|cell| match cell.note {
            Some(NoteEvent::Note { pitch }) => Some(Some(WebActiveNote {
                pitch,
                velocity: cell.velocity.unwrap_or(0x7f),
            })),
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
