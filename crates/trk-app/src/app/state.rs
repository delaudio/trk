use super::*;

impl App {
    pub(crate) fn mutate_song(&mut self, mutate: impl FnOnce(&mut Song, Cursor)) {
        self.mutate_song_with(TransactionSpec::new("Edit song"), mutate);
    }

    pub(crate) fn mutate_song_with(
        &mut self,
        spec: TransactionSpec,
        mutate: impl FnOnce(&mut Song, Cursor),
    ) {
        let result = self.try_mutate_song(spec, |song, cursor| {
            mutate(song, cursor);
            Ok::<(), std::convert::Infallible>(())
        });
        debug_assert!(result.is_ok());
    }

    pub(crate) fn try_mutate_song<E>(
        &mut self,
        spec: TransactionSpec,
        mutate: impl FnOnce(&mut Song, Cursor) -> Result<(), E>,
    ) -> Result<bool, E> {
        self.transact_song(spec, |transaction, cursor| {
            transaction.nested(|nested| mutate(nested.song_mut(), cursor))
        })
    }

    pub(crate) fn transact_song<E>(
        &mut self,
        spec: TransactionSpec,
        edit: impl FnOnce(&mut SongTransaction, Cursor) -> Result<(), E>,
    ) -> Result<bool, E> {
        let mut transaction = SongTransaction::new(&self.song);
        edit(&mut transaction, self.cursor)?;
        let changed = self.history.commit(&mut self.song, transaction, spec);
        if changed {
            self.refresh_dirty();
            self.clamp_sequence_cursor();
            self.clamp_clip_cursor();
        }
        Ok(changed)
    }

    pub(crate) fn undo(&mut self) {
        if let Some(label) = self.history.undo(&mut self.song) {
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.clamp_clip_cursor();
            self.notify_info(format!("Undo: {label}"));
        } else {
            self.notify_warning("Nothing to undo");
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(label) = self.history.redo(&mut self.song) {
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.clamp_clip_cursor();
            self.notify_info(format!("Redo: {label}"));
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
            || "untitled.trk".to_string(),
            |path| path.display().to_string(),
        );
        self.command_buffer = format!("saveas {path}");
        self.capture_command_mode();
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

    pub(crate) fn resolve_project_save_path(
        &self,
        path: Option<PathBuf>,
    ) -> Result<PathBuf, String> {
        match path {
            Some(path) if is_bare_project_path(&path) => {
                if let Some(library) = &self.project_library {
                    ensure_library_dir(library)?;
                    Ok(library.join(project_name_with_extension(path)))
                } else {
                    Ok(path)
                }
            }
            Some(path) => Ok(path),
            None => {
                if let Some(path) = &self.project_path {
                    Ok(path.clone())
                } else if let Some(library) = &self.project_library {
                    ensure_library_dir(library)?;
                    Ok(library.join("untitled.trk"))
                } else {
                    Ok(PathBuf::from("untitled.trk"))
                }
            }
        }
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

    pub(crate) fn clamp_clip_cursor(&mut self) {
        if self.song.clip_scenes.is_empty() {
            self.clip_scene_cursor = 0;
            self.active_clip_scene = None;
            self.queued_clip_scene = None;
        } else {
            let max_scene = self.song.clip_scenes.len().saturating_sub(1);
            self.clip_scene_cursor = self.clip_scene_cursor.min(max_scene);
            self.active_clip_scene = self.active_clip_scene.filter(|scene| *scene <= max_scene);
            self.queued_clip_scene = self.queued_clip_scene.filter(|scene| *scene <= max_scene);
        }
        if self.song.tracks.is_empty() {
            self.clip_track_cursor = 0;
        } else {
            self.clip_track_cursor = self
                .clip_track_cursor
                .min(self.song.tracks.len().saturating_sub(1));
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
        self.focus.focused().tui_view()
    }

    pub(crate) fn focus_panel(&mut self, panel: FocusPanel) {
        self.focus.focus(panel);
        self.mode = panel.app_mode();
    }

    pub(crate) fn restore_previous_focus(&mut self) {
        self.mode = self.focus.restore_previous().app_mode();
    }

    pub(crate) fn capture_focus(&mut self, capture: FocusCapture, mode: AppMode) {
        self.focus.capture_input(capture);
        self.mode = mode;
    }

    pub(crate) fn close_focus_capture(&mut self) {
        self.mode = self.focus.release_capture().app_mode();
    }

    pub(crate) fn open_command_prompt(&mut self) {
        self.command_buffer.clear();
        self.capture_command_mode();
    }

    pub(crate) fn capture_command_mode(&mut self) {
        self.capture_focus(FocusCapture::Command, AppMode::Command);
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

    pub(crate) fn keep_active_viewport_visible(
        &mut self,
        visible_rows: usize,
        visible_tracks: usize,
    ) {
        self.keep_active_row_visible(visible_rows);
        self.keep_track_visible(self.cursor.track, visible_tracks);
    }

    pub(crate) fn keep_row_visible(&mut self, row: usize, visible_rows: usize) {
        let mut viewport = ViewportAxis::with_offset(
            self.current_row_count(),
            visible_rows.max(1),
            self.row_offset,
        );
        viewport.keep_visible(row);
        self.row_offset = viewport.offset();
    }

    pub(crate) fn keep_track_visible(&mut self, track: usize, visible_tracks: usize) {
        let mut viewport = ViewportAxis::with_offset(
            self.song.tracks.len(),
            visible_tracks.max(1),
            self.track_offset,
        );
        viewport.keep_visible(track);
        self.track_offset = viewport.offset();
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

fn is_bare_project_path(path: &Path) -> bool {
    path.is_relative() && path.components().count() == 1
}

fn project_name_with_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("trk");
    }
    path
}

fn ensure_library_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| {
        format!(
            "failed to create project library {}: {error}",
            path.display()
        )
    })
}
