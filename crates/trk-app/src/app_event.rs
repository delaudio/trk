use std::{collections::VecDeque, path::PathBuf};

use crossterm::event::KeyEvent;
use trk_ai::{AiProposal, CellAddress};
use trk_core::{Direction, NoteEvent, Song};
use trk_midi::MidiInputPacket;

use crate::{command::TrkCommand, playback_runtime::PlaybackUpdate, task_runtime::TaskUpdate};

#[derive(Debug)]
pub enum AppEvent {
    Intent(AppIntent),
    Runtime(RuntimeEvent),
}

#[derive(Debug)]
pub enum RuntimeEvent {
    PlaybackUpdate(PlaybackUpdate),
    MidiInput(MidiInputPacket),
    MidiInputFailed(String),
    SampleBrowserFinished(Result<Option<PathBuf>, String>),
    ProjectLoaded {
        request_id: RequestId,
        path: PathBuf,
        result: Box<Result<Song, String>>,
    },
    MidiImported {
        path: PathBuf,
        result: Box<Result<Song, String>>,
    },
    ProjectSaved {
        path: PathBuf,
        song: Box<Song>,
        quit_after: bool,
        result: Result<(), String>,
    },
    TaskUpdate(TaskUpdate<AppTaskResult>),
    Notification(NotificationRequest),
    ViewportRefresh {
        visible_rows: usize,
        visible_tracks: usize,
    },
}

#[derive(Debug)]
pub enum AppAction {
    ApplyIntent(AppIntent),
    ApplyRuntime(RuntimeAction),
}

