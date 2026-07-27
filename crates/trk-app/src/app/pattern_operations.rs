use super::*;

impl App {
    pub(crate) fn copy_pattern_operation(&mut self) {
        if let Some(selection) = self.selection_bounds() {
            self.copy_selection(selection);
            self.notify_success("Selection copied");
        } else {
            let Some(pattern) = self.song.pattern(self.pattern_index) else {
                return;
            };
            self.clipboard = Some(Clipboard::Region(ClipboardRegion {
                cells: pattern.rows.iter().map(|row| row.cells.clone()).collect(),
            }));
            self.notify_success("Pattern copied");
        }
    }

    pub(crate) fn paste_pattern_operation(&mut self) {
        let before = self.song.clone();
        self.paste_clipboard();
        if self.song != before {
            self.notify_success("Pattern paste applied");
        }
    }

    pub(crate) fn fill_pattern_operation(&mut self) {
        let source_position = if self.selection.is_some() {
            self.selection_bounds()
                .map(|bounds| (bounds.row_start, bounds.track_start))
        } else {
            Some((self.cursor.row, self.cursor.track))
        };
        let Some(bounds) = self.pattern_operation_bounds() else {
            return;
        };
        let Some((source_row, source_track)) = source_position else {
            return;
        };
        let Some(source) = self
            .song
            .pattern(self.pattern_index)
            .and_then(|pattern| pattern.cell(source_row, source_track))
            .cloned()
        else {
            return;
        };
        let pattern_index = self.pattern_index;
        let changed = self.mutate_pattern_operation("Fill pattern", |song| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            for row in bounds.row_start..=bounds.row_end {
                for track in bounds.track_start..=bounds.track_end {
                    let _ = pattern.set_cell(row, track, source.clone());
                }
            }
        });
        if changed {
            self.notify_success("Pattern fill applied");
        }
    }

    pub(crate) fn invert_pattern_operation(&mut self) {
        let Some(bounds) = self.pattern_operation_bounds() else {
            return;
        };
        let pattern_index = self.pattern_index;
        let changed = self.mutate_pattern_operation("Invert pattern", |song| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let height = bounds.row_end.saturating_sub(bounds.row_start) + 1;
            for offset in 0..(height / 2) {
                let top = bounds.row_start + offset;
                let bottom = bounds.row_end - offset;
                for track in bounds.track_start..=bounds.track_end {
                    let top_cell = pattern.cell(top, track).cloned().unwrap_or_default();
                    let bottom_cell = pattern.cell(bottom, track).cloned().unwrap_or_default();
                    let _ = pattern.set_cell(top, track, bottom_cell);
                    let _ = pattern.set_cell(bottom, track, top_cell);
                }
            }
        });
        if changed {
            self.notify_success("Pattern inverted");
        }
    }

    pub(crate) fn duplicate_pattern_region_operation(&mut self) {
        let Some(bounds) = self.pattern_operation_bounds() else {
            return;
        };
        let pattern_index = self.pattern_index;
        let track_count = self.song.tracks.len();
        let changed = self.mutate_pattern_operation("Duplicate pattern region", |song| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let copied_rows = (bounds.row_start..=bounds.row_end)
                .map(|row| {
                    (bounds.track_start..=bounds.track_end)
                        .map(|track| pattern.cell(row, track).cloned().unwrap_or_default())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let insert_at = bounds.row_end.saturating_add(1).min(pattern.rows.len());
            for row_offset in 0..copied_rows.len() {
                pattern
                    .rows
                    .insert(insert_at + row_offset, blank_row(track_count));
            }
            for (row_offset, copied_row) in copied_rows.into_iter().enumerate() {
                for (track_offset, cell) in copied_row.into_iter().enumerate() {
                    let _ = pattern.set_cell(
                        insert_at + row_offset,
                        bounds.track_start + track_offset,
                        cell,
                    );
                }
            }
        });
        if changed {
            self.notify_success("Pattern region duplicated");
        }
        self.clamp_cursor();
    }

    pub(crate) fn expand_pattern_operation(&mut self) {
        let Some(bounds) = self.pattern_operation_bounds() else {
            return;
        };
        let pattern_index = self.pattern_index;
        let track_count = self.song.tracks.len();
        let changed = self.mutate_pattern_operation("Expand pattern", |song| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let insert_count = bounds.row_end.saturating_sub(bounds.row_start) + 1;
            for offset in 0..insert_count {
                let insert_at = bounds.row_start + (offset * 2) + 1;
                pattern.rows.insert(insert_at, blank_row(track_count));
            }
        });
        if changed {
            self.notify_success("Pattern expanded");
        }
        self.clamp_cursor();
    }

    pub(crate) fn shrink_pattern_operation(&mut self) {
        let Some(bounds) = self.pattern_operation_bounds() else {
            return;
        };
        let pattern_index = self.pattern_index;
        let changed = self.mutate_pattern_operation("Shrink pattern", |song| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let removable_rows = (bounds.row_start..=bounds.row_end)
                .filter(|row| (row - bounds.row_start) % 2 == 1)
                .collect::<Vec<_>>();
            for row in removable_rows.into_iter().rev() {
                if pattern.rows.len() > 1 {
                    pattern.rows.remove(row);
                }
            }
        });
        if changed {
            self.notify_success("Pattern shrunk");
        }
        self.clamp_cursor();
    }

    fn pattern_operation_bounds(&self) -> Option<SelectionBounds> {
        self.selection_bounds().or_else(|| {
            TrackerSelection::pattern(self.selection_endpoint())
                .bounds(self.current_row_count(), self.song.tracks.len())
        })
    }

    fn mutate_pattern_operation(
        &mut self,
        label: &'static str,
        mutate: impl FnOnce(&mut Song),
    ) -> bool {
        let result = self.try_mutate_song(TransactionSpec::new(label), |song, _| {
            mutate(song);
            Ok::<(), std::convert::Infallible>(())
        });
        result.unwrap_or(false)
    }
}

fn blank_row(track_count: usize) -> PatternRow {
    PatternRow {
        cells: vec![PatternCell::default(); track_count],
    }
}
