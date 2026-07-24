use super::*;

impl App {
    pub(crate) fn tui_dsp_rack_view(&self) -> DspRackViewState<'_> {
        let track = self.song.tracks.get(self.cursor.track);
        let track_effects = track
            .and_then(|track| {
                self.song
                    .mixer
                    .tracks
                    .iter()
                    .find(|mixer| mixer.track == track.id)
            })
            .map_or(&[][..], |mixer| mixer.effects.as_slice());
        DspRackViewState {
            track_name: track.map_or("<missing track>", |track| track.name.as_str()),
            track_number: self.cursor.track + 1,
            track_effects,
            master_effects: self.song.mixer.master_effects.as_slice(),
            selected_target: match self.dsp_rack_target {
                DspRackTarget::Track => DspRackTargetView::Track,
                DspRackTarget::Master => DspRackTargetView::Master,
            },
            selected_index: self.dsp_rack_cursor,
        }
    }

    pub(crate) fn keep_dsp_rack_cursor_in_bounds(&mut self) {
        let effect_count = self.selected_dsp_rack_effect_count();
        self.dsp_rack_cursor = self.dsp_rack_cursor.min(effect_count.saturating_sub(1));
    }

    pub(crate) fn move_dsp_rack_cursor(&mut self, delta: isize) {
        let effect_count = self.selected_dsp_rack_effect_count();
        if effect_count == 0 {
            self.dsp_rack_cursor = 0;
            return;
        }
        self.dsp_rack_cursor = self
            .dsp_rack_cursor
            .saturating_add_signed(delta)
            .min(effect_count.saturating_sub(1));
    }

    pub(crate) fn toggle_dsp_rack_target(&mut self) {
        self.dsp_rack_target = match self.dsp_rack_target {
            DspRackTarget::Track => DspRackTarget::Master,
            DspRackTarget::Master => DspRackTarget::Track,
        };
        self.keep_dsp_rack_cursor_in_bounds();
    }

    fn selected_dsp_rack_effect_count(&self) -> usize {
        match self.dsp_rack_target {
            DspRackTarget::Track => self.current_track_effect_count(),
            DspRackTarget::Master => self.song.mixer.master_effects.len(),
        }
    }

    fn current_track_effect_count(&self) -> usize {
        self.song
            .tracks
            .get(self.cursor.track)
            .and_then(|track| {
                self.song
                    .mixer
                    .tracks
                    .iter()
                    .find(|mixer| mixer.track == track.id)
            })
            .map_or(0, |mixer| mixer.effects.len())
    }
}
