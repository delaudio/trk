use super::*;

impl App {
    pub(crate) fn handle_key_action(&mut self, key: KeyEvent) {
        if let Some(command) = self.keymap.command_for(self.mode.keymap_mode(), &key) {
            self.dispatch_intent(AppIntent::Command(command));
            return;
        }
        if self.mode != AppMode::CommandPalette && self.handle_control_key(key) {
            return;
        }

        match self.mode {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::Edit => self.handle_edit_key(key),
            AppMode::Command => self.handle_command_key(key),
            AppMode::CommandPalette => self.handle_command_palette_key(key),
            AppMode::Help => self.handle_help_key(key),
            AppMode::Dialog => self.handle_dialog_key(key),
            AppMode::MidiSettings => self.handle_midi_settings_key(key),
            AppMode::Sequence => self.handle_sequence_key(key),
            AppMode::Clips => self.handle_clip_key(key),
            AppMode::Tracks => self.handle_tracks_key(key),
            AppMode::Patterns => self.handle_patterns_key(key),
            AppMode::Sampler => self.handle_sampler_key(key),
            AppMode::DspRack => self.handle_dsp_rack_key(key),
            AppMode::Ai => self.handle_ai_key(key),
            AppMode::SampleBrowser => self.handle_sample_browser_key(key),
            AppMode::ProjectBrowser => self.handle_project_browser_key(key),
        }
    }

    pub(crate) fn handle_mouse_wheel(&mut self, kind: MouseEventKind) {
        let delta = match kind {
            MouseEventKind::ScrollUp => -3,
            MouseEventKind::ScrollDown => 3,
            _ => return,
        };

        match self.mode {
            AppMode::Help => {
                self.help_scroll = self.help_scroll.saturating_add_signed(delta);
            }
            AppMode::SampleBrowser => self.move_sample_browser_cursor(delta),
            AppMode::ProjectBrowser => self.move_project_browser_cursor(delta),
            AppMode::Sampler => self.pan_sample_waveform(delta.signum()),
            AppMode::DspRack => self.move_dsp_rack_cursor(delta),
            AppMode::CommandPalette => self.move_command_palette_selection(delta),
            AppMode::Normal | AppMode::Edit => {
                self.cursor.row = self
                    .cursor
                    .row
                    .saturating_add_signed(delta)
                    .min(self.current_row_count().saturating_sub(1));
            }
            _ => {}
        }
    }

