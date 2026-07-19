use std::{collections::VecDeque, path::PathBuf};

use crossterm::event::KeyEvent;
use salieri_ai::{AiProposal, CellAddress};
use salieri_core::Song;
use salieri_midi::MidiInputPacket;

use crate::playback_runtime::PlaybackUpdate;

#[derive(Debug)]
pub enum AppEvent {
    TerminalInput(KeyEvent),
    PlaybackUpdate(PlaybackUpdate),
    MidiInput(MidiInputPacket),
    MidiInputFailed(String),
    SampleBrowserFinished(Result<Option<PathBuf>, String>),
    ProjectLoaded {
        path: PathBuf,
        result: Box<Result<Song, String>>,
    },
    AiProposalPrepared(Result<PreparedAiProposal, String>),
    Notification(NotificationRequest),
    ViewportRefresh {
        visible_rows: usize,
    },
}

#[derive(Debug)]
pub enum AppAction {
    HandleTerminalInput(KeyEvent),
    ApplyPlaybackUpdate(PlaybackUpdate),
    HandleMidiInput(MidiInputPacket),
    ReportMidiInputFailure(String),
    ApplySampleBrowserResult(Result<Option<PathBuf>, String>),
    ApplyProjectLoad {
        path: PathBuf,
        result: Box<Result<Song, String>>,
    },
    ApplyAiProposal(Result<PreparedAiProposal, String>),
    ShowNotification(NotificationRequest),
    KeepActiveRowVisible {
        visible_rows: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedAiProposal {
    pub proposal: AiProposal,
    pub touched_cells: Vec<CellAddress>,
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
        AppEvent::TerminalInput(key) => AppAction::HandleTerminalInput(key),
        AppEvent::PlaybackUpdate(update) => AppAction::ApplyPlaybackUpdate(update),
        AppEvent::MidiInput(packet) => AppAction::HandleMidiInput(packet),
        AppEvent::MidiInputFailed(error) => AppAction::ReportMidiInputFailure(error),
        AppEvent::SampleBrowserFinished(result) => AppAction::ApplySampleBrowserResult(result),
        AppEvent::ProjectLoaded { path, result } => AppAction::ApplyProjectLoad { path, result },
        AppEvent::AiProposalPrepared(result) => AppAction::ApplyAiProposal(result),
        AppEvent::Notification(notification) => AppAction::ShowNotification(notification),
        AppEvent::ViewportRefresh { visible_rows } => {
            AppAction::KeepActiveRowVisible { visible_rows }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

    #[test]
    fn dispatcher_preserves_fifo_order_for_nested_events() {
        let mut dispatcher = AppDispatcher::default();
        assert!(dispatcher.enqueue(AppEvent::TerminalInput(KeyEvent::new(
            KeyCode::Char('i'),
            KeyModifiers::NONE,
        ))));
        assert!(!dispatcher.enqueue(AppEvent::ViewportRefresh { visible_rows: 12 }));

        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::HandleTerminalInput(KeyEvent {
                code: KeyCode::Char('i'),
                ..
            }))
        ));
        assert!(
            !dispatcher.enqueue(AppEvent::Notification(NotificationRequest::new(
                NotificationLevel::Info,
                "ready"
            )))
        );
        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::KeepActiveRowVisible { visible_rows: 12 })
        ));
        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ShowNotification(NotificationRequest { message, .. }))
                if message == "ready"
        ));
        assert!(dispatcher.next_action().is_none());
        dispatcher.finish();
    }

    #[test]
    fn non_mutating_viewport_event_routes_without_domain_payload() {
        let action = route_event(AppEvent::ViewportRefresh { visible_rows: 24 });

        assert!(matches!(
            action,
            AppAction::KeepActiveRowVisible { visible_rows: 24 }
        ));
    }

    #[test]
    fn runtime_and_project_events_use_the_same_dispatch_queue() {
        let mut dispatcher = AppDispatcher::default();
        assert!(dispatcher.enqueue(AppEvent::PlaybackUpdate(PlaybackUpdate::Stopped)));
        assert!(!dispatcher.enqueue(AppEvent::ProjectLoaded {
            path: PathBuf::from("song.salieri"),
            result: Box::new(Ok(Song::empty())),
        }));

        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyPlaybackUpdate(PlaybackUpdate::Stopped))
        ));
        assert!(matches!(
            dispatcher.next_action(),
            Some(AppAction::ApplyProjectLoad { path, result })
                if path.as_path() == std::path::Path::new("song.salieri") && result.is_ok()
        ));
        dispatcher.finish();
    }
}
