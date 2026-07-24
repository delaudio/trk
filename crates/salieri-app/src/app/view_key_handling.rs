use super::*;

impl App {
    pub(crate) fn handle_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => match self.dialog.clone() {
                Some(Dialog::QuitDirty) => {
                    self.save_and_quit();
                }
                Some(Dialog::DeleteTrack { track_index, .. }) => {
                    self.dialog = None;
                    self.close_focus_capture();
                    self.delete_track(track_index);
                }
                Some(Dialog::DeletePattern { pattern_index, .. }) => {
                    self.dialog = None;
                    self.close_focus_capture();
                    self.delete_pattern(pattern_index);
                }
                Some(Dialog::OpenProjectDirty { path, .. }) => {
                    self.dialog = None;
                    self.open_project_file(path);
                }
                None => self.close_focus_capture(),
            },
            KeyCode::Char('n') | KeyCode::Char('N') => match self.dialog {
                Some(Dialog::QuitDirty) => self.force_quit(),
                Some(
                    Dialog::DeleteTrack { .. }
                    | Dialog::DeletePattern { .. }
                    | Dialog::OpenProjectDirty { .. },
                )
                | None => self.cancel_dialog(),
            },
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                self.cancel_dialog();
            }
            _ => {}
        }
    }

    pub(crate) fn handle_midi_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.restore_previous_focus(),
            KeyCode::Up => self.previous_midi_port(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_midi_port(),
            KeyCode::Down => self.next_midi_port(),
            KeyCode::Char('j') if self.vim_navigation => self.next_midi_port(),
            KeyCode::Home => self.midi_port_cursor = 0,
            KeyCode::End => {
                self.midi_port_cursor = self.midi_ports.len().saturating_sub(1);
            }
            KeyCode::Enter => self.connect_selected_midi_port(),
            KeyCode::Char('d') | KeyCode::Char('D') => self.disconnect_midi(),
            KeyCode::Char('p') | KeyCode::Char('P') => self.panic_midi(),
            KeyCode::F(5) | KeyCode::Char('r') | KeyCode::Char('R') => self.refresh_midi_ports(),
            _ => {}
        }
    }

    pub(crate) fn handle_sequence_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => {
                self.open_command_prompt();
            }
            KeyCode::Char(' ') => self.toggle_playback(),
            KeyCode::F(8) => self.stop_playback(),
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(7) => self.open_sequence_view(),
            KeyCode::Up => self.previous_sequence_position(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_sequence_position(),
            KeyCode::Down => self.next_sequence_position(),
            KeyCode::Char('j') if self.vim_navigation => self.next_sequence_position(),
            KeyCode::Home => {
                self.sequence_cursor = 0;
                self.notify_info("Sequence position 00");
            }
            KeyCode::End => {
                self.sequence_cursor = self.song.sequence.len().saturating_sub(1);
                self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
            }
            KeyCode::Char('A') => self.add_sequence_pattern(self.pattern_index),
            KeyCode::Char('Y') => self.duplicate_selected_sequence_position(),
            KeyCode::Char('R') => self.remove_selected_sequence_position(),
            KeyCode::Char('T') => self.set_selected_sequence_to_current_pattern(),
            KeyCode::Char('<') => self.move_selected_sequence_position_up(),
            KeyCode::Char('>') => self.move_selected_sequence_position_down(),
            KeyCode::Enter => self.start_sequence_playback_from_selected_position(),
            _ => {}
        }
    }

    pub(crate) fn handle_clip_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => self.open_command_prompt(),
            KeyCode::F(8) => self.stop_clip_launcher(),
            KeyCode::Up => self.previous_clip_scene(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_clip_scene(),
            KeyCode::Down => self.next_clip_scene(),
            KeyCode::Char('j') if self.vim_navigation => self.next_clip_scene(),
            KeyCode::Left => self.previous_clip_track(),
            KeyCode::Char('h') if self.vim_navigation => self.previous_clip_track(),
            KeyCode::Right => self.next_clip_track(),
            KeyCode::Char('l') if self.vim_navigation => self.next_clip_track(),
            KeyCode::Char('A') => self.add_clip_scene_from_current_pattern(),
            KeyCode::Char('T') => self.set_selected_clip_to_current_pattern(),
            KeyCode::Char('R') => self.clear_selected_clip(),
            KeyCode::Enter => self.queue_selected_clip_scene(),
            KeyCode::Char(' ') => self.launch_queued_clip_scene(),
            _ => {}
        }
    }

    pub(crate) fn handle_tracks_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => {
                self.open_command_prompt();
            }
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(9) => self.open_tracks_view(),
            KeyCode::Up => self.previous_track(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_track(),
            KeyCode::Down => self.next_track(),
            KeyCode::Char('j') if self.vim_navigation => self.next_track(),
            KeyCode::Home => self.cursor.track = 0,
            KeyCode::End => self.cursor.track = self.song.tracks.len().saturating_sub(1),
            KeyCode::Char('N') => self.create_track(),
            KeyCode::Char('D') => self.duplicate_track(self.cursor.track),
            KeyCode::Char('r') => self.start_track_rename_command(),
            KeyCode::Char('c') => self.start_track_channel_command(),
            KeyCode::Delete => self.request_delete_current_track(),
            KeyCode::Char('{') => self.move_current_track_left(),
            KeyCode::Char('}') => self.move_current_track_right(),
            KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_current_mute(),
            KeyCode::Char('s') | KeyCode::Char('S') => self.toggle_current_solo(),
            _ => {}
        }
    }

    pub(crate) fn handle_patterns_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => {
                self.open_command_prompt();
            }
            KeyCode::F(10) => self.open_patterns_view(),
            KeyCode::Up => self.select_pattern(self.pattern_index.saturating_sub(1)),
            KeyCode::Char('k') if self.vim_navigation => {
                self.select_pattern(self.pattern_index.saturating_sub(1));
            }
            KeyCode::Down => self.select_pattern(self.pattern_index.saturating_add(1)),
            KeyCode::Char('j') if self.vim_navigation => {
                self.select_pattern(self.pattern_index.saturating_add(1));
            }
            KeyCode::Home => self.select_pattern(0),
            KeyCode::End => self.select_pattern(self.song.patterns.len().saturating_sub(1)),
            KeyCode::Char('N') => self.create_pattern(),
            KeyCode::Char('P') => self.duplicate_current_pattern(),
            KeyCode::Char('X') | KeyCode::Delete => self.request_delete_current_pattern(),
            KeyCode::Char('r') => self.start_pattern_rename_command(),
            KeyCode::F(6) => self.start_pattern_length_command(),
            KeyCode::Char('1') => self.resize_current_pattern(16),
            KeyCode::Char('2') => self.resize_current_pattern(32),
            KeyCode::Char('3') => self.resize_current_pattern(64),
            KeyCode::Char('4') => self.resize_current_pattern(128),
            KeyCode::Char('5') => self.resize_current_pattern(256),
            KeyCode::Enter => self.open_tracker_view(),
            _ => {}
        }
    }

    pub(crate) fn handle_sampler_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => {
                self.open_command_prompt();
            }
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(7) => self.open_sequence_view(),
            KeyCode::F(9) => self.open_tracks_view(),
            KeyCode::F(10) => self.open_patterns_view(),
            KeyCode::F(8) => self.stop_playback(),
            KeyCode::Char('b') | KeyCode::Char('B') => self.open_sample_browser_view(None),
            KeyCode::Tab => self.next_sampler_envelope_field(),
            KeyCode::BackTab => self.previous_sampler_envelope_field(),
            KeyCode::Char('[') => self.adjust_selected_sampler_envelope(-1.0, false),
            KeyCode::Char(']') => self.adjust_selected_sampler_envelope(1.0, false),
            KeyCode::Char('{') => self.adjust_selected_sampler_envelope(-1.0, true),
            KeyCode::Char('}') => self.adjust_selected_sampler_envelope(1.0, true),
            KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_sample_waveform_in(),
            KeyCode::Char('-') => self.zoom_sample_waveform_out(),
            KeyCode::Left | KeyCode::Char('h') => self.pan_sample_waveform(-1),
            KeyCode::Right | KeyCode::Char('l') => self.pan_sample_waveform(1),
            KeyCode::Home => self.jump_sample_waveform_start(),
            KeyCode::End => self.jump_sample_waveform_end(),
            KeyCode::Char(' ') => self.toggle_playback(),
            _ => {}
        }
    }

    pub(crate) fn handle_dsp_rack_key(&mut self, key: KeyEvent) {
        if self.dsp_device_palette_open {
            match key.code {
                KeyCode::Esc => self.close_dsp_device_palette(),
                KeyCode::Char('q') => self.request_quit(false),
                KeyCode::Char(':') => self.open_command_prompt(),
                KeyCode::Tab | KeyCode::BackTab => self.toggle_dsp_rack_target(),
                KeyCode::Up => self.move_dsp_device_palette_cursor(-1),
                KeyCode::Char('k') if self.vim_navigation => {
                    self.move_dsp_device_palette_cursor(-1)
                }
                KeyCode::Down => self.move_dsp_device_palette_cursor(1),
                KeyCode::Char('j') if self.vim_navigation => self.move_dsp_device_palette_cursor(1),
                KeyCode::Home => self.dsp_device_palette_cursor = 0,
                KeyCode::End => self.dsp_device_palette_cursor = usize::MAX,
                KeyCode::Enter => self.assign_selected_dsp_device(),
                _ => {}
            }
            self.move_dsp_device_palette_cursor(0);
            return;
        }
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => self.open_command_prompt(),
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(7) => self.open_sequence_view(),
            KeyCode::F(9) => self.open_tracks_view(),
            KeyCode::F(10) => self.open_patterns_view(),
            KeyCode::F(8) => self.stop_playback(),
            KeyCode::Tab | KeyCode::BackTab => self.toggle_dsp_rack_target(),
            KeyCode::Char('a') | KeyCode::Char('A') => self.open_dsp_device_palette(),
            KeyCode::Char('p') | KeyCode::Char('P') => self.set_selected_dsp_parameter_lock(),
            KeyCode::Char('r') | KeyCode::Char('R') => self.reset_selected_dsp_parameter_lock(),
            KeyCode::Char('c') | KeyCode::Char('C') => self.clear_selected_dsp_parameter_lock(),
            KeyCode::Up => self.move_dsp_rack_cursor(-1),
            KeyCode::Char('k') if self.vim_navigation => self.move_dsp_rack_cursor(-1),
            KeyCode::Down => self.move_dsp_rack_cursor(1),
            KeyCode::Char('j') if self.vim_navigation => self.move_dsp_rack_cursor(1),
            KeyCode::Char('[') => self.move_dsp_parameter_cursor(-1),
            KeyCode::Char(']') => self.move_dsp_parameter_cursor(1),
            KeyCode::Left => self.adjust_selected_dsp_parameter(-1.0),
            KeyCode::Char('h') if self.vim_navigation => {
                self.adjust_selected_dsp_parameter(-1.0);
            }
            KeyCode::Right => self.adjust_selected_dsp_parameter(1.0),
            KeyCode::Char('l') if self.vim_navigation => {
                self.adjust_selected_dsp_parameter(1.0);
            }
            KeyCode::Home => self.dsp_rack_cursor = 0,
            KeyCode::End => {
                self.dsp_rack_cursor = usize::MAX;
                self.keep_dsp_rack_cursor_in_bounds();
            }
            KeyCode::Char(' ') => self.toggle_playback(),
            _ => {}
        }
    }

    pub(crate) fn handle_sample_browser_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_sampler_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => {
                self.open_command_prompt();
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_sample_browser_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_sample_browser_cursor(1),
            KeyCode::PageUp => self.move_sample_browser_cursor(-10),
            KeyCode::PageDown => self.move_sample_browser_cursor(10),
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => self.sample_browser_parent(),
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.assign_selected_sample_browser_entry();
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.select_sample_browser_entry()
            }
            _ => {}
        }
    }

    pub(crate) fn handle_project_browser_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.open_tracker_view(),
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.open_help(),
            KeyCode::Char(':') => {
                self.open_command_prompt();
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_project_browser_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_project_browser_cursor(1),
            KeyCode::PageUp => self.move_project_browser_cursor(-10),
            KeyCode::PageDown => self.move_project_browser_cursor(10),
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                self.project_browser_parent()
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.select_project_browser_entry()
            }
            KeyCode::Char('r') | KeyCode::Char('R') => self.refresh_project_browser_view(),
            _ => {}
        }
    }

    pub(crate) fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_buffer.clear();
                self.close_focus_capture();
            }
            KeyCode::Enter => self.execute_command(),
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(value) => self.command_buffer.push(value),
            _ => {}
        }
    }
}
