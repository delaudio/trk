use super::*;

impl App {
    pub(crate) fn set_bpm(&mut self, bpm: u16) {
        self.mutate_song(|song, _| {
            song.transport.bpm = bpm;
        });
    }

    pub(crate) fn adjust_bpm(&mut self, delta: i16) {
        let bpm = (i32::from(self.song.transport.bpm) + i32::from(delta))
            .clamp(i32::from(MIN_BPM), i32::from(MAX_BPM)) as u16;
        self.set_bpm(bpm);
        self.notify_info(format!("BPM {bpm}"));
    }

    pub(crate) fn set_lpb(&mut self, lpb: u8) {
        self.mutate_song(|song, _| {
            song.transport.lines_per_beat = lpb;
        });
    }

    pub(crate) fn adjust_lpb(&mut self, delta: i8) {
        let lpb = (i16::from(self.song.transport.lines_per_beat) + i16::from(delta))
            .clamp(i16::from(MIN_LPB), i16::from(MAX_LPB)) as u8;
        self.set_lpb(lpb);
        self.notify_info(format!("LPB {lpb}"));
    }

    pub(crate) fn create_pattern(&mut self) {
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            song.create_pattern(64);
        });
        if self.song.patterns.len() > before_count {
            self.pattern_index = self.song.patterns.len().saturating_sub(1);
            self.cursor.row = 0;
            self.row_offset = 0;
        }
    }

    pub(crate) fn duplicate_current_pattern(&mut self) {
        let pattern_index = self.pattern_index;
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.duplicate_pattern(pattern_index);
        });
        if self.song.patterns.len() > before_count {
            self.pattern_index = self.song.patterns.len().saturating_sub(1);
            self.cursor.row = 0;
            self.row_offset = 0;
        }
    }

    pub(crate) fn request_delete_current_pattern(&mut self) {
        if self.song.patterns.len() <= 1 {
            self.notify_warning("Cannot delete the last pattern");
            return;
        }

        let pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
        let Some(pattern) = self.song.patterns.get(pattern_index) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        self.dialog = Some(Dialog::DeletePattern {
            pattern_index,
            message: format!("Delete pattern {:02} {}?", pattern_index + 1, pattern.name),
        });
        self.mode = AppMode::Dialog;
        self.notify_warning("Confirm pattern delete");
    }

    pub(crate) fn delete_pattern(&mut self, pattern_index: usize) {
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_pattern(pattern_index);
        });
        if self.song.patterns.len() < before_count {
            self.pattern_index = self
                .pattern_index
                .min(self.song.patterns.len().saturating_sub(1));
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.row_offset = 0;
            self.notify_success("Pattern deleted");
        }
    }

    pub(crate) fn resize_current_pattern(&mut self, row_count: usize) {
        let pattern_index = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.resize_pattern(pattern_index, row_count) {
            self.notify_warning(format!("Pattern length failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.clamp_cursor();
        self.keep_cursor_visible(1);
        self.notify_success(format!("Pattern length set to {row_count}"));
    }

    pub(crate) fn start_pattern_length_command(&mut self) {
        self.command_buffer = "pattern length ".to_string();
        self.mode = AppMode::Command;
        self.notify_info("Set current pattern length");
    }

    pub(crate) fn rename_current_pattern(&mut self, name: String) {
        let pattern_index = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.rename_pattern(pattern_index, name) {
            self.notify_warning(format!("Pattern rename failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success("Pattern renamed");
    }

    pub(crate) fn start_pattern_rename_command(&mut self) {
        self.command_buffer = "pattern rename ".to_string();
        self.mode = AppMode::Command;
        self.notify_info("Rename current pattern");
    }

    pub(crate) fn select_pattern(&mut self, pattern_index: usize) {
        if pattern_index < self.song.patterns.len() {
            self.pattern_index = pattern_index;
            self.clamp_cursor();
            self.row_offset = 0;
        }
    }

    pub(crate) fn selected_sequence_position(&mut self) -> Option<usize> {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return None;
        }

        self.clamp_sequence_cursor();
        Some(self.sequence_cursor)
    }

    pub(crate) fn previous_sequence_position(&mut self) {
        self.sequence_cursor = self.sequence_cursor.saturating_sub(1);
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    pub(crate) fn next_sequence_position(&mut self) {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return;
        }

        self.sequence_cursor = self
            .sequence_cursor
            .saturating_add(1)
            .min(self.song.sequence.len().saturating_sub(1));
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    pub(crate) fn add_sequence_pattern(&mut self, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        let before_len = self.song.sequence.len();
        self.mutate_song(|song, _| {
            let _ = song.push_sequence_pattern(pattern_id);
        });
        if self.song.sequence.len() > before_len {
            self.sequence_cursor = self.song.sequence.len().saturating_sub(1);
        }
        self.notify_success(format!("Sequence added pattern {:02}", pattern_index + 1));
    }

    pub(crate) fn remove_sequence_position(&mut self, position: usize) {
        let before_len = self.song.sequence.len();
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.remove_sequence_position(position) {
            self.notify_warning(format!("Sequence remove failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        if self.song.sequence.len() < before_len {
            self.sequence_cursor = position.min(self.song.sequence.len().saturating_sub(1));
        }
        self.clamp_sequence_cursor();
        self.notify_success(format!("Sequence removed position {position:02}"));
    }

    pub(crate) fn duplicate_sequence_position(&mut self, position: usize) {
        let before_len = self.song.sequence.len();
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.duplicate_sequence_position(position) {
            self.notify_warning(format!("Sequence duplicate failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        if self.song.sequence.len() > before_len {
            self.sequence_cursor = position.saturating_add(1);
            self.clamp_sequence_cursor();
        }
        self.notify_success(format!("Sequence duplicated position {position:02}"));
    }

    pub(crate) fn duplicate_selected_sequence_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.duplicate_sequence_position(position);
        }
    }

    pub(crate) fn remove_selected_sequence_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.remove_sequence_position(position);
        }
    }

    pub(crate) fn set_sequence_pattern(&mut self, position: usize, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_sequence_pattern(position, pattern_id) {
            self.notify_warning(format!("Sequence set failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success(format!(
            "Sequence position {position:02} set to pattern {:02}",
            pattern_index + 1
        ));
    }

    pub(crate) fn set_selected_sequence_to_current_pattern(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.set_sequence_pattern(position, self.pattern_index);
        }
    }

    pub(crate) fn move_sequence_position(&mut self, from: usize, to: usize) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.move_sequence_position(from, to) {
            self.notify_warning(format!("Sequence move failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.sequence_cursor = to;
        self.notify_success(format!("Sequence moved position {from:02} to {to:02}"));
    }

    pub(crate) fn move_selected_sequence_position_up(&mut self) {
        let Some(position) = self.selected_sequence_position() else {
            return;
        };
        if position == 0 {
            self.notify_warning("Sequence already at first position");
            return;
        }
        self.move_sequence_position(position, position - 1);
    }

    pub(crate) fn move_selected_sequence_position_down(&mut self) {
        let Some(position) = self.selected_sequence_position() else {
            return;
        };
        let next_position = position.saturating_add(1);
        if next_position >= self.song.sequence.len() {
            self.notify_warning("Sequence already at last position");
            return;
        }
        self.move_sequence_position(position, next_position);
    }
}
