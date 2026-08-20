use super::*;

impl App {
    pub(crate) fn handle_piano_roll_key(&mut self, key: KeyEvent) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char(':') => self.open_command_prompt(),
            KeyCode::Char('?') => self.open_help(),
            KeyCode::Left if shift => self.resize_piano_roll_gate(-1),
            KeyCode::Right if shift => self.resize_piano_roll_gate(1),
            KeyCode::Left if alt => self.move_piano_roll_note(-1, 0),
            KeyCode::Right if alt => self.move_piano_roll_note(1, 0),
            KeyCode::Up if alt => self.move_piano_roll_note(0, 1),
            KeyCode::Down if alt => self.move_piano_roll_note(0, -1),
            KeyCode::Left => self.cursor.row = self.cursor.row.saturating_sub(1),
            KeyCode::Right => {
                self.cursor.row = self
                    .cursor
                    .row
                    .saturating_add(1)
                    .min(self.current_row_count().saturating_sub(1));
            }
            KeyCode::Up => self.piano_roll_pitch = self.piano_roll_pitch.saturating_add(1).min(127),
            KeyCode::Down => self.piano_roll_pitch = self.piano_roll_pitch.saturating_sub(1),
            KeyCode::Char(' ') => self.toggle_piano_roll_note(),
            KeyCode::Char('g' | 'G') => {
                self.piano_roll_ghosts = !self.piano_roll_ghosts;
                self.notify_info(if self.piano_roll_ghosts {
                    "Piano Roll ghost notes ON"
                } else {
                    "Piano Roll ghost notes OFF"
                });
            }
            KeyCode::Char('[') => self.change_piano_roll_zoom(-1),
            KeyCode::Char(']') => self.change_piano_roll_zoom(1),
            KeyCode::Char(digit @ '1'..='9') => {
                self.set_piano_roll_velocity(digit.to_digit(10).unwrap_or(1) as u8);
            }
            _ => {}
        }
    }

    fn toggle_piano_roll_note(&mut self) {
        let pattern_index = self.pattern_index;
        let pitch = self.piano_roll_pitch;
        let cell = self
            .song
            .patterns
            .get(pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .cloned()
            .unwrap_or_default();
        let remove = matches!(cell.note, Some(NoteEvent::Note { pitch: value }) if value == pitch);
        if !remove && cell != PatternCell::default() {
            self.notify_warning("Piano Roll cell is occupied at another pitch");
            return;
        }
        self.mutate_song_with(
            TransactionSpec::new("Toggle Piano Roll note"),
            |song, cursor| {
                let Some(pattern) = song.pattern_mut(pattern_index) else {
                    return;
                };
                if remove {
                    let _ = pattern.clear_cell(cursor.row, cursor.track);
                } else {
                    let _ = pattern.set_note(
                        cursor.row,
                        cursor.track,
                        NoteEvent::Note { pitch },
                        DEFAULT_NOTE_VELOCITY,
                    );
                    let _ = pattern.set_gate(cursor.row, cursor.track, Some(1));
                }
            },
        );
    }

    fn resize_piano_roll_gate(&mut self, delta: i8) {
        let pattern_index = self.pattern_index;
        let pitch = self.piano_roll_pitch;
        if !self.piano_roll_cursor_note_matches() {
            self.notify_warning("No note at the Piano Roll cursor pitch");
            return;
        }
        self.mutate_song_with(
            TransactionSpec::merged("Resize Piano Roll note", "piano-roll.gate"),
            move |song, cursor| {
                let Some(pattern) = song.pattern_mut(pattern_index) else {
                    return;
                };
                let remaining = pattern.row_count().saturating_sub(cursor.row).clamp(1, 127) as u8;
                let Some(cell) = pattern.cell(cursor.row, cursor.track) else {
                    return;
                };
                if !matches!(cell.note, Some(NoteEvent::Note { pitch: value }) if value == pitch) {
                    return;
                }
                let current = cell.gate.unwrap_or(1).clamp(1, remaining);
                let next = if delta < 0 {
                    current.saturating_sub(delta.unsigned_abs()).max(1)
                } else {
                    current.saturating_add(delta as u8).min(remaining)
                };
                let _ = pattern.set_gate(cursor.row, cursor.track, Some(next));
            },
        );
    }

    fn move_piano_roll_note(&mut self, row_delta: i8, pitch_delta: i8) {
        let pattern_index = self.pattern_index;
        let source_pitch = self.piano_roll_pitch;
        if !self.piano_roll_cursor_note_matches() {
            self.notify_warning("No note at the Piano Roll cursor pitch");
            return;
        }
        let destination_row = if row_delta < 0 {
            self.cursor
                .row
                .checked_sub(row_delta.unsigned_abs() as usize)
        } else {
            self.cursor.row.checked_add(row_delta as usize)
        };
        let destination_pitch = if pitch_delta < 0 {
            self.piano_roll_pitch
                .checked_sub(pitch_delta.unsigned_abs())
        } else {
            self.piano_roll_pitch
                .checked_add(pitch_delta as u8)
                .filter(|pitch| *pitch <= 127)
        };
        let (Some(destination_row), Some(destination_pitch)) = (destination_row, destination_pitch)
        else {
            return;
        };
        if destination_row >= self.current_row_count() {
            return;
        }
        let moved = self.try_mutate_song(
            TransactionSpec::new("Move Piano Roll note"),
            move |song, cursor| -> Result<(), ()> {
                let pattern = song.pattern_mut(pattern_index).ok_or(())?;
                let source = pattern.cell(cursor.row, cursor.track).cloned().ok_or(())?;
                if !matches!(source.note, Some(NoteEvent::Note { pitch }) if pitch == source_pitch)
                {
                    return Err(());
                }
                if destination_row != cursor.row
                    && pattern
                        .cell(destination_row, cursor.track)
                        .is_some_and(|cell| *cell != PatternCell::default())
                {
                    return Err(());
                }
                let mut moved = source;
                moved.note = Some(NoteEvent::Note {
                    pitch: destination_pitch,
                });
                if destination_row != cursor.row {
                    let _ = pattern.clear_cell(cursor.row, cursor.track);
                }
                *pattern.cell_mut(destination_row, cursor.track).ok_or(())? = moved;
                Ok(())
            },
        );
        if matches!(moved, Ok(true)) {
            self.cursor.row = destination_row;
            self.piano_roll_pitch = destination_pitch;
        } else if moved.is_err() {
            self.notify_warning("Piano Roll destination is occupied");
        }
    }

    fn set_piano_roll_velocity(&mut self, digit: u8) {
        let pattern_index = self.pattern_index;
        let pitch = self.piano_roll_pitch;
        if !self.piano_roll_cursor_note_matches() {
            self.notify_warning("No note at the Piano Roll cursor pitch");
            return;
        }
        let percent = if digit == 9 { 100 } else { digit * 10 };
        let velocity = ((u16::from(percent) * 127 + 50) / 100) as u8;
        self.mutate_song_with(
            TransactionSpec::merged("Set Piano Roll velocity", "piano-roll.velocity"),
            move |song, cursor| {
                let Some(cell) = song
                    .pattern_mut(pattern_index)
                    .and_then(|pattern| pattern.cell_mut(cursor.row, cursor.track))
                else {
                    return;
                };
                if matches!(cell.note, Some(NoteEvent::Note { pitch: value }) if value == pitch) {
                    cell.velocity = Some(velocity);
                }
            },
        );
    }

    fn piano_roll_cursor_note_matches(&self) -> bool {
        self.song
            .patterns
            .get(self.pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .is_some_and(|cell| {
                matches!(cell.note, Some(NoteEvent::Note { pitch }) if pitch == self.piano_roll_pitch)
            })
    }

    fn change_piano_roll_zoom(&mut self, direction: i8) {
        const ZOOMS: [u8; 3] = [16, 32, 64];
        let current = ZOOMS
            .iter()
            .position(|zoom| *zoom == self.piano_roll_rows)
            .unwrap_or(0);
        let next = if direction < 0 {
            current.saturating_sub(1)
        } else {
            current.saturating_add(1).min(ZOOMS.len() - 1)
        };
        self.piano_roll_rows = ZOOMS[next];
    }
}
