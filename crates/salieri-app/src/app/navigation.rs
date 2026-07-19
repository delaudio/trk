use super::*;

impl App {
    pub(crate) fn request_quit(&mut self, force: bool) {
        if force || !self.dirty {
            self.force_quit();
        } else {
            self.stop_playback();
            self.dialog = Some(Dialog::QuitDirty);
            self.capture_focus(FocusCapture::Dialog, AppMode::Dialog);
            self.notify_warning("Unsaved changes");
        }
    }

    pub(crate) fn force_quit(&mut self) {
        self.stop_playback();
        self.dialog = None;
        self.should_quit = true;
    }

    pub(crate) fn cancel_dialog(&mut self) {
        let return_to_project_browser =
            matches!(self.dialog, Some(Dialog::OpenProjectDirty { .. }))
                && self.project_browser_view.is_some();
        self.dialog = None;
        if return_to_project_browser {
            self.focus_panel(FocusPanel::ProjectBrowser);
        } else {
            self.close_focus_capture();
        }
        self.notify_info("Cancelled");
    }

    pub(crate) fn toggle_playback(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::TogglePlayback));
    }

    pub(crate) fn toggle_loop(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::ToggleLoop));
    }

    pub(crate) fn start_playback(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::StartPattern));
    }

    pub(crate) fn start_playback_from_cursor(&mut self) {
        self.dispatch_intent(AppIntent::Transport(
            TransportIntent::StartPatternFromCursor,
        ));
    }

    pub(crate) fn start_sequence_playback_at(&mut self, start_sequence_index: usize) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::StartSequence {
            position: start_sequence_index,
        }));
    }

    pub(crate) fn start_sequence_playback_from_selected_position(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::StartSelectedSequence));
    }

    pub(crate) fn stop_playback(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::Stop));
    }

    pub(crate) fn connect_midi(&mut self, port_index: usize) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::ConnectMidi {
            port_index,
        }));
    }

    pub(crate) fn open_midi_settings(&mut self) {
        self.refresh_midi_ports();
        self.focus_panel(FocusPanel::MidiSettings);
    }

    pub(crate) fn open_help(&mut self) {
        self.help_scroll = 0;
        self.help_tab = match self.mode {
            AppMode::Sampler | AppMode::SampleBrowser => HelpTab::Sampler,
            AppMode::ProjectBrowser => HelpTab::Commands,
            AppMode::MidiSettings => HelpTab::Midi,
            AppMode::Command => HelpTab::Commands,
            AppMode::Edit => HelpTab::Editing,
            _ => HelpTab::Basics,
        };
        self.capture_focus(FocusCapture::Help, AppMode::Help);
        if let Some(summary) = self.keymap.help_summary() {
            self.notify_info(summary);
        }
    }

    pub(crate) fn open_tracker_view(&mut self) {
        self.focus_panel(FocusPanel::Tracker);
        self.notify_info("Tracker editor");
    }

    pub(crate) fn open_sequence_view(&mut self) {
        self.clamp_sequence_cursor();
        self.focus_panel(FocusPanel::Sequence);
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    pub(crate) fn open_tracks_view(&mut self) {
        self.cursor.track = self
            .cursor
            .track
            .min(self.song.tracks.len().saturating_sub(1));
        self.focus_panel(FocusPanel::Tracks);
        self.notify_info(format!("Track {:02}", self.cursor.track + 1));
    }

    pub(crate) fn open_patterns_view(&mut self) {
        self.clamp_pattern_index();
        self.focus_panel(FocusPanel::Patterns);
        self.notify_info(format!("Pattern {:02}", self.pattern_index + 1));
    }

    pub(crate) fn open_sampler_view(&mut self) {
        self.focus_panel(FocusPanel::Sampler);
        if let Some(sample) = &self.sample_view {
            self.notify_info(format!("Sample {}", sample.sample.name));
        } else {
            self.notify_info("Sampler view");
        }
    }
}
