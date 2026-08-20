use std::path::PathBuf;

use crossterm::event::KeyEvent;
use trk_core::{Cursor, PatternVariationHistory, Song};

use crate::{
    app_effect::{AppEffect, AppEffectExecutor, RuntimeEffectExecutor},
    app_event::{AppAction, AppEvent, AppIntent, RuntimeAction, RuntimeEvent},
    focus::FocusPanel,
    playback_runtime::PlaybackUpdate,
    App,
};

impl App {
    pub(super) fn dispatch_event(&mut self, event: AppEvent) {
        self.dispatch_event_with(event, &mut RuntimeEffectExecutor);
    }

    pub(crate) fn dispatch_event_with(
        &mut self,
        event: AppEvent,
        effects: &mut impl AppEffectExecutor,
    ) {
        if !self.dispatcher.enqueue(event) {
            return;
        }
        while let Some(action) = self.dispatcher.next_action() {
            for effect in self.apply_action(action) {
                effects.execute(self, effect);
            }
        }
        self.dispatcher.finish();
    }

    fn apply_action(&mut self, action: AppAction) -> Vec<AppEffect> {
        match action {
            AppAction::ApplyIntent(intent) => self.apply_intent(intent),
            AppAction::ApplyRuntime(action) => self.apply_runtime_action(action),
        }
    }

    fn apply_runtime_action(&mut self, action: RuntimeAction) -> Vec<AppEffect> {
        match action {
            RuntimeAction::ApplyPlaybackUpdate(update) => self.apply_playback_update(update),
            RuntimeAction::HandleMidiInput(packet) => self.apply_midi_input_packet(packet),
            RuntimeAction::ReportMidiInputFailure(error) => {
                self.midi_input_status = format!("MIDI In Error: {error}");
                self.notify_error(format!("MIDI input error: {error}"));
            }
            RuntimeAction::ApplySampleBrowserResult(result) => {
                self.apply_sample_browser_result(result)
            }
            RuntimeAction::ApplyProjectLoad {
                request_id,
                path,
                result,
            } => return self.apply_project_load(request_id, path, *result),
            RuntimeAction::ApplyMidiImport { path, result } => {
                self.apply_midi_import(path, *result)
            }
            RuntimeAction::ApplyProjectSave {
                path,
                project,
                quit_after,
                result,
            } => self.apply_project_save(path, *project, quit_after, result),
            RuntimeAction::ApplyTaskUpdate(update) => self.apply_task_update(update),
            RuntimeAction::ShowNotification(notification) => self.show_notification(notification),
            RuntimeAction::KeepActiveViewportVisible {
                visible_rows,
                visible_tracks,
            } => {
                self.keep_active_viewport_visible(visible_rows, visible_tracks);
            }
        }
        Vec::new()
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        self.dispatch_intent(AppIntent::KeyInput(key));
    }

    pub(super) fn drain_playback_updates(&mut self) {
        while let Some(update) = self.playback.try_recv() {
            self.dispatch_event(AppEvent::Runtime(RuntimeEvent::PlaybackUpdate(update)));
        }
    }

