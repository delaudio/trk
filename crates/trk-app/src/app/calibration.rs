use super::*;

const CONTROL_COUNT: usize = 8;

impl App {
    pub(super) fn open_calibration(&mut self) {
        let target_track_id = self
            .song
            .tracks
            .get(self.cursor.track)
            .map(|track| track.id.0);
        let mut settings = self.playback.calibration_settings();
        settings.target_track_id = target_track_id;
        let _ = self.playback.set_calibration(settings);
        self.calibration_cursor = 0;
        self.calibration_open = true;
    }

    pub(super) fn handle_calibration_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('t') => self.calibration_open = false,
            KeyCode::Up | KeyCode::Char('k') => {
                self.calibration_cursor = self.calibration_cursor.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.calibration_cursor =
                    (self.calibration_cursor + 1).min(CONTROL_COUNT.saturating_sub(1));
            }
            KeyCode::Left | KeyCode::Char('-') | KeyCode::Char('h') => {
                self.adjust_calibration(-1.0);
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('l') => {
                self.adjust_calibration(1.0);
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.adjust_calibration(1.0),
            KeyCode::Char('r') => self.reset_calibration(),
            _ => {}
        }
    }

    fn adjust_calibration(&mut self, direction: f32) {
        let mut settings = self.playback.calibration_settings();
        match self.calibration_cursor {
            0 => settings.master_gain = gain_step(settings.master_gain, direction),
            1 => settings.track_gain = gain_step(settings.track_gain, direction),
            2 => settings.low_gain = gain_step(settings.low_gain, direction),
            3 => settings.mid_gain = gain_step(settings.mid_gain, direction),
            4 => settings.high_gain = gain_step(settings.high_gain, direction),
            5 => {
                settings.gate_threshold =
                    (settings.gate_threshold + direction * 0.01).clamp(0.0, 0.5);
            }
            6 => {
                settings.meter_decay = (settings.meter_decay + direction * 0.05).clamp(0.0, 0.95);
            }
            7 => settings.auto_gain = !settings.auto_gain,
            _ => return,
        }
        let _ = self.playback.set_calibration(settings);
    }

    fn reset_calibration(&mut self) {
        let target_track_id = self.playback.calibration_settings().target_track_id;
        let settings = CalibrationSettings {
            target_track_id,
            ..CalibrationSettings::default()
        };
        let _ = self.playback.set_calibration(settings);
    }

    pub(super) fn tui_calibration(&self) -> Option<CalibrationViewState<'_>> {
        if !self.calibration_open {
            return None;
        }
        let settings = self.playback.calibration_settings();
        let meters = self.playback.calibration_meters();
        let track_name = settings.target_track_id.and_then(|target| {
            self.song
                .tracks
                .iter()
                .find(|track| track.id.0 == target)
                .map(|track| track.name.as_str())
        });
        Some(CalibrationViewState {
            color_mode: self.terminal_color_mode,
            selected: self.calibration_cursor,
            track_name,
            master_gain: settings.master_gain,
            track_gain: settings.track_gain,
            low_gain: settings.low_gain,
            mid_gain: settings.mid_gain,
            high_gain: settings.high_gain,
            gate_threshold: settings.gate_threshold,
            meter_decay: settings.meter_decay,
            auto_gain: settings.auto_gain,
            meter_low: meters.low,
            meter_mid: meters.mid,
            meter_high: meters.high,
            meter_rms: meters.rms,
            meter_peak: meters.peak,
        })
    }
}

fn gain_step(value: f32, direction: f32) -> f32 {
    (value + direction * 0.1).clamp(0.1, 4.0)
}
