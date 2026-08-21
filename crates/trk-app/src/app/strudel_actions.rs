use super::*;

impl App {
    pub(crate) fn handle_strudel_command(&mut self, arguments: &[String]) {
        if arguments.first().is_some_and(|value| value == "live") {
            self.open_strudel_live(arguments[1..].join(" "));
            return;
        }
        let expression = arguments.join(" ");
        if expression.trim().is_empty() {
            self.notify_warning("Usage: :strudel EXPR | :strudel live [EXPR]");
            return;
        }
        match self.apply_strudel_command(&expression) {
            Ok(evaluation) => self.notify_success(format!(
                "Strudel wrote {} events across {} track(s)",
                evaluation.writes.len(),
                evaluation.track_count
            )),
            Err(error) => self.notify_warning(error),
        }
    }

    pub(crate) fn handle_strudel_live_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.cancel_strudel_live(),
            KeyCode::Enter => self.accept_strudel_live(),
            KeyCode::Backspace => {
                if let Some(session) = &mut self.strudel_live {
                    session.buffer.pop();
                }
                self.refresh_strudel_live();
            }
            KeyCode::Char(value) => {
                if let Some(session) = &mut self.strudel_live {
                    session.buffer.push(value);
                }
                self.refresh_strudel_live();
            }
            _ => {}
        }
    }

    fn apply_strudel_command(&mut self, expression: &str) -> Result<Evaluation, String> {
        let program = Program::parse(expression).map_err(|error| error.to_string())?;
        let pattern_index = self.pattern_index;
        let start_track = self.cursor.track;
        let mut evaluation = None;
        self.try_mutate_song(TransactionSpec::new("Apply Strudel pattern"), |song, _| {
            let available_tracks = song.tracks.len().saturating_sub(start_track);
            evaluation = Some(evaluate_into_song(
                song,
                pattern_index,
                start_track,
                available_tracks,
                &program,
            )?);
            Ok::<(), String>(())
        })?;
        self.update_live_playback_pattern(pattern_index);
        Ok(evaluation.expect("successful Strudel evaluation records a report"))
    }

    pub(crate) fn open_strudel_live(&mut self, expression: String) {
        self.strudel_live = Some(StrudelLiveSession {
            entry_song: self.song.clone(),
            pattern_index: self.pattern_index,
            start_track: self.cursor.track,
            line: format!("strudel live {expression}"),
            buffer: expression,
            error: None,
            last_evaluation: None,
        });
        self.capture_focus(FocusCapture::Command, AppMode::Strudel);
        self.refresh_strudel_live();
    }

    fn refresh_strudel_live(&mut self) {
        let Some(session) = &self.strudel_live else {
            return;
        };
        let expression = session.buffer.clone();
        let pattern_index = session.pattern_index;
        let start_track = session.start_track;
        let previous_width = session
            .last_evaluation
            .as_ref()
            .map_or(0, |evaluation| evaluation.track_count);
        if expression.trim().is_empty() {
            if let Some(session) = &mut self.strudel_live {
                session.error = Some("expression cannot be empty".to_string());
            }
            self.update_strudel_live_line();
            return;
        }
        let entry_song = session.entry_song.clone();
        let current_song = self.song.clone();
        let result = Program::parse(&expression)
            .map_err(|error| error.to_string())
            .and_then(|program| {
                let available_tracks = entry_song.tracks.len().saturating_sub(start_track);
                let mut preview_pattern = entry_song
                    .pattern(pattern_index)
                    .cloned()
                    .ok_or_else(|| format!("pattern {} does not exist", pattern_index + 1))?;
                let row_count = preview_pattern.row_count();
                let evaluation = apply_strudel(
                    &mut preview_pattern,
                    &program,
                    EvaluateOptions::for_pattern(row_count, start_track, available_tracks),
                )
                .map_err(|error| error.to_string())?;
                let mut preview = current_song;
                let target = preview
                    .pattern_mut(pattern_index)
                    .ok_or_else(|| format!("pattern {} does not exist", pattern_index + 1))?;
                copy_pattern_tracks(
                    target,
                    &preview_pattern,
                    start_track,
                    previous_width.max(evaluation.track_count),
                )?;
                Ok((preview, evaluation))
            });
        match result {
            Ok((preview, evaluation)) => {
                self.song = preview;
                self.variation_history.reconcile(&self.song);
                self.refresh_dirty();
                if let Some(session) = &mut self.strudel_live {
                    session.error = None;
                    session.last_evaluation = Some(evaluation);
                }
                self.update_live_playback_pattern(pattern_index);
            }
            Err(error) => {
                if let Some(session) = &mut self.strudel_live {
                    session.error = Some(error);
                }
            }
        }
        self.update_strudel_live_line();
    }

    fn accept_strudel_live(&mut self) {
        if let Some(error) = self
            .strudel_live
            .as_ref()
            .and_then(|session| session.error.clone())
        {
            self.notify_warning(format!(
                "Fix the Strudel expression before accepting: {error}"
            ));
            return;
        }
        let Some(session) = self.strudel_live.take() else {
            return;
        };
        let event_count = session
            .last_evaluation
            .as_ref()
            .map_or(0, |evaluation| evaluation.writes.len());
        let mut transaction = SongTransaction::new(&session.entry_song);
        *transaction.song_mut() = self.song.clone();
        let changed = self.history.commit(
            &mut self.song,
            transaction,
            TransactionSpec::new("Apply Strudel live pattern"),
        );
        self.close_focus_capture();
        self.variation_history.reconcile(&self.song);
        self.refresh_dirty();
        if changed {
            self.notify_success(format!(
                "Strudel live pattern accepted ({event_count} events)"
            ));
        } else {
            self.notify_info("Strudel live pattern unchanged");
        }
    }

    fn cancel_strudel_live(&mut self) {
        let Some(session) = self.strudel_live.take() else {
            return;
        };
        let pattern_index = session.pattern_index;
        self.song = session.entry_song;
        self.variation_history.reconcile(&self.song);
        self.refresh_dirty();
        self.update_live_playback_pattern(pattern_index);
        self.close_focus_capture();
        self.notify_info("Strudel live changes cancelled");
    }

    fn update_strudel_live_line(&mut self) {
        if let Some(session) = &mut self.strudel_live {
            session.line = session.error.as_ref().map_or_else(
                || format!("strudel live {}", session.buffer),
                |error| format!("strudel live {}  ! {error}", session.buffer),
            );
        }
    }

    pub(crate) fn update_live_playback_pattern(&self, pattern_index: usize) {
        if !self.is_playing {
            return;
        }
        if let Some(pattern) = self.song.pattern(pattern_index) {
            self.playback
                .replace_pattern(pattern_index, pattern.clone());
        }
    }
}

fn copy_pattern_tracks(
    target: &mut trk_core::Pattern,
    source: &trk_core::Pattern,
    start_track: usize,
    track_count: usize,
) -> Result<(), String> {
    if target.row_count() != source.row_count() {
        return Err("target pattern changed size during Strudel live editing".to_string());
    }
    for row in 0..source.row_count() {
        for track in start_track..start_track.saturating_add(track_count) {
            let cell = source
                .cell(row, track)
                .cloned()
                .ok_or_else(|| "target track changed during Strudel live editing".to_string())?;
            let target_cell = target
                .cell_mut(row, track)
                .ok_or_else(|| "target track changed during Strudel live editing".to_string())?;
            *target_cell = cell;
        }
    }
    Ok(())
}

fn evaluate_into_song(
    song: &mut Song,
    pattern_index: usize,
    start_track: usize,
    available_tracks: usize,
    program: &Program,
) -> Result<Evaluation, String> {
    let pattern = song
        .pattern_mut(pattern_index)
        .ok_or_else(|| format!("pattern {} does not exist", pattern_index + 1))?;
    apply_strudel(
        pattern,
        program,
        EvaluateOptions::for_pattern(pattern.row_count(), start_track, available_tracks),
    )
    .map_err(|error| error.to_string())
}
