use super::*;

impl App {
    pub(crate) fn move_hovered_dsp_devices(&mut self, target: DspRackChain, delta: isize) {
        self.dsp_rack_target = app_dsp_rack_target(target);
        self.keep_dsp_rack_cursor_in_bounds();
        self.move_dsp_rack_cursor(delta);
    }

    pub(crate) fn handle_dsp_rack_mouse_click(
        &mut self,
        column: u16,
        row: u16,
        activate: bool,
        primary_click: bool,
    ) {
        if self.handle_dsp_rack_selection_target(column, row, primary_click) {
            return;
        }
        let Some(region) = self.interaction_map.hit_test(column, row).copied() else {
            return;
        };
        match (region.id, region.payload) {
            (
                interaction_region::DSP_PARAMETER_ROW,
                InteractionPayload::DspParameterRow { index },
            ) if primary_click || activate => {
                if self.select_dsp_parameter(index) && activate {
                    self.adjust_selected_dsp_parameter(1.0);
                }
            }
            (interaction_region::DSP_PARAMETER_ROW, _) => {}
            _ => {}
        }
    }

    pub(crate) fn handle_dsp_palette_mouse_click(
        &mut self,
        column: u16,
        row: u16,
        primary_click: bool,
    ) {
        if !primary_click {
            return;
        }
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::DSP_PALETTE_ENTRY)
            .map(|region| region.payload);
        let Some(InteractionPayload::DspPaletteEntry { index }) = target else {
            return;
        };
        self.assign_dsp_palette_entry(index);
    }

    fn handle_dsp_rack_selection_target(
        &mut self,
        column: u16,
        row: u16,
        primary_click: bool,
    ) -> bool {
        let Some(region) = self.interaction_map.hit_test(column, row).copied() else {
            return false;
        };
        match (region.id, region.payload) {
            (interaction_region::DSP_RACK_TARGET, InteractionPayload::DspRackTarget { target })
                if primary_click =>
            {
                self.dsp_rack_target = app_dsp_rack_target(target);
                self.keep_dsp_rack_cursor_in_bounds();
                true
            }
            (
                interaction_region::DSP_DEVICE_ROW,
                InteractionPayload::DspDeviceRow { target, index },
            ) if primary_click && index < self.dsp_effect_count(target) => {
                self.dsp_rack_target = app_dsp_rack_target(target);
                self.dsp_rack_cursor = index;
                self.keep_dsp_rack_cursor_in_bounds();
                true
            }
            (
                interaction_region::DSP_RACK_TARGET
                | interaction_region::DSP_CHAIN
                | interaction_region::DSP_DEVICE_ROW,
                _,
            ) => true,
            _ => false,
        }
    }

    fn dsp_effect_count(&self, target: DspRackChain) -> usize {
        match target {
            DspRackChain::Track => self
                .song
                .tracks
                .get(self.cursor.track)
                .and_then(|track| {
                    self.song
                        .mixer
                        .tracks
                        .iter()
                        .find(|mixer| mixer.track == track.id)
                })
                .map_or(0, |mixer| mixer.effects.len()),
            DspRackChain::Master => self.song.mixer.master_effects.len(),
        }
    }
}

fn app_dsp_rack_target(target: DspRackChain) -> DspRackTarget {
    match target {
        DspRackChain::Track => DspRackTarget::Track,
        DspRackChain::Master => DspRackTarget::Master,
    }
}
