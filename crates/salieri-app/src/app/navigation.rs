use super::*;

impl App {
    pub(crate) fn request_quit(&mut self, force: bool) {
        if force || !self.dirty {
            self.force_quit();
        } else {
            self.stop_playback();
            self.mode = AppMode::Dialog;
            self.dialog = Some(Dialog::QuitDirty);
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
        self.mode = if return_to_project_browser {
            AppMode::ProjectBrowser
        } else {
            AppMode::Normal
        };
        self.notify_info("Cancelled");
    }

    pub(crate) fn toggle_playback(&mut self) {
        if self.is_playing {
            self.stop_playback();
        } else {
            self.start_playback();
        }
    }

    pub(crate) fn toggle_loop(&mut self) {
        self.loop_pattern = !self.loop_pattern;
        let state = if self.loop_pattern { "ON" } else { "OFF" };
        self.notify_info(format!("Pattern loop {state}"));
    }

    pub(crate) fn start_playback(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = None;
        self.playback.start_pattern_from(
            self.song.clone(),
            self.sample_base_dir(),
            self.pattern_index,
            0,
            self.loop_pattern,
        );
        self.notify_info("Playing pattern from start");
    }

    pub(crate) fn start_playback_from_cursor(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(self.cursor.row);
        self.sequence_position = None;
        self.playback.start_pattern_from(
            self.song.clone(),
            self.sample_base_dir(),
            self.pattern_index,
            self.cursor.row,
            self.loop_pattern,
        );
        self.notify_info(format!("Playing pattern from row {:02}", self.cursor.row));
    }

    pub(crate) fn start_sequence_playback_at(&mut self, start_sequence_index: usize) {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return;
        }

        if start_sequence_index >= self.song.sequence.len() {
            self.notify_warning("Sequence position out of range");
            return;
        }

        if let Some(first_pattern_id) = self.song.sequence.get(start_sequence_index) {
            if let Some(pattern_index) = self
                .song
                .patterns
                .iter()
                .position(|pattern| pattern.id == *first_pattern_id)
            {
                self.pattern_index = pattern_index;
            }
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = Some(start_sequence_index);
        self.playback.start_sequence(
            self.song.clone(),
            self.sample_base_dir(),
            start_sequence_index,
        );
        self.notify_info(format!("Playing sequence from {start_sequence_index}"));
    }

    pub(crate) fn start_sequence_playback_from_selected_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.start_sequence_playback_at(position);
        }
    }

    pub(crate) fn stop_playback(&mut self) {
        self.playback.stop();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
        self.notify_info("Playback stopped");
    }

    pub(crate) fn connect_midi(&mut self, port_index: usize) {
        self.midi_status = format!("MIDI Connecting {port_index}");
        self.playback.connect_midi(port_index);
        self.notify_info(format!("Connecting MIDI output {port_index}"));
    }

    pub(crate) fn open_midi_settings(&mut self) {
        self.refresh_midi_ports();
        self.mode = AppMode::MidiSettings;
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
        self.mode = AppMode::Help;
        if let Some(summary) = self.keymap.help_summary() {
            self.notify_info(summary);
        }
    }

    pub(crate) fn open_tracker_view(&mut self) {
        self.mode = AppMode::Normal;
        self.notify_info("Tracker editor");
    }

    pub(crate) fn open_sequence_view(&mut self) {
        self.clamp_sequence_cursor();
        self.mode = AppMode::Sequence;
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    pub(crate) fn open_tracks_view(&mut self) {
        self.cursor.track = self
            .cursor
            .track
            .min(self.song.tracks.len().saturating_sub(1));
        self.mode = AppMode::Tracks;
        self.notify_info(format!("Track {:02}", self.cursor.track + 1));
    }

    pub(crate) fn open_patterns_view(&mut self) {
        self.clamp_pattern_index();
        self.mode = AppMode::Patterns;
        self.notify_info(format!("Pattern {:02}", self.pattern_index + 1));
    }

    pub(crate) fn open_sampler_view(&mut self) {
        self.mode = AppMode::Sampler;
        if let Some(sample) = &self.sample_view {
            self.notify_info(format!("Sample {}", sample.sample.name));
        } else {
            self.notify_info("Sampler view");
        }
    }
}