    fn apply_playback_update(&mut self, update: PlaybackUpdate) {
        match update {
            PlaybackUpdate::Position(position) => {
                self.is_playing = true;
                self.pattern_index = position
                    .pattern_index
                    .min(self.song.patterns.len().saturating_sub(1));
                self.sequence_position = position.sequence_index.and_then(|sequence_index| {
                    (!self.song.sequence.is_empty())
                        .then_some(sequence_index.min(self.song.sequence.len().saturating_sub(1)))
                });
                if let Some(sequence_position) = self
                    .sequence_position
                    .or_else(|| self.sequence_position_for_pattern_index(self.pattern_index))
                {
                    self.sequence_cursor =
                        sequence_position.min(self.song.sequence.len().saturating_sub(1));
                }
                self.playhead_row = Some(
                    position
                        .position
                        .row
                        .min(self.current_row_count().saturating_sub(1)),
                );
            }
            PlaybackUpdate::Stopped => {
                self.is_playing = false;
                self.parameter_surface.pending_reload = None;
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
            PlaybackUpdate::PerformanceReloaded { token } => {
                self.apply_pending_performance_reload(token);
            }
        }
    }

    fn apply_midi_import(&mut self, path: PathBuf, result: Result<Song, String>) {
        match result {
            Ok(song) => {
                self.clear_performance_state_for_project_change();
                let pattern_count = song.patterns.len();
                let track_count = song.tracks.len();
                self.song = song;
                self.project_path = None;
                self.clean_song = Song::empty();
                self.variation_history = PatternVariationHistory::default();
                self.clean_variation_history = PatternVariationHistory::default();
                self.variation_history_open = false;
                self.variation_history_cursor = 0;
                self.pattern_index = 0;
                self.cursor = Cursor::new();
                self.row_offset = 0;
                self.track_offset = 0;
                self.selection = None;
                self.history.clear();
                self.is_playing = false;
                self.playhead_row = None;
                self.sequence_position = None;
                self.sequence_cursor = 0;
                self.clip_scene_cursor = 0;
                self.clip_track_cursor = 0;
                self.active_clip_scene = None;
                self.queued_clip_scene = None;
                self.clamp_cursor();
                self.clamp_sequence_cursor();
                self.clamp_clip_cursor();
                self.refresh_dirty();
                self.dirty = true;
                self.focus_panel(FocusPanel::Tracker);
                self.notify_success(format!(
                    "MIDI imported: {} ({} tracks, {} patterns)",
                    path.display(),
                    track_count,
                    pattern_count
                ));
            }
            Err(error) => {
                tracing::error!(%error, path = %path.display(), "failed to import MIDI");
                self.notify_error(error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crossterm::event::{KeyCode, KeyModifiers};
    use trk_core::{Direction, Song};

    use super::*;
    use crate::{
        app_effect::PlaybackEffect,
        app_event::{AiIntent, NavigationIntent, ParameterIntent, TrackerIntent, TransportIntent},
        command::TrkCommand,
        AppMode,
    };

    #[derive(Default)]
    struct RecordingEffects {
        effects: Vec<AppEffect>,
    }

    impl AppEffectExecutor for RecordingEffects {
        fn execute(&mut self, _app: &mut App, effect: AppEffect) {
            self.effects.push(effect);
        }
    }

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

        app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
            visible_rows: 4,
            visible_tracks: 2,
        }));

        assert_eq!(app.row_offset, 7);
        assert_eq!(app.track_offset, 0);
        assert_eq!(app.song, before);
    }

    #[test]
    fn nested_playback_notification_is_drained_after_state_update() {
        let mut app = App {
            is_playing: true,
            playhead_row: Some(3),
            ..App::default()
        };

        app.dispatch_event(AppEvent::Runtime(RuntimeEvent::PlaybackUpdate(
            PlaybackUpdate::Stopped,
        )));

        assert!(!app.is_playing);
        assert_eq!(app.playhead_row, None);
        assert_eq!(
            app.notification
                .as_ref()
                .map(|notification| notification.message.as_str()),
            Some("Playback stopped")
        );
    }

    #[test]
    fn representative_intents_reduce_only_their_owned_state() {
        let mut app = App::default();
        let mut effects = RecordingEffects::default();

        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::Navigation(NavigationIntent::MoveCursor(
                Direction::Down,
            ))),
            &mut effects,
        );
        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::Tracker(TrackerIntent::InsertNote(64))),
            &mut effects,
        );
        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::Parameter(ParameterIntent::SetBpm(150))),
            &mut effects,
        );
        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::Command(TrkCommand::SetLinesPerBeat(6))),
            &mut effects,
        );

        assert_eq!(app.cursor.row, 2);
        assert_eq!(
            app.song.pattern(0).expect("pattern").rows[1].cells[0]
                .note
                .as_ref()
                .and_then(|note| match note {
                    trk_core::NoteEvent::Note { pitch } => Some(*pitch),
                    _ => None,
                }),
            Some(64)
        );
        assert_eq!(app.song.transport.bpm, 150);
        assert_eq!(app.song.transport.lines_per_beat, 6);
        assert!(effects.effects.is_empty());
    }

    #[test]
    fn playback_and_ai_intents_emit_injectable_effects() {
        let mut app = App::default();
        let mut effects = RecordingEffects::default();

        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::Transport(TransportIntent::StartPattern)),
            &mut effects,
        );
        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::Ai(AiIntent::Propose("four beats".to_string()))),
            &mut effects,
        );
        let save_path =
            std::env::temp_dir().join(format!("trk-recorded-effect-{}.trk", std::process::id()));
        let _ = std::fs::remove_file(&save_path);
        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::SaveProject {
                path: Some(save_path.clone()),
                quit_after: false,
            }),
            &mut effects,
        );

        assert!(app.is_playing);
        assert!(matches!(
            effects.effects.first(),
            Some(AppEffect::Playback(PlaybackEffect::StartPattern {
                pattern_index: 0,
                row: 0,
                ..
            }))
        ));
        assert!(matches!(
            effects.effects.get(1),
            Some(AppEffect::SubmitAiProposal { request, .. })
                if request.prompt == "four beats"
        ));
        assert!(matches!(
            effects.effects.get(2),
            Some(AppEffect::SaveProject { path, .. }) if path == &save_path
        ));
        assert!(app.task_runtime.is_idle());
        assert!(!save_path.exists());
    }

    #[test]
    fn stale_project_results_cannot_replace_a_newer_request() {
        let mut app = App::default();
        let mut effects = RecordingEffects::default();
        let first_path = PathBuf::from("first.trk");
        let second_path = PathBuf::from("second.trk");

        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::OpenProject(first_path.clone())),
            &mut effects,
        );
        app.dispatch_event_with(
            AppEvent::Intent(AppIntent::OpenProject(second_path.clone())),
            &mut effects,
        );

        let request_ids = effects
            .effects
            .iter()
            .filter_map(|effect| match effect {
                AppEffect::LoadProject { request_id, .. } => Some(*request_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(request_ids.len(), 2);

        app.dispatch_event_with(
            AppEvent::Runtime(RuntimeEvent::ProjectLoaded {
                request_id: request_ids[0],
                path: first_path,
                result: Box::new(Ok(crate::persistence::ProjectFile::new(Song::empty()))),
            }),
            &mut effects,
        );
        assert!(app.project_path.is_none());
        assert_eq!(app.pending_project_load, Some(request_ids[1]));

        app.dispatch_event_with(
            AppEvent::Runtime(RuntimeEvent::ProjectLoaded {
                request_id: request_ids[1],
                path: second_path.clone(),
                result: Box::new(Ok(crate::persistence::ProjectFile::new(Song::empty()))),
            }),
            &mut effects,
        );
        assert_eq!(app.project_path.as_deref(), Some(Path::new("second.trk")));
        assert!(app.pending_project_load.is_none());
        assert_eq!(
            app.notification
                .as_ref()
                .map(|notification| notification.message.as_str()),
            Some("Project opened: second.trk")
        );
    }
}
