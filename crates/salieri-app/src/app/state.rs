use super::*;

impl App {
    pub(crate) fn mutate_song(&mut self, mutate: impl FnOnce(&mut Song, Cursor)) {
        let before = self.song.clone();
        mutate(&mut self.song, self.cursor);
        if self.song != before {
            self.undo_stack.push(before);
            if self.undo_stack.len() > UNDO_LIMIT {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
            self.refresh_dirty();
            self.clamp_sequence_cursor();
        }
    }

    pub(crate) fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.song, previous);
            self.redo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.notify_info("Undo");
        } else {
            self.notify_warning("Nothing to undo");
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.song, next);
            self.undo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.notify_info("Redo");
        } else {
            self.notify_warning("Nothing to redo");
        }
    }

    pub(crate) fn advance_after_edit(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(self.edit_step)
            .min(self.current_row_count().saturating_sub(1));
    }

    pub(crate) fn increment_octave(&mut self) {
        self.octave = self.octave.saturating_add(1).min(9);
    }

    pub(crate) fn decrement_octave(&mut self) {
        self.octave = self.octave.saturating_sub(1);
    }

    pub(crate) fn start_save_as_command(&mut self) {
        let path = self.project_path.as_ref().map_or_else(
            || "untitled.salieri".to_string(),
            |path| path.display().to_string(),
        );
        self.command_buffer = format!("saveas {path}");
        self.mode = AppMode::Command;
        self.notify_info("Save As: edit path and press Enter");
    }

    pub(crate) fn refresh_dirty(&mut self) {
        self.dirty = self.song != self.clean_song;
    }

    pub(crate) fn save(&mut self) {
        self.dispatch_intent(AppIntent::SaveProject {
            path: None,
            quit_after: false,
        });
    }

    pub(crate) fn save_as(&mut self, path: PathBuf) {
        self.dispatch_intent(AppIntent::SaveProject {
            path: Some(path),
            quit_after: false,
        });
    }

    pub(crate) fn save_and_quit(&mut self) {
        self.dispatch_intent(AppIntent::SaveProject {
            path: None,
            quit_after: true,
        });
    }

    pub(crate) fn apply_project_save(
        &mut self,
        path: PathBuf,
        saved_song: Song,
        quit_after: bool,
        result: std::result::Result<(), String>,
    ) {
        match result {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.clean_song = saved_song;
                self.refresh_dirty();
                self.record_recent_project(path);
                self.notify_success("Project saved");
                if quit_after {
                    self.force_quit();
                }
            }
            Err(error) => {
                tracing::error!(%error, "failed to save project");
                self.notify_error(format!("Save failed: {error}"));
            }
        }
    }

    pub(crate) fn clamp_cursor(&mut self) {
        self.clamp_pattern_index();
        self.cursor
            .clamp(self.current_row_count(), self.song.tracks.len());
    }

    pub(crate) fn clamp_pattern_index(&mut self) {
        self.pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
    }

    pub(crate) fn clamp_sequence_cursor(&mut self) {
        if self.song.sequence.is_empty() {
            self.sequence_cursor = 0;
        } else {
            self.sequence_cursor = self
                .sequence_cursor
                .min(self.song.sequence.len().saturating_sub(1));
        }
    }

    pub(crate) fn tui_sequence_position(&self) -> Option<usize> {
        if self.is_playing {
            return self
                .sequence_position
                .or_else(|| self.sequence_position_for_pattern_index(self.pattern_index));
        }
        self.sequence_position.or_else(|| {
            (!self.song.sequence.is_empty()).then_some(
                self.sequence_cursor
                    .min(self.song.sequence.len().saturating_sub(1)),
            )
        })
    }

    pub(crate) fn sequence_position_for_pattern_index(
        &self,
        pattern_index: usize,
    ) -> Option<usize> {
        let pattern_id = self.song.pattern(pattern_index)?.id;
        self.song
            .sequence
            .iter()
            .position(|sequence_pattern| *sequence_pattern == pattern_id)
    }

    pub(crate) fn tui_active_view(&self) -> TuiView {
        match self.mode {
            AppMode::Sequence => TuiView::Sequence,
            AppMode::Tracks => TuiView::Tracks,
            AppMode::Patterns => TuiView::Patterns,
            AppMode::Sampler => TuiView::Sampler,
            AppMode::SampleBrowser => TuiView::SampleBrowser,
            AppMode::ProjectBrowser => TuiView::ProjectBrowser,
            AppMode::Normal
            | AppMode::Edit
            | AppMode::Command
            | AppMode::Help
            | AppMode::Dialog
            | AppMode::MidiSettings => TuiView::Pattern,
        }
    }

    pub(crate) fn keep_cursor_visible(&mut self, visible_rows: usize) {
        self.keep_row_visible(self.cursor.row, visible_rows);
    }

    pub(crate) fn keep_active_row_visible(&mut self, visible_rows: usize) {
        let row = if self.is_playing && self.follow_playhead {
            self.playhead_row.unwrap_or(self.cursor.row)
        } else {
            self.cursor.row
        };
        self.keep_row_visible(row, visible_rows);
    }

    pub(crate) fn keep_row_visible(&mut self, row: usize, visible_rows: usize) {
        let visible_rows = visible_rows.max(1);
        if row < self.row_offset {
            self.row_offset = row;
        } else if row >= self.row_offset.saturating_add(visible_rows) {
            self.row_offset = row.saturating_sub(visible_rows - 1);
        }

        let max_offset = self.current_row_count().saturating_sub(visible_rows);
        self.row_offset = self.row_offset.min(max_offset);
    }

    pub(crate) fn current_row_count(&self) -> usize {
        self.song
            .pattern(self.pattern_index)
            .map_or(0, |pattern| pattern.row_count())
    }

    pub(crate) fn command_line(&self) -> Option<&str> {
        if self.mode == AppMode::Command {
            Some(self.command_buffer.as_str())
        } else {
            None
        }
    }

    pub(crate) fn quit_confirmation(&self) -> bool {
        self.mode == AppMode::Dialog && matches!(self.dialog, Some(Dialog::QuitDirty))
    }

    pub(crate) fn delete_confirmation_message(&self) -> Option<&str> {
        if self.mode != AppMode::Dialog {
            return None;
        }

        match &self.dialog {
            Some(Dialog::DeleteTrack { message, .. }) => Some(message.as_str()),
            Some(Dialog::DeletePattern { message, .. }) => Some(message.as_str()),
            Some(Dialog::OpenProjectDirty { message, .. }) => Some(message.as_str()),
            Some(Dialog::QuitDirty) | None => None,
        }
    }
}