    pub(crate) fn handle_mouse(&mut self, mouse: MouseEvent, viewport: MouseViewport) {
        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                self.handle_mouse_wheel(mouse.kind)
            }
            MouseEventKind::ScrollLeft => self.handle_mouse_horizontal_scroll(-1),
            MouseEventKind::ScrollRight => self.handle_mouse_horizontal_scroll(1),
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_mouse_click(mouse.column, mouse.row, viewport, false, true)
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.handle_mouse_click(mouse.column, mouse.row, viewport, false, false)
            }
            MouseEventKind::Down(MouseButton::Right) => {
                self.handle_mouse_click(mouse.column, mouse.row, viewport, true, false)
            }
            _ => {}
        }
    }

    fn handle_mouse_horizontal_scroll(&mut self, delta: isize) {
        match self.mode {
            AppMode::Normal | AppMode::Edit => {
                self.cursor.track = self
                    .cursor
                    .track
                    .saturating_add_signed(delta)
                    .min(self.song.tracks.len().saturating_sub(1));
            }
            AppMode::Sampler => self.pan_sample_waveform(delta),
            _ => {}
        }
    }

    fn handle_mouse_click(
        &mut self,
        column: u16,
        row: u16,
        viewport: MouseViewport,
        activate: bool,
        primary_click: bool,
    ) {
        if let Some(region) = self.interaction_map.hit_test(column, row) {
            tracing::trace!(
                region = region.id.as_str(),
                column,
                row,
                "mouse interaction region"
            );
        }
        if row < 3 {
            if column < 12 {
                self.toggle_playback();
            }
            return;
        }

        if matches!(self.mode, AppMode::Normal | AppMode::Edit)
            && self.handle_renoise_sidebar_mouse_click(column, row, viewport)
        {
            return;
        }

        match self.mode {
            AppMode::Normal | AppMode::Edit => {
                let handled_composite = primary_click
                    && (self.handle_composite_track_mouse_click(column, row)
                        || self.handle_composite_sequence_mouse_click(column, row));
                if !handled_composite {
                    self.handle_tracker_mouse_click(column, row);
                }
            }
            AppMode::Tracks => self.handle_track_list_mouse_click(row),
            AppMode::SampleBrowser => {
                self.handle_sample_browser_mouse_click(column, row, activate);
            }
            AppMode::ProjectBrowser => {
                self.handle_project_browser_mouse_click(column, row, activate);
            }
            AppMode::Sampler => self.handle_sampler_mouse_click(column, row),
            AppMode::DspRack if self.dsp_device_palette_open => {
                self.handle_dsp_palette_mouse_click(row);
            }
            AppMode::DspRack => self.handle_dsp_rack_mouse_click(row, activate),
            _ => {}
        }
    }

    fn handle_renoise_sidebar_mouse_click(
        &mut self,
        column: u16,
        row: u16,
        viewport: MouseViewport,
    ) -> bool {
        let Some(sidebar) =
            renoise_pattern_sidebar_area(viewport.terminal_width, viewport.terminal_height)
        else {
            return false;
        };
        if !sidebar.contains(column, row) {
            return false;
        }
        let local_column = column.saturating_sub(sidebar.x);
        let local_row = row.saturating_sub(sidebar.y);

        if local_row == 1 {
            self.handle_renoise_sidebar_tab_click(local_column);
            return true;
        }

        let content_row = local_row.saturating_sub(1);
        let samples_header = 11 + usize::from(self.active_cell_has_data());
        let sample_count = self.song.samples.len().min(8);
        let sample_start = samples_header + 1;
        let songs_header = sample_start + sample_count;
        let pattern_start = songs_header + 1;
        let row_index = usize::from(content_row);

        if row_index == samples_header {
            self.open_sample_browser_view(None);
            return true;
        }
        if (sample_start..songs_header).contains(&row_index) {
            let sample_index = row_index - sample_start;
            self.select_track_for_sidebar_sample(sample_index);
            return true;
        }
        if row_index == songs_header {
            self.open_patterns_view();
            return true;
        }
        if (pattern_start..pattern_start + self.song.patterns.len().min(8)).contains(&row_index) {
            self.select_pattern(row_index - pattern_start);
            self.open_tracker_view();
            return true;
        }
        true
    }

    fn handle_renoise_sidebar_tab_click(&mut self, local_column: u16) {
        match local_column {
            1..=7 => self.open_tracker_view(),
            8..=15 => self.open_tracks_view(),
            16..=25 => self.open_sample_browser_view(None),
            26..=34 => self.notify_info("Other instrument properties are not implemented yet"),
            _ => {}
        }
    }

    fn active_cell_has_data(&self) -> bool {
        self.song
            .pattern(self.pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .is_some_and(pattern_cell_has_data)
    }

    fn select_track_for_sidebar_sample(&mut self, sample_index: usize) {
        let Some(sample) = self.song.samples.get(sample_index) else {
            return;
        };
        let sample_name = sample.name.clone();
        let Some(track_index) = self
            .song
            .sample_assignments
            .iter()
            .find(|assignment| assignment.sample == sample.id)
            .and_then(|assignment| {
                self.song
                    .tracks
                    .iter()
                    .position(|track| track.id == assignment.track)
            })
        else {
            self.notify_info(format!("Sample {sample_name}"));
            return;
        };
        self.cursor.track = track_index;
        self.focus_panel(FocusPanel::Tracker);
        self.notify_info(format!("Track {:02}: {sample_name}", track_index + 1));
    }

    fn handle_tracker_mouse_click(&mut self, column: u16, row: u16) {
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::PATTERN_CELL)
            .map(|region| region.payload);
        let Some(InteractionPayload::PatternCell { row, track }) = target else {
            return;
        };
        if row >= self.current_row_count() || track >= self.song.tracks.len() {
            return;
        }
        self.focus_panel(FocusPanel::Tracker);
        self.cursor.row = row;
        self.cursor.track = track;
        self.cursor.digit = 0;
    }

    fn handle_composite_track_mouse_click(&mut self, column: u16, row: u16) -> bool {
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::COMPOSITE_TRACK_ROW)
            .map(|region| region.payload);
        let Some(InteractionPayload::CompositeTrackRow { track }) = target else {
            return false;
        };
        if track < self.song.tracks.len() {
            self.cursor.track = track;
        }
        true
    }

    fn handle_composite_sequence_mouse_click(&mut self, column: u16, row: u16) -> bool {
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::COMPOSITE_SEQUENCE_ROW)
            .map(|region| region.payload);
        let Some(InteractionPayload::CompositeSequenceRow { position }) = target else {
            return false;
        };
        if position < self.song.sequence.len() {
            self.sequence_cursor = position;
            self.notify_info(format!("Sequence position {position:02}"));
        }
        true
    }

    fn handle_track_list_mouse_click(&mut self, row: u16) {
        const LIST_FIRST_ROW: u16 = 4;
        if row < LIST_FIRST_ROW || self.song.tracks.is_empty() {
            return;
        }
        self.cursor.track =
            usize::from(row - LIST_FIRST_ROW).min(self.song.tracks.len().saturating_sub(1));
        self.focus_panel(FocusPanel::Tracks);
    }

    fn handle_sample_browser_mouse_click(&mut self, column: u16, row: u16, activate: bool) {
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::SAMPLE_BROWSER_ENTRY)
            .map(|region| region.payload);
        let Some(InteractionPayload::SampleBrowserEntry { index }) = target else {
            return;
        };
        if let Some(browser) = &mut self.sample_browser_view {
            if index >= browser.entries.len() {
                return;
            }
            browser.cursor = index;
        } else {
            return;
        }
        self.update_sample_browser_preview();
        if activate {
            self.assign_selected_sample_browser_entry();
        }
    }

    fn handle_project_browser_mouse_click(&mut self, column: u16, row: u16, activate: bool) {
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::PROJECT_BROWSER_ENTRY)
            .map(|region| region.payload);
        let Some(InteractionPayload::ProjectBrowserEntry { index }) = target else {
            return;
        };
        if let Some(browser) = &mut self.project_browser_view {
            if index >= browser.entries.len() {
                return;
            }
            browser.cursor = index;
        } else {
            return;
        }
        if activate {
            self.select_project_browser_entry();
        }
    }

    fn handle_sampler_mouse_click(&mut self, column: u16, row: u16) {
        if row == 3 && (15..=25).contains(&column) {
            self.focus_panel(FocusPanel::Sampler);
        } else if column <= 28 {
            self.open_sample_browser_view(None);
        }
    }

    pub(crate) fn handle_control_key(&mut self, key: KeyEvent) -> bool {
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }

        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.start_save_as_command();
                true
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.save();
                true
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('\n') => {
                self.open_sampler_view();
                true
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.create_track();
                true
            }
            KeyCode::Up => {
                self.dispatch_intent(AppIntent::Parameter(ParameterIntent::AdjustBpm(1)));
                true
            }
            KeyCode::Down => {
                self.dispatch_intent(AppIntent::Parameter(ParameterIntent::AdjustBpm(-1)));
                true
            }
            KeyCode::Right => {
                self.dispatch_intent(AppIntent::Parameter(ParameterIntent::AdjustLinesPerBeat(1)));
                true
            }
            KeyCode::Left => {
                self.dispatch_intent(AppIntent::Parameter(ParameterIntent::AdjustLinesPerBeat(
                    -1,
                )));
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if self.mode == AppMode::Ai {
                    self.cancel_active_task();
                    return true;
                }
                self.copy_selection_or_current_cell();
                true
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.cut_selection_or_current_cell();
                true
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.paste_clipboard();
                true
            }
            KeyCode::Delete => {
                self.delete_current_row();
                true
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.redo();
                } else {
                    self.undo();
                }
                true
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.redo();
                true
            }
            KeyCode::Char('p') | KeyCode::Char('P')
                if !key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.open_command_palette();
                true
            }
            KeyCode::Char('p') | KeyCode::Char('P')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.panic_midi();
                true
            }
            _ => true,
        }
    }

    pub(crate) fn handle_ai_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') if self.ai_thread.composer.is_empty() => self.request_quit(false),
            KeyCode::Char('a' | 'A')
                if self.ai_thread.composer.is_empty() && self.pending_ai_proposal.is_some() =>
            {
                self.apply_ai_proposal();
            }
            KeyCode::Char('r' | 'R')
                if self.ai_thread.composer.is_empty() && self.pending_ai_proposal.is_some() =>
            {
                self.reject_ai_proposal();
            }
            KeyCode::Char('p' | 'P')
                if self.ai_thread.composer.is_empty() && self.pending_ai_proposal.is_some() =>
            {
                self.show_ai_proposal();
            }
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => self.open_command_prompt(),
            KeyCode::Enter => self.submit_ai_chat_prompt(),
            KeyCode::Backspace => {
                self.ai_thread.composer.pop();
            }
            KeyCode::Char(ch) => self.ai_thread.composer.push(ch),
            _ => {}
        }
    }

    pub(crate) fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.pending_goto_start {
            self.pending_goto_start = false;
            if self.vim_navigation && key.code == KeyCode::Char('g') {
                self.cursor.row = 0;
                return;
            }
        }

        let direction = match key.code {
            KeyCode::Esc => {
                self.selection = None;
                return;
            }
            KeyCode::Char('q') => {
                self.request_quit(false);
                return;
            }
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.start_playback();
                return;
            }
            KeyCode::Char(' ') => {
                self.toggle_playback();
                return;
            }
            KeyCode::F(8) => {
                self.stop_playback();
                return;
            }
            KeyCode::F(1) => {
                self.decrement_octave();
                return;
            }
            KeyCode::F(2) => {
                self.increment_octave();
                return;
            }
            KeyCode::F(3) => {
                self.start_pattern_rename_command();
                return;
            }
            KeyCode::F(4) => {
                self.open_midi_settings();
                return;
            }
            KeyCode::F(7) => {
                self.open_sequence_view();
                return;
            }
            KeyCode::F(9) => {
                self.open_tracks_view();
                return;
            }
            KeyCode::F(10) => {
                self.open_patterns_view();
                return;
            }
            KeyCode::F(6) => {
                self.start_pattern_length_command();
                return;
            }
            KeyCode::Char('r') => {
                self.start_track_rename_command();
                return;
            }
            KeyCode::Char('c') => {
                self.start_track_channel_command();
                return;
            }
            KeyCode::Char('D') => {
                self.duplicate_track(self.cursor.track);
                return;
            }
            KeyCode::Char('{') => {
                self.move_current_track_left();
                return;
            }
            KeyCode::Char('}') => {
                self.move_current_track_right();
                return;
            }
            KeyCode::Char('N') => {
                self.create_pattern();
                return;
            }
            KeyCode::Char('P') => {
                self.duplicate_current_pattern();
                return;
            }
            KeyCode::Char('X') => {
                self.request_delete_current_pattern();
                return;
            }
            KeyCode::Char('A') => {
                self.add_sequence_pattern(self.pattern_index);
                return;
            }
            KeyCode::Char(',') => {
                self.previous_sequence_position();
                return;
            }
            KeyCode::Char('.') => {
                self.next_sequence_position();
                return;
            }
            KeyCode::Char('Y') => {
                self.duplicate_selected_sequence_position();
                return;
            }
            KeyCode::Char('R') => {
                self.remove_selected_sequence_position();
                return;
            }
            KeyCode::Char('T') => {
                self.set_selected_sequence_to_current_pattern();
                return;
            }
            KeyCode::Char('<') => {
                self.move_selected_sequence_position_up();
                return;
            }
            KeyCode::Char('>') => {
                self.move_selected_sequence_position_down();
                return;
            }
            KeyCode::Char('L') => {
                self.toggle_loop();
                return;
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.start_sequence_playback_from_selected_position();
                return;
            }
            KeyCode::Enter => {
                self.start_playback_from_cursor();
                return;
            }
            KeyCode::Char('i') => {
                self.selection = None;
                self.mode = AppMode::Edit;
                return;
            }
            KeyCode::Char(':') => {
                self.open_command_prompt();
                return;
            }
            KeyCode::Char('?') | KeyCode::Char('H') => {
                self.open_help();
                return;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.start_selection();
                return;
            }
            KeyCode::Char('[') => {
                self.select_pattern(self.pattern_index.saturating_sub(1));
                return;
            }
            KeyCode::Char(']') => {
                self.select_pattern(self.pattern_index.saturating_add(1));
                return;
            }
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Char('k') if self.vim_navigation => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Char('j') if self.vim_navigation => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Char('h') if self.vim_navigation => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            KeyCode::Char('l') if self.vim_navigation => Some(Direction::Right),
            KeyCode::Tab => {
                self.next_track();
                return;
            }
            KeyCode::BackTab => {
                self.previous_track();
                return;
            }
            KeyCode::Home => {
                self.cursor.row = 0;
                return;
            }
            KeyCode::End => {
                self.cursor.row = self.current_row_count().saturating_sub(1);
                return;
            }
            KeyCode::Char('g') if self.vim_navigation => {
                self.pending_goto_start = true;
                return;
            }
            KeyCode::Char('G') if self.vim_navigation => {
                self.cursor.row = self.current_row_count().saturating_sub(1);
                return;
            }
            KeyCode::PageUp => {
                self.page_cursor_up();
                return;
            }
            KeyCode::PageDown => {
                self.page_cursor_down();
                return;
            }
            KeyCode::Insert => {
                self.insert_current_row();
                return;
            }
            KeyCode::Delete => {
                if self.selection.is_some() {
                    self.clear_selection_region();
                } else {
                    self.request_delete_current_track();
                }
                return;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.toggle_current_mute();
                return;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.toggle_current_solo();
                return;
            }
            _ => None,
        };

        if let Some(direction) = direction {
            self.dispatch_intent(AppIntent::Navigation(NavigationIntent::MoveCursor(
                direction,
            )));
        }
    }

    pub(crate) fn handle_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Up => self.dispatch_intent(AppIntent::Navigation(
                NavigationIntent::MoveCursor(Direction::Up),
            )),
            KeyCode::Down => self.dispatch_intent(AppIntent::Navigation(
                NavigationIntent::MoveCursor(Direction::Down),
            )),
            KeyCode::Left => self.dispatch_intent(AppIntent::Navigation(
                NavigationIntent::MoveCursor(Direction::Left),
            )),
            KeyCode::Right => self.dispatch_intent(AppIntent::Navigation(
                NavigationIntent::MoveCursor(Direction::Right),
            )),
            KeyCode::Tab => {
                self.dispatch_intent(AppIntent::Navigation(NavigationIntent::NextTrack))
            }
            KeyCode::BackTab => {
                self.dispatch_intent(AppIntent::Navigation(NavigationIntent::PreviousTrack))
            }
            KeyCode::Home => self.cursor.row = 0,
            KeyCode::End => self.cursor.row = self.current_row_count().saturating_sub(1),
            KeyCode::PageUp => {
                self.dispatch_intent(AppIntent::Navigation(NavigationIntent::PageUp))
            }
            KeyCode::PageDown => {
                self.dispatch_intent(AppIntent::Navigation(NavigationIntent::PageDown))
            }
            KeyCode::Insert => self.insert_current_row(),
            KeyCode::Delete | KeyCode::Backspace => {
                self.dispatch_intent(AppIntent::Tracker(TrackerIntent::ClearCell))
            }
            KeyCode::F(1) | KeyCode::Char('-') => self.decrement_octave(),
            KeyCode::F(2) | KeyCode::Char('+') | KeyCode::Char('=') => self.increment_octave(),
            KeyCode::Char('o') | KeyCode::Char('O') => self.dispatch_intent(AppIntent::Tracker(
                TrackerIntent::InsertNoteEvent(NoteEvent::NoteOff),
            )),
            KeyCode::Char('.') => self.dispatch_intent(AppIntent::Tracker(
                TrackerIntent::InsertNoteEvent(NoteEvent::NoteCut),
            )),
            KeyCode::Char(value) if self.cursor.field != CellField::Note => {
                if let Some(hex) = value.to_digit(16) {
                    self.dispatch_intent(AppIntent::Tracker(TrackerIntent::EnterHexDigit(
                        hex as u8,
                    )));
                }
            }
            KeyCode::Char(value) => {
                if let Some(note) = keyboard_note(value, self.octave) {
                    self.dispatch_intent(AppIntent::Tracker(TrackerIntent::InsertNote(note)));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.close_focus_capture();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.help_scroll = self.help_scroll.saturating_add(1);
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.help_tab = self.help_tab.next();
                self.help_scroll = 0;
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                self.help_tab = self.help_tab.previous();
                self.help_scroll = 0;
            }
            KeyCode::PageUp => {
                self.help_scroll = self.help_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.help_scroll = self.help_scroll.saturating_add(10);
            }
            KeyCode::Home => {
                self.help_scroll = 0;
            }
            KeyCode::End => {
                self.help_scroll = usize::MAX;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MouseRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl MouseRect {
    fn contains(self, column: u16, row: u16) -> bool {
        (self.x..self.x.saturating_add(self.width)).contains(&column)
            && (self.y..self.y.saturating_add(self.height)).contains(&row)
    }
}

fn renoise_pattern_sidebar_area(terminal_width: u16, terminal_height: u16) -> Option<MouseRect> {
    const LARGE_MIN_WIDTH: u16 = 120;
    const RIGHT_WIDTH: u16 = 38;
    const HEADER_HEIGHT: u16 = 3;
    const STATUS_HEIGHT: u16 = 1;
    const COMPACT_BODY_HEIGHT: u16 = 34;

    if terminal_width < LARGE_MIN_WIDTH || terminal_width < RIGHT_WIDTH {
        return None;
    }

    let body_height = terminal_height.saturating_sub(HEADER_HEIGHT + STATUS_HEIGHT);
    let compact_height = body_height < COMPACT_BODY_HEIGHT;
    let top_height = if compact_height { 4 } else { 6 };
    let bottom_height = if compact_height { 7 } else { 10 };
    let height = body_height
        .saturating_sub(top_height)
        .saturating_sub(bottom_height);
    (height > 0).then_some(MouseRect {
        x: terminal_width - RIGHT_WIDTH,
        y: HEADER_HEIGHT + top_height,
        width: RIGHT_WIDTH,
        height,
    })
}

fn pattern_cell_has_data(cell: &PatternCell) -> bool {
    cell.note.is_some()
        || cell.velocity.is_some()
        || cell.instrument.is_some()
        || cell.volume.is_some()
        || cell.pan.is_some()
        || cell.delay.is_some()
        || cell.gate.is_some()
        || cell.command.is_some()
        || cell.command2.is_some()
        || !cell.parameter_locks.is_empty()
}
