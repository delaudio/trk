use super::*;
use crate::command::ScaleCommand;

const LOWER_NOTE_KEYS: &str = "zsxdcvgbhnjm";
const UPPER_NOTE_KEYS: &str = "q2w3er5t6y7u";

impl App {
    pub(crate) fn handle_scale_lock_shortcut(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.mode, AppMode::Normal | AppMode::Edit)
            || key.code != KeyCode::Char('K')
            || (key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT)
        {
            return false;
        }
        self.toggle_scale_lock();
        true
    }

    pub(crate) fn apply_scale_command(&mut self, command: ScaleCommand) {
        match command {
            ScaleCommand::Status => self.report_scale_lock(),
            ScaleCommand::On => self.set_scale_lock_enabled(true),
            ScaleCommand::Off => self.set_scale_lock_enabled(false),
            ScaleCommand::Toggle => self.toggle_scale_lock(),
            ScaleCommand::Select(scale) => {
                self.scale_lock.scale = scale;
                self.scale_lock.enabled = true;
                self.notify_success(format!("Scale Lock ON ({})", scale.label()));
            }
        }
    }

    pub(crate) fn toggle_scale_lock(&mut self) {
        self.set_scale_lock_enabled(!self.scale_lock.enabled);
    }

    fn set_scale_lock_enabled(&mut self, enabled: bool) {
        self.scale_lock.enabled = enabled;
        self.report_scale_lock();
    }

    fn report_scale_lock(&mut self) {
        let state = if self.scale_lock.enabled { "ON" } else { "OFF" };
        self.notify_info(format!(
            "Scale Lock {state} ({})",
            self.scale_lock.scale.label()
        ));
    }

    pub(crate) fn keyboard_note_for_entry(&self, key: char) -> Option<u8> {
        if !self.scale_lock.enabled {
            return keyboard_note(key, self.octave);
        }
        let key = key.to_ascii_lowercase();
        let degree = LOWER_NOTE_KEYS
            .chars()
            .position(|candidate| candidate == key)
            .or_else(|| {
                UPPER_NOTE_KEYS
                    .chars()
                    .position(|candidate| candidate == key)
                    .map(|index| self.scale_lock.scale.intervals().len() + index)
            })?;
        self.scale_lock.scale.degree_pitch(self.octave, degree)
    }

    pub(crate) fn current_chord_name(&self) -> Option<String> {
        if !self.is_playing {
            return None;
        }
        let row = self.playhead_row?;
        let sequence_pattern_id = self
            .sequence_position
            .and_then(|position| self.song.sequence.get(position))
            .copied();
        let pattern = sequence_pattern_id
            .and_then(|pattern_id| {
                self.song
                    .patterns
                    .iter()
                    .find(|pattern| pattern.id == pattern_id)
            })
            .or_else(|| self.song.pattern(self.pattern_index))?;
        let pitches = active_pitches_at_row(&self.song, pattern, row);
        identify_chord(&pitches).map(|chord| chord.to_string())
    }

    pub(crate) fn tui_harmonic_mode_label(&self, active_view: TuiView) -> String {
        let mut label = self.mode.label().to_string();
        if active_view != TuiView::Pattern || !matches!(self.mode, AppMode::Normal | AppMode::Edit)
        {
            return label;
        }
        if self.scale_lock.enabled {
            label.push_str(" K:");
            label.push_str(&self.scale_lock.scale.short_label());
        }
        if let Some(chord) = self.current_chord_name() {
            label.push(' ');
            label.push_str(&chord);
        }
        label
    }
}
