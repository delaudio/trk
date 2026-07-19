use super::*;

impl App {
    pub(crate) fn move_cursor(&mut self, direction: Direction) {
        let row_count = self.current_row_count();
        let track_count = self.song.tracks.len();
        self.cursor.move_in(direction, row_count, track_count);
    }

    pub(crate) fn next_track(&mut self) {
        if self.song.tracks.is_empty() {
            return;
        }
        self.cursor.track = self
            .cursor
            .track
            .saturating_add(1)
            .min(self.song.tracks.len().saturating_sub(1));
        self.cursor.digit = 0;
    }

    pub(crate) fn previous_track(&mut self) {
        self.cursor.track = self.cursor.track.saturating_sub(1);
        self.cursor.digit = 0;
    }

    pub(crate) fn page_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(16);
    }

    pub(crate) fn page_cursor_down(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(16)
            .min(self.current_row_count().saturating_sub(1));
    }

    pub(crate) fn insert_note(&mut self, pitch: u8) {
        let pattern_index = self.pattern_index;
        self.mutate_song_with(
            TransactionSpec::merged("Type note", "tracker.typing"),
            |song, cursor| {
                let Some(pattern) = song.pattern_mut(pattern_index) else {
                    return;
                };
                let _ = pattern.set_note(
                    cursor.row,
                    cursor.track,
                    NoteEvent::Note { pitch },
                    DEFAULT_NOTE_VELOCITY,
                );
            },
        );
        self.advance_after_edit();
    }

    pub(crate) fn insert_note_event(&mut self, note: NoteEvent) {
        let pattern_index = self.pattern_index;
        self.mutate_song_with(
            TransactionSpec::merged("Type note", "tracker.typing"),
            |song, cursor| {
                let Some(pattern) = song.pattern_mut(pattern_index) else {
                    return;
                };
                let _ = pattern.set_note_event(cursor.row, cursor.track, note, None);
            },
        );
        self.advance_after_edit();
    }

    pub(crate) fn enter_cell_hex_digit(&mut self, digit: u8) {
        let current_digit = self.cursor.digit.min(1);
        let field = self.cursor.field;
        let pattern_index = self.pattern_index;
        let merge_key = format!(
            "tracker.hex.{pattern_index}.{}.{}.{}",
            self.cursor.row, self.cursor.track, field
        );
        self.mutate_song_with(
            TransactionSpec::merged("Enter cell value", merge_key),
            |song, cursor| {
                let Some(pattern) = song.pattern_mut(pattern_index) else {
                    return;
                };
                let current_value = pattern
                    .cell(cursor.row, cursor.track)
                    .and_then(|cell| current_cell_hex_value(cell, field))
                    .unwrap_or(0);
                let next_value = if current_digit == 0 {
                    (digit << 4) | (current_value & 0x0f)
                } else {
                    (current_value & 0xf0) | digit
                };
                if let Some(cell) = pattern.cell_mut(cursor.row, cursor.track) {
                    set_current_cell_hex_value(cell, field, next_value);
                }
            },
        );

        if current_digit == 0 {
            self.cursor.digit = 1;
        } else {
            self.cursor.digit = 0;
            self.advance_after_edit();
        }
    }

    pub(crate) fn clear_current_cell(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.clear_cell(cursor.row, cursor.track);
        });
    }

    pub(crate) fn copy_current_cell(&mut self) {
        self.clipboard = self
            .song
            .pattern(self.pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .cloned()
            .map(Clipboard::Cell);
    }

    pub(crate) fn cut_current_cell(&mut self) {
        self.copy_current_cell();
        self.clear_current_cell();
    }

    pub(crate) fn copy_selection_or_current_cell(&mut self) {
        if let Some(selection) = self.selection_rect() {
            self.copy_selection(selection);
        } else {
            self.copy_current_cell();
        }
    }

    pub(crate) fn cut_selection_or_current_cell(&mut self) {
        if self.selection_anchor.is_some() {
            if let Some(selection) = self.selection_rect() {
                self.copy_selection(selection);
                self.clear_region(selection);
                self.selection_anchor = None;
            }
        } else {
            self.cut_current_cell();
        }
    }

    pub(crate) fn paste_clipboard(&mut self) {
        let Some(clipboard) = self.clipboard.clone() else {
            return;
        };
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            match clipboard {
                Clipboard::Cell(cell) => {
                    let _ = pattern.set_cell(cursor.row, cursor.track, cell);
                }
                Clipboard::Region(region) => {
                    for (row_offset, row) in region.cells.iter().enumerate() {
                        for (track_offset, cell) in row.iter().enumerate() {
                            let _ = pattern.set_cell(
                                cursor.row.saturating_add(row_offset),
                                cursor.track.saturating_add(track_offset),
                                cell.clone(),
                            );
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn start_selection(&mut self) {
        self.selection_anchor = Some(SelectionAnchor {
            row: self.cursor.row,
            track: self.cursor.track,
        });
    }

    pub(crate) fn selection_rect(&self) -> Option<SelectionRect> {
        let anchor = self.selection_anchor?;
        let row_count = self.current_row_count();
        let track_count = self.song.tracks.len();
        if row_count == 0 || track_count == 0 {
            return None;
        }

        let anchor_row = anchor.row.min(row_count.saturating_sub(1));
        let cursor_row = self.cursor.row.min(row_count.saturating_sub(1));
        let anchor_track = anchor.track.min(track_count.saturating_sub(1));
        let cursor_track = self.cursor.track.min(track_count.saturating_sub(1));

        Some(SelectionRect {
            row_start: anchor_row.min(cursor_row),
            row_end: anchor_row.max(cursor_row),
            track_start: anchor_track.min(cursor_track),
            track_end: anchor_track.max(cursor_track),
        })
    }

    pub(crate) fn copy_selection(&mut self, selection: SelectionRect) {
        let Some(pattern) = self.song.pattern(self.pattern_index) else {
            return;
        };
        let cells = (selection.row_start..=selection.row_end)
            .map(|row| {
                (selection.track_start..=selection.track_end)
                    .map(|track| pattern.cell(row, track).cloned().unwrap_or_default())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        self.clipboard = Some(Clipboard::Region(ClipboardRegion { cells }));
    }

    pub(crate) fn clear_selection_region(&mut self) {
        if let Some(selection) = self.selection_rect() {
            self.clear_region(selection);
            self.selection_anchor = None;
        }
    }

    pub(crate) fn clear_region(&mut self, selection: SelectionRect) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, _| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            for row in selection.row_start..=selection.row_end {
                for track in selection.track_start..=selection.track_end {
                    let _ = pattern.clear_cell(row, track);
                }
            }
        });
    }

    pub(crate) fn insert_current_row(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let _ = song.insert_pattern_row(pattern_index, cursor.row);
        });
    }

    pub(crate) fn delete_current_row(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let _ = song.delete_pattern_row(pattern_index, cursor.row);
        });
        self.clamp_cursor();
    }

    pub(crate) fn create_track(&mut self) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            song.create_track();
        });

        if self.song.tracks.len() > before_count {
            self.cursor.track = self.song.tracks.len().saturating_sub(1);
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
        }
    }

    pub(crate) fn request_delete_current_track(&mut self) {
        self.request_delete_track(self.cursor.track);
    }

    pub(crate) fn request_delete_track(&mut self, track_index: usize) {
        if self.song.tracks.len() <= 1 {
            self.notify_warning("Cannot delete the last track");
            return;
        }

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        self.dialog = Some(Dialog::DeleteTrack {
            track_index,
            message: format!("Delete track {:02} {}?", track_index + 1, track.name),
        });
        self.capture_focus(FocusCapture::Dialog, AppMode::Dialog);
        self.notify_warning("Confirm track delete");
    }

    pub(crate) fn delete_track(&mut self, track: usize) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_track(track);
        });

        if self.song.tracks.len() < before_count {
            self.clamp_cursor();
            self.cursor.digit = 0;
            self.notify_success("Track deleted");
        }
    }

    pub(crate) fn duplicate_track(&mut self, track_index: usize) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.duplicate_track(track_index);
        });

        if self.song.tracks.len() > before_count {
            self.cursor.track = self.song.tracks.len().saturating_sub(1);
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
        }
    }

    pub(crate) fn move_track(&mut self, from: usize, to: usize) {
        let before = self.song.clone();
        self.mutate_song(|song, _| {
            let _ = song.move_track(from, to);
        });

        if self.song != before {
            self.cursor.track = to.min(self.song.tracks.len().saturating_sub(1));
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
            self.notify_success("Track moved");
        }
    }

    pub(crate) fn move_current_track_left(&mut self) {
        if self.cursor.track == 0 {
            self.notify_warning("Track already at first position");
            return;
        }

        self.move_track(self.cursor.track, self.cursor.track - 1);
    }

    pub(crate) fn move_current_track_right(&mut self) {
        let next_track = self.cursor.track.saturating_add(1);
        if next_track >= self.song.tracks.len() {
            self.notify_warning("Track already at last position");
            return;
        }

        self.move_track(self.cursor.track, next_track);
    }

    pub(crate) fn toggle_current_mute(&mut self) {
        self.toggle_track_mute(self.cursor.track);
    }

    pub(crate) fn toggle_current_solo(&mut self) {
        self.toggle_track_solo(self.cursor.track);
    }

    pub(crate) fn toggle_track_mute(&mut self, track_index: usize) {
        if track_index >= self.song.tracks.len() {
            self.notify_warning("Track out of range");
            return;
        }

        self.mutate_song(|song, _| {
            let _ = song.toggle_mute(track_index);
        });
    }

    pub(crate) fn toggle_track_solo(&mut self, track_index: usize) {
        if track_index >= self.song.tracks.len() {
            self.notify_warning("Track out of range");
            return;
        }

        self.mutate_song(|song, _| {
            let _ = song.toggle_solo(track_index);
        });
    }

    pub(crate) fn set_track_midi_channel(&mut self, track_index: usize, midi_channel: u8) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_track_midi_channel(track_index, midi_channel) {
            self.notify_warning(format!("Track channel failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success(format!("Track channel set to {midi_channel}"));
    }

    pub(crate) fn rename_track(&mut self, track_index: usize, name: String) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.rename_track(track_index, name) {
            self.notify_warning(format!("Track rename failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success("Track renamed");
    }

    pub(crate) fn start_track_rename_command(&mut self) {
        self.command_buffer = format!("track rename {} ", self.cursor.track + 1);
        self.capture_command_mode();
        self.notify_info("Rename current track");
    }

    pub(crate) fn start_track_channel_command(&mut self) {
        self.command_buffer = format!("track channel {} ", self.cursor.track + 1);
        self.capture_command_mode();
        self.notify_info("Set current track MIDI channel");
    }
}
