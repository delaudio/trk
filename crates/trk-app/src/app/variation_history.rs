use super::*;

impl App {
    pub(crate) fn open_variation_history(&mut self) {
        self.variation_history_open = true;
        self.variation_history_cursor = self
            .variation_history
            .active_id()
            .and_then(|active| {
                self.variation_history
                    .entries()
                    .iter()
                    .position(|entry| entry.id == active)
            })
            .unwrap_or_else(|| self.variation_history.entries().len().saturating_sub(1));
        self.notify_info("Pattern history: Up/Down select, Enter restore, Esc close");
    }

    pub(crate) fn close_variation_history(&mut self) {
        self.variation_history_open = false;
    }

    pub(crate) fn select_next_variation(&mut self) {
        let len = self.variation_history.entries().len();
        if len > 0 {
            self.variation_history_cursor = (self.variation_history_cursor + 1) % len;
        }
    }

    pub(crate) fn select_previous_variation(&mut self) {
        let len = self.variation_history.entries().len();
        if len > 0 {
            self.variation_history_cursor = self
                .variation_history_cursor
                .checked_sub(1)
                .unwrap_or(len - 1);
        }
    }

    pub(crate) fn restore_selected_variation(&mut self) {
        let Some(entry) = self
            .variation_history
            .entries()
            .get(self.variation_history_cursor)
            .cloned()
        else {
            self.notify_warning("No pattern variations to restore");
            return;
        };
        let pattern_index = entry.pattern_index;
        let snapshot = entry.snapshot;
        let result = self.try_mutate_song(
            TransactionSpec::new(format!("Restore {}", entry.id)),
            |song, _| {
                let Some(pattern) = song.patterns.get_mut(pattern_index) else {
                    return Err(format!("pattern {} no longer exists", pattern_index + 1));
                };
                *pattern = snapshot;
                song.validate()
                    .map_err(|error| format!("snapshot is incompatible: {error}"))
            },
        );
        match result {
            Ok(false) => {
                self.variation_history_open = false;
                self.notify_info(format!("{} already matches the current pattern", entry.id));
            }
            Ok(true) => {
                if let Err(error) = self.variation_history.set_active(entry.id) {
                    self.notify_warning(format!("History restore failed: {error}"));
                    return;
                }
                self.pattern_index = pattern_index;
                if let Some(track) = entry.track_index {
                    self.cursor.track = track.min(self.song.tracks.len().saturating_sub(1));
                }
                self.clamp_cursor();
                self.variation_history_open = false;
                self.refresh_dirty();
                self.notify_success(format!("Restored {}: {}", entry.id, entry.description));
            }
            Err(error) => self.notify_warning(format!("History restore failed: {error}")),
        }
    }

    pub(crate) fn handle_variation_history_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') => self.close_variation_history(),
            KeyCode::Up | KeyCode::Char('k') => self.select_previous_variation(),
            KeyCode::Down | KeyCode::Char('j') => self.select_next_variation(),
            KeyCode::Enter => self.restore_selected_variation(),
            _ => {}
        }
    }
}