#[derive(Debug)]
pub enum RuntimeAction {
    ApplyPlaybackUpdate(PlaybackUpdate),
    HandleMidiInput(MidiInputPacket),
    ReportMidiInputFailure(String),
    ApplySampleBrowserResult(Result<Option<PathBuf>, String>),
    ApplyProjectLoad {
        request_id: RequestId,
        path: PathBuf,
        result: Box<Result<Song, String>>,
    },
    ApplyMidiImport {
        path: PathBuf,
        result: Box<Result<Song, String>>,
    },
    ApplyProjectSave {
        path: PathBuf,
        song: Box<Song>,
        quit_after: bool,
        result: Result<(), String>,
    },
    ApplyTaskUpdate(TaskUpdate<AppTaskResult>),
    ShowNotification(NotificationRequest),
    KeepActiveViewportVisible {
        visible_rows: usize,
        visible_tracks: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppIntent {
    KeyInput(KeyEvent),
    Command(TrkCommand),
    Tracker(TrackerIntent),
    Navigation(NavigationIntent),
    Transport(TransportIntent),
    Parameter(ParameterIntent),
    Ai(AiIntent),
    OpenProject(PathBuf),
    SaveProject {
        path: Option<PathBuf>,
        quit_after: bool,
    },
    ImportMidi(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerIntent {
    InsertNote(u8),
    InsertNoteEvent(NoteEvent),
    EnterHexDigit(u8),
    ClearCell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationIntent {
    MoveCursor(Direction),
    PageUp,
    PageDown,
    NextTrack,
    PreviousTrack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportIntent {
    TogglePlayback,
    StartPattern,
    StartPatternFromCursor,
    StartSequence { position: usize },
    StartSelectedSequence,
    Stop,
    ToggleLoop,
    ConnectMidi { port_index: usize },
    DisconnectMidi,
    PanicMidi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterIntent {
    SetBpm(u16),
    AdjustBpm(i16),
    SetLinesPerBeat(u8),
    AdjustLinesPerBeat(i8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiIntent {
    Propose(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedAiProposal {
    pub proposal: AiProposal,
    pub touched_cells: Vec<CellAddress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppTaskResult {
    AiProposal(PreparedAiProposal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRequest {
    pub level: NotificationLevel,
    pub message: String,
}

impl NotificationRequest {
    pub fn new(level: NotificationLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Default)]
pub struct AppDispatcher {
    queue: VecDeque<AppEvent>,
    dispatching: bool,
}

impl AppDispatcher {
    pub fn enqueue(&mut self, event: AppEvent) -> bool {
        self.queue.push_back(event);
        if self.dispatching {
            false
        } else {
            self.dispatching = true;
            true
        }
    }

    pub fn next_action(&mut self) -> Option<AppAction> {
        self.queue.pop_front().map(route_event)
    }

    pub fn finish(&mut self) {
        debug_assert!(self.queue.is_empty());
        self.dispatching = false;
    }
}

fn route_event(event: AppEvent) -> AppAction {
    match event {
        AppEvent::Intent(intent) => AppAction::ApplyIntent(intent),
        AppEvent::Runtime(event) => AppAction::ApplyRuntime(match event {
            RuntimeEvent::PlaybackUpdate(update) => RuntimeAction::ApplyPlaybackUpdate(update),
            RuntimeEvent::MidiInput(packet) => RuntimeAction::HandleMidiInput(packet),
            RuntimeEvent::MidiInputFailed(error) => RuntimeAction::ReportMidiInputFailure(error),
            RuntimeEvent::SampleBrowserFinished(result) => {
                RuntimeAction::ApplySampleBrowserResult(result)
            }
            RuntimeEvent::ProjectLoaded {
                request_id,
                path,
                result,
            } => RuntimeAction::ApplyProjectLoad {
                request_id,
                path,
                result,
            },
            RuntimeEvent::MidiImported { path, result } => {
                RuntimeAction::ApplyMidiImport { path, result }
            }
            RuntimeEvent::ProjectSaved {
                path,
                song,
                quit_after,
                result,
            } => RuntimeAction::ApplyProjectSave {
                path,
                song,
                quit_after,
                result,
            },
            RuntimeEvent::TaskUpdate(update) => RuntimeAction::ApplyTaskUpdate(update),
            RuntimeEvent::Notification(notification) => {
                RuntimeAction::ShowNotification(notification)
            }
            RuntimeEvent::ViewportRefresh {
                visible_rows,
                visible_tracks,
            } => RuntimeAction::KeepActiveViewportVisible {
                visible_rows,
                visible_tracks,
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    #[test]
    fn dispatcher_preserves_fifo_order_for_nested_events() {
        let mut dispatcher = AppDispatcher::default();
        assert!(
            dispatcher.enqueue(AppEvent::Intent(AppIntent::KeyInput(KeyEvent::new(
                KeyCode::Char('i'),
                KeyModifiers::NONE,
            ))))
        );
        assert!(
            !dispatcher.enqueue(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
                visible_rows: 12,
                visible_tracks: 4,
            }))
        );

        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyIntent(AppIntent::KeyInput(KeyEvent {
                code: KeyCode::Char('i'),
                ..
            })))
        ));
        assert!(
            !dispatcher.enqueue(AppEvent::Runtime(RuntimeEvent::Notification(
                NotificationRequest::new(NotificationLevel::Info, "ready")
            )))
        );
        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyRuntime(
                RuntimeAction::KeepActiveViewportVisible {
                    visible_rows: 12,
                    visible_tracks: 4,
                }
            ))
        ));
        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyRuntime(RuntimeAction::ShowNotification(
                NotificationRequest { message, .. }
            )))
                if message == "ready"
        ));
        assert!(dispatcher.next_action().is_none());
        dispatcher.finish();
    }

    #[test]
    fn non_mutating_viewport_event_routes_without_domain_payload() {
        let action = route_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
            visible_rows: 24,
            visible_tracks: 6,
        }));

        assert!(matches!(
            action,
            AppAction::ApplyRuntime(RuntimeAction::KeepActiveViewportVisible {
                visible_rows: 24,
                visible_tracks: 6,
            })
        ));
    }

    #[test]
    fn runtime_and_project_events_use_the_same_dispatch_queue() {
        let mut dispatcher = AppDispatcher::default();
        assert!(
            dispatcher.enqueue(AppEvent::Runtime(RuntimeEvent::PlaybackUpdate(
                PlaybackUpdate::Stopped,
            )))
        );
        assert!(
            !dispatcher.enqueue(AppEvent::Runtime(RuntimeEvent::ProjectLoaded {
                request_id: RequestId::new(7),
                path: PathBuf::from("song.trk"),
                result: Box::new(Ok(Song::empty())),
            }))
        );

        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyRuntime(RuntimeAction::ApplyPlaybackUpdate(
                PlaybackUpdate::Stopped
            )))
        ));
        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyRuntime(RuntimeAction::ApplyProjectLoad {
                request_id,
                path,
                result,
            })) if request_id == RequestId::new(7)
                && path.as_path() == std::path::Path::new("song.trk")
                && result.is_ok()
        ));
        dispatcher.finish();
    }
}
