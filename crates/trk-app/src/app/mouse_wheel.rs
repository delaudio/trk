use super::*;

impl App {
    pub(super) fn handle_mouse_vertical_scroll(
        &mut self,
        column: u16,
        row: u16,
        kind: MouseEventKind,
    ) {
        if self.mode == AppMode::CommandPalette {
            self.handle_command_palette_mouse_wheel(column, row, kind);
            return;
        }
        if self.mode == AppMode::Help {
            self.handle_help_mouse_wheel(column, row, kind);
            return;
        }
        if matches!(self.mode, AppMode::Command | AppMode::Dialog) {
            return;
        }

        let delta = match kind {
            MouseEventKind::ScrollUp => -3,
            MouseEventKind::ScrollDown => 3,
            _ => return,
        };

        let target = self.interaction_map.scroll_target_at(column, row);
        if self.mode == AppMode::DspRack && self.dsp_device_palette_open {
            if target == Some(ScrollTarget::DspPalette) {
                self.move_dsp_device_palette_cursor(delta);
            }
            return;
        }
        if self.mode == AppMode::MidiSettings {
            if target == Some(ScrollTarget::MidiPorts) {
                self.midi_port_cursor = self
                    .midi_port_cursor
                    .saturating_add_signed(delta)
                    .min(self.midi_ports.len().saturating_sub(1));
            }
            return;
        }

        match target {
            Some(ScrollTarget::PatternRows) => {
                self.cursor.row = self
                    .cursor
                    .row
                    .saturating_add_signed(delta)
                    .min(self.current_row_count().saturating_sub(1));
            }
            Some(ScrollTarget::Tracks) if !self.song.tracks.is_empty() => {
                self.cursor.track = self
                    .cursor
                    .track
                    .saturating_add_signed(delta)
                    .min(self.song.tracks.len().saturating_sub(1));
                self.cursor.digit = 0;
            }
            Some(ScrollTarget::Sequence) if !self.song.sequence.is_empty() => {
                self.sequence_cursor = self
                    .sequence_cursor
                    .saturating_add_signed(delta)
                    .min(self.song.sequence.len().saturating_sub(1));
            }
            Some(ScrollTarget::Clips) if !self.song.clip_scenes.is_empty() => {
                self.clip_scene_cursor = self
                    .clip_scene_cursor
                    .saturating_add_signed(delta)
                    .min(self.song.clip_scenes.len().saturating_sub(1));
            }
            Some(ScrollTarget::Patterns) if !self.song.patterns.is_empty() => {
                let pattern = self
                    .pattern_index
                    .saturating_add_signed(delta)
                    .min(self.song.patterns.len().saturating_sub(1));
                self.select_pattern(pattern);
            }
            Some(ScrollTarget::SampleBrowser) => self.move_sample_browser_cursor(delta),
            Some(ScrollTarget::ProjectBrowser) => self.move_project_browser_cursor(delta),
            Some(ScrollTarget::SamplerWaveform) => self.pan_sample_waveform(delta.signum()),
            Some(ScrollTarget::DspDevices { target }) => {
                self.move_hovered_dsp_devices(target, delta);
            }
            Some(ScrollTarget::DspParameters) => self.move_dsp_parameter_cursor(delta),
            _ => {}
        }
    }

    pub(super) fn handle_mouse_horizontal_scroll(&mut self, column: u16, row: u16, delta: isize) {
        if matches!(
            self.mode,
            AppMode::Command
                | AppMode::CommandPalette
                | AppMode::Help
                | AppMode::MidiSettings
                | AppMode::Dialog
        ) || (self.mode == AppMode::DspRack && self.dsp_device_palette_open)
        {
            return;
        }

        match self.interaction_map.scroll_target_at(column, row) {
            Some(ScrollTarget::PatternRows) if !self.song.tracks.is_empty() => {
                self.cursor.track = self
                    .cursor
                    .track
                    .saturating_add_signed(delta)
                    .min(self.song.tracks.len().saturating_sub(1));
                self.cursor.digit = 0;
            }
            Some(ScrollTarget::Clips) if !self.song.tracks.is_empty() => {
                self.clip_track_cursor = self
                    .clip_track_cursor
                    .saturating_add_signed(delta)
                    .min(self.song.tracks.len().saturating_sub(1));
            }
            Some(ScrollTarget::SamplerWaveform) => self.pan_sample_waveform(delta),
            _ => {}
        }
    }
}
