use std::time::Instant;

use crossterm::event::KeyEvent;
use salieri_tui::NotificationKind;

use crate::{
    app_event::{AppAction, AppEvent, NotificationLevel, NotificationRequest},
    playback_runtime::PlaybackUpdate,
    App, Notification, NOTIFICATION_TTL,
};

impl App {
    pub(super) fn dispatch_event(&mut self, event: AppEvent) {
        if !self.dispatcher.enqueue(event) {
            return;
        }
        while let Some(action) = self.dispatcher.next_action() {
            self.apply_action(action);
        }
        self.dispatcher.finish();
    }

    fn apply_action(&mut self, action: AppAction) {
        match action {
            AppAction::HandleTerminalInput(key) => self.handle_key_action(key),
            AppAction::ApplyPlaybackUpdate(update) => self.apply_playback_update(update),
            AppAction::HandleMidiInput(packet) => self.apply_midi_input_packet(packet),
            AppAction::ReportMidiInputFailure(error) => {
                self.midi_input_status = format!("MIDI In Error: {error}");
                self.notify_error(format!("MIDI input error: {error}"));
            }
            AppAction::ApplySampleBrowserResult(result) => self.apply_sample_browser_result(result),
            AppAction::ApplyProjectLoad { path, result } => self.apply_project_load(path, *result),
            AppAction::ApplyAiProposal(result) => self.apply_prepared_ai_proposal(result),
            AppAction::ShowNotification(notification) => self.show_notification(notification),
            AppAction::KeepActiveRowVisible { visible_rows } => {
                self.keep_active_row_visible(visible_rows);
            }
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        self.dispatch_event(AppEvent::TerminalInput(key));
    }

    pub(super) fn drain_playback_updates(&mut self) {
        while let Some(update) = self.playback.try_recv() {
            self.dispatch_event(AppEvent::PlaybackUpdate(update));
        }
    }

    fn apply_playback_update(&mut self, update: PlaybackUpdate) {
        match update {
            PlaybackUpdate::Position(position) => {
                self.is_playing = true;
                self.pattern_index = position.pattern_index;
                self.sequence_position = position.sequence_index;
                if let Some(sequence_position) = position
                    .sequence_index
                    .or_else(|| self.sequence_position_for_pattern_index(position.pattern_index))
                {
                    self.sequence_cursor =
                        sequence_position.min(self.song.sequence.len().saturating_sub(1));
                }
                self.playhead_row = Some(position.position.row);
            }
            PlaybackUpdate::Stopped => {
                self.is_playing = false;
                self.playhead_row = None;
                self.sequence_position = None;
                self.notify_info("Playback stopped");
            }
            PlaybackUpdate::MidiConnected { port_index } => {
                self.midi_status = format!("MIDI Connected {port_index}");
                self.notify_success(format!("MIDI output connected: {port_index}"));
            }
            PlaybackUpdate::MidiDisconnected => {
                self.midi_status = "MIDI Disconnected".to_string();
                self.notify_info("MIDI output disconnected");
            }
            PlaybackUpdate::MidiError(error) => {
                self.midi_status = format!("MIDI Error: {error}");
                self.is_playing = false;
                self.playhead_row = None;
                self.sequence_position = None;
                self.notify_error(format!("MIDI error: {error}"));
            }
            PlaybackUpdate::MidiLogError(error) => {
                self.midi_status = format!("MIDI Log Error: {error}");
                self.notify_error(format!("MIDI log error: {error}"));
            }
            PlaybackUpdate::AudioError(error) => {
                self.notify_error(format!("Audio error: {error}"));
            }
        }
    }

    fn notify(&mut self, kind: NotificationKind, message: impl Into<String>) {
        let level = match kind {
            NotificationKind::Info => NotificationLevel::Info,
            NotificationKind::Success => NotificationLevel::Success,
            NotificationKind::Warning => NotificationLevel::Warning,
            NotificationKind::Error => NotificationLevel::Error,
        };
        self.dispatch_event(AppEvent::Notification(NotificationRequest::new(
            level, message,
        )));
    }

    fn show_notification(&mut self, notification: NotificationRequest) {
        let kind = match notification.level {
            NotificationLevel::Info => NotificationKind::Info,
            NotificationLevel::Success => NotificationKind::Success,
            NotificationLevel::Warning => NotificationKind::Warning,
            NotificationLevel::Error => NotificationKind::Error,
        };
        self.notification = Some(Notification {
            kind,
            message: notification.message,
            expires_at: Instant::now() + NOTIFICATION_TTL,
        });
    }

    pub(super) fn notify_info(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Info, message);
    }

    pub(super) fn notify_success(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Success, message);
    }

    pub(super) fn notify_warning(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Warning, message);
    }

    pub(super) fn notify_error(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Error, message);
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;
    use crate::AppMode;

    #[test]
    fn terminal_input_dispatches_without_mutating_song() {
        let mut app = App::default();
        let before = app.song.clone();

        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(app.song, before);
    }

    #[test]
    fn viewport_refresh_is_a_non_mutating_action() {
        let mut app = App::default();
        app.cursor.row = 10;
        let before = app.song.clone();

        app.dispatch_event(AppEvent::ViewportRefresh { visible_rows: 4 });

        assert_eq!(app.row_offset, 7);
        assert_eq!(app.song, before);
    }

    #[test]
    fn nested_playback_notification_is_drained_after_state_update() {
        let mut app = App {
            is_playing: true,
            playhead_row: Some(3),
            ..App::default()
        };

        app.dispatch_event(AppEvent::PlaybackUpdate(PlaybackUpdate::Stopped));

        assert!(!app.is_playing);
        assert_eq!(app.playhead_row, None);
        assert_eq!(
            app.notification
                .as_ref()
                .map(|notification| notification.message.as_str()),
            Some("Playback stopped")
        );
    }
}
