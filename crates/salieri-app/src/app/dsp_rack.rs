use super::*;

const DSP_DEVICE_PALETTE: [DspDevicePaletteEntryView<'static>; 16] = [
    DspDevicePaletteEntryView {
        label: "Gain",
        summary: "utility level",
    },
    DspDevicePaletteEntryView {
        label: "Pan",
        summary: "left/right placement",
    },
    DspDevicePaletteEntryView {
        label: "Balance",
        summary: "stereo balance",
    },
    DspDevicePaletteEntryView {
        label: "Width",
        summary: "stereo width",
    },
    DspDevicePaletteEntryView {
        label: "Phase",
        summary: "invert L/R phase",
    },
    DspDevicePaletteEntryView {
        label: "Filter",
        summary: "multimode filter",
    },
    DspDevicePaletteEntryView {
        label: "Delay",
        summary: "stereo delay",
    },
    DspDevicePaletteEntryView {
        label: "Reverb",
        summary: "room/space",
    },
    DspDevicePaletteEntryView {
        label: "Drive",
        summary: "saturation/clip",
    },
    DspDevicePaletteEntryView {
        label: "Bitcrusher",
        summary: "bit/sample reduction",
    },
    DspDevicePaletteEntryView {
        label: "Chorus",
        summary: "modulated voices",
    },
    DspDevicePaletteEntryView {
        label: "Flanger",
        summary: "comb modulation",
    },
    DspDevicePaletteEntryView {
        label: "Phaser",
        summary: "phase modulation",
    },
    DspDevicePaletteEntryView {
        label: "Compressor",
        summary: "dynamic control",
    },
    DspDevicePaletteEntryView {
        label: "Gate",
        summary: "noise gate",
    },
    DspDevicePaletteEntryView {
        label: "Limiter",
        summary: "peak ceiling",
    },
];

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
            device_palette: self
                .dsp_device_palette_open
                .then_some(DspDevicePaletteViewState {
                    entries: &DSP_DEVICE_PALETTE,
                    selected: self
                        .dsp_device_palette_cursor
                        .min(DSP_DEVICE_PALETTE.len().saturating_sub(1)),
                }),
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

    pub(crate) fn open_dsp_device_palette(&mut self) {
        self.dsp_device_palette_open = true;
        self.dsp_device_palette_cursor = self
            .dsp_device_palette_cursor
            .min(DSP_DEVICE_PALETTE.len().saturating_sub(1));
        self.notify_info("DSP device palette");
    }

    pub(crate) fn close_dsp_device_palette(&mut self) {
        self.dsp_device_palette_open = false;
    }

    pub(crate) fn move_dsp_device_palette_cursor(&mut self, delta: isize) {
        self.dsp_device_palette_cursor = self
            .dsp_device_palette_cursor
            .saturating_add_signed(delta)
            .min(DSP_DEVICE_PALETTE.len().saturating_sub(1));
    }

    pub(crate) fn assign_selected_dsp_device(&mut self) {
        let Some(device) = default_dsp_device(self.dsp_device_palette_cursor) else {
            self.notify_warning("No DSP device selected");
            return;
        };
        let label = DSP_DEVICE_PALETTE
            .get(self.dsp_device_palette_cursor)
            .map_or("DSP device", |entry| entry.label);
        match self.dsp_rack_target {
            DspRackTarget::Track => self.upsert_track_dsp_device(self.cursor.track, device),
            DspRackTarget::Master => self.upsert_master_dsp_device(device),
        }
        self.dsp_device_palette_open = false;
        self.keep_dsp_rack_cursor_in_bounds();
        self.notify_success(format!("{label} assigned"));
    }

    pub(crate) fn handle_dsp_palette_mouse_click(&mut self, row: u16) -> bool {
        let Some(cursor) = dsp_palette_row_to_cursor(row) else {
            return false;
        };
        self.dsp_device_palette_cursor = cursor.min(DSP_DEVICE_PALETTE.len().saturating_sub(1));
        self.assign_selected_dsp_device();
        true
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

fn default_dsp_device(index: usize) -> Option<EffectDevice> {
    match index {
        0 => Some(EffectDevice::gain(1, 1.0)),
        1 => Some(EffectDevice::pan(2, 0.0)),
        2 => Some(EffectDevice::balance(3, 0.0)),
        3 => Some(EffectDevice::stereo_width(4, 1.0)),
        4 => Some(EffectDevice::phase_invert(5, false, false)),
        5 => Some(EffectDevice::filter(6, FilterSpec::default())),
        6 => Some(EffectDevice::delay(7, DelaySpec::default())),
        7 => Some(EffectDevice::reverb(8, ReverbSpec::default())),
        8 => Some(EffectDevice::drive(9, DriveSpec::default())),
        9 => Some(EffectDevice::bitcrusher(10, BitcrusherSpec::default())),
        10 => Some(EffectDevice::chorus(11, ChorusSpec::default())),
        11 => Some(EffectDevice::flanger(12, FlangerSpec::default())),
        12 => Some(EffectDevice::phaser(13, PhaserSpec::default())),
        13 => Some(EffectDevice::compressor(14, CompressorSpec::default())),
        14 => Some(EffectDevice::gate(15, GateSpec::default())),
        15 => Some(EffectDevice::limiter(16, LimiterSpec::default())),
        _ => None,
    }
}

fn dsp_palette_row_to_cursor(row: u16) -> Option<usize> {
    const PALETTE_FIRST_ROW: u16 = 8;
    (row >= PALETTE_FIRST_ROW).then_some(usize::from(row - PALETTE_FIRST_ROW))
}
