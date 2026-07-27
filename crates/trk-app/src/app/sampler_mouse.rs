use super::*;

impl App {
    pub(super) fn handle_sampler_mouse_click(&mut self, column: u16, row: u16) {
        let Some(region) = self.interaction_map.hit_test(column, row) else {
            return;
        };
        let (interaction_region::SAMPLER_ACTION, InteractionPayload::SamplerAction { action }) =
            (region.id, region.payload)
        else {
            return;
        };
        if action == SamplerAction::Browse {
            self.open_sample_browser_view(None);
            return;
        }
        if self.sample_view.is_none() {
            return;
        }
        match action {
            SamplerAction::SelectEnvelope(field) => self.select_sampler_envelope_field(field),
            SamplerAction::DecrementEnvelope => {
                self.adjust_selected_sampler_envelope(-1.0, false);
            }
            SamplerAction::IncrementEnvelope => {
                self.adjust_selected_sampler_envelope(1.0, false);
            }
            SamplerAction::ZoomOut => self.zoom_sample_waveform_out(),
            SamplerAction::ZoomIn => self.zoom_sample_waveform_in(),
            SamplerAction::PanLeft => self.pan_sample_waveform(-1),
            SamplerAction::PanRight => self.pan_sample_waveform(1),
            SamplerAction::Browse => {}
        }
    }
}
