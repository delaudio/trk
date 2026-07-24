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
            selected_parameter_index: self.dsp_parameter_cursor,
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
        self.keep_dsp_parameter_cursor_in_bounds();
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
        self.keep_dsp_parameter_cursor_in_bounds();
    }

    pub(crate) fn toggle_dsp_rack_target(&mut self) {
        self.dsp_rack_target = match self.dsp_rack_target {
            DspRackTarget::Track => DspRackTarget::Master,
            DspRackTarget::Master => DspRackTarget::Track,
        };
        self.keep_dsp_rack_cursor_in_bounds();
    }

    pub(crate) fn move_dsp_parameter_cursor(&mut self, delta: isize) {
        let parameter_count = self.selected_dsp_parameter_count();
        if parameter_count == 0 {
            self.dsp_parameter_cursor = 0;
            return;
        }
        self.dsp_parameter_cursor = self
            .dsp_parameter_cursor
            .saturating_add_signed(delta)
            .min(parameter_count.saturating_sub(1));
    }

    pub(crate) fn adjust_selected_dsp_parameter(&mut self, delta: f32) {
        let target = self.dsp_rack_target;
        let track_index = self.cursor.track;
        let device_index = self.dsp_rack_cursor;
        let parameter_index = self.dsp_parameter_cursor;
        let mut adjusted_device = None;
        self.mutate_song(|song, _| {
            let device = match target {
                DspRackTarget::Track => song
                    .tracks
                    .get(track_index)
                    .and_then(|track| {
                        song.mixer
                            .tracks
                            .iter_mut()
                            .find(|mixer| mixer.track == track.id)
                    })
                    .and_then(|mixer| mixer.effects.get_mut(device_index)),
                DspRackTarget::Master => song.mixer.master_effects.get_mut(device_index),
            };
            let Some(device) = device else {
                return;
            };
            let mut candidate = device.clone();
            if adjust_effect_parameter(&mut candidate.kind, parameter_index, delta)
                && effect_device_is_valid(&candidate)
            {
                *device = candidate.clone();
                adjusted_device = Some(candidate);
            }
        });
        if let Some(device) = adjusted_device {
            self.notify_success(format!("DSP {}", format_effect_device(&device)));
        } else {
            self.notify_warning("No editable DSP parameter selected");
        }
    }

    pub(crate) fn select_dsp_parameter_from_mouse(&mut self, row: u16) -> bool {
        let Some(cursor) = dsp_parameter_row_to_cursor(row) else {
            return false;
        };
        self.dsp_parameter_cursor = cursor;
        self.keep_dsp_parameter_cursor_in_bounds();
        true
    }

    pub(crate) fn handle_dsp_rack_mouse_click(&mut self, row: u16, activate: bool) {
        let selected_parameter = self.select_dsp_parameter_from_mouse(row);
        if selected_parameter && activate {
            self.adjust_selected_dsp_parameter(1.0);
        }
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

    fn keep_dsp_parameter_cursor_in_bounds(&mut self) {
        self.dsp_parameter_cursor = self
            .dsp_parameter_cursor
            .min(self.selected_dsp_parameter_count().saturating_sub(1));
    }

    fn selected_dsp_parameter_count(&self) -> usize {
        self.selected_dsp_effect_kind()
            .map_or(0, effect_parameter_count)
    }

    fn selected_dsp_effect_kind(&self) -> Option<&EffectDeviceKind> {
        match self.dsp_rack_target {
            DspRackTarget::Track => self
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
                .and_then(|mixer| mixer.effects.get(self.dsp_rack_cursor))
                .map(|device| &device.kind),
            DspRackTarget::Master => self
                .song
                .mixer
                .master_effects
                .get(self.dsp_rack_cursor)
                .map(|device| &device.kind),
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

fn dsp_parameter_row_to_cursor(row: u16) -> Option<usize> {
    const PARAMETER_FIRST_ROW: u16 = 19;
    (row >= PARAMETER_FIRST_ROW).then_some(usize::from(row - PARAMETER_FIRST_ROW))
}

fn effect_parameter_count(kind: &EffectDeviceKind) -> usize {
    match kind {
        EffectDeviceKind::Gain { .. }
        | EffectDeviceKind::Pan { .. }
        | EffectDeviceKind::Balance { .. }
        | EffectDeviceKind::StereoWidth { .. } => 1,
        EffectDeviceKind::PhaseInvert { .. } => 2,
        EffectDeviceKind::Filter { .. } => 5,
        EffectDeviceKind::Delay { .. } => 6,
        EffectDeviceKind::Reverb { .. } => 4,
        EffectDeviceKind::Drive { .. } => 4,
        EffectDeviceKind::Bitcrusher { .. } => 4,
        EffectDeviceKind::Chorus { .. } => 4,
        EffectDeviceKind::Flanger { .. } => 5,
        EffectDeviceKind::Phaser { .. } => 5,
        EffectDeviceKind::Compressor { .. } => 5,
        EffectDeviceKind::Gate { .. } => 4,
        EffectDeviceKind::Limiter { .. } => 4,
    }
}

fn adjust_effect_parameter(kind: &mut EffectDeviceKind, index: usize, delta: f32) -> bool {
    match kind {
        EffectDeviceKind::Gain { gain } => adjust_float(gain, delta, 0.0, 4.0, 0.05, index, 0),
        EffectDeviceKind::Pan { pan } => adjust_float(pan, delta, -1.0, 1.0, 0.05, index, 0),
        EffectDeviceKind::Balance { balance } => {
            adjust_float(balance, delta, -1.0, 1.0, 0.05, index, 0)
        }
        EffectDeviceKind::StereoWidth { width } => {
            adjust_float(width, delta, 0.0, 2.0, 0.05, index, 0)
        }
        EffectDeviceKind::PhaseInvert {
            invert_left,
            invert_right,
        } => match index {
            0 => toggle_bool(invert_left),
            1 => toggle_bool(invert_right),
            _ => false,
        },
        EffectDeviceKind::Filter {
            mode,
            cutoff_hz,
            resonance,
            drive_db,
            mix,
            ..
        } => match index {
            0 => cycle_filter_mode(mode, delta),
            1 => adjust_scaled(cutoff_hz, delta, 20.0, 20_000.0, 200.0),
            2 => adjust_scaled(resonance, delta, 0.0, 1.0, 0.05),
            3 => adjust_scaled(drive_db, delta, -24.0, 24.0, 0.5),
            4 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Delay {
            sync,
            time_left_ms,
            time_right_ms,
            feedback,
            ping_pong,
            mix,
            ..
        } => match index {
            0 => toggle_bool(sync),
            1 => adjust_scaled(time_left_ms, delta, 1.0, 4_000.0, 25.0),
            2 => adjust_scaled(time_right_ms, delta, 1.0, 4_000.0, 25.0),
            3 => adjust_scaled(feedback, delta, 0.0, 0.95, 0.05),
            4 => toggle_bool(ping_pong),
            5 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Reverb {
            size,
            predelay_ms,
            decay_s,
            mix,
            ..
        } => match index {
            0 => adjust_scaled(size, delta, 0.0, 1.0, 0.05),
            1 => adjust_scaled(predelay_ms, delta, 0.0, 250.0, 5.0),
            2 => adjust_scaled(decay_s, delta, 0.1, 30.0, 0.1),
            3 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Drive {
            mode,
            drive_db,
            tone,
            mix,
            ..
        } => match index {
            0 => cycle_drive_mode(mode, delta),
            1 => adjust_scaled(drive_db, delta, 0.0, 48.0, 0.5),
            2 => adjust_scaled(tone, delta, 0.0, 1.0, 0.05),
            3 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Bitcrusher {
            bit_depth,
            reduction_ratio,
            dither,
            mix,
            ..
        } => match index {
            0 => adjust_u8(bit_depth, delta, 1, 24),
            1 => adjust_scaled(reduction_ratio, delta, 1.0, 64.0, 1.0),
            2 => toggle_bool(dither),
            3 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Chorus {
            rate_hz,
            depth,
            voices,
            mix,
            ..
        } => match index {
            0 => adjust_scaled(rate_hz, delta, 0.01, 10.0, 0.05),
            1 => adjust_scaled(depth, delta, 0.0, 1.0, 0.05),
            2 => adjust_u8(voices, delta, 1, 8),
            3 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Flanger {
            rate_hz,
            depth,
            manual,
            feedback,
            mix,
            ..
        } => match index {
            0 => adjust_scaled(rate_hz, delta, 0.01, 10.0, 0.05),
            1 => adjust_scaled(depth, delta, 0.0, 1.0, 0.05),
            2 => adjust_scaled(manual, delta, 0.0, 1.0, 0.05),
            3 => adjust_scaled(feedback, delta, -0.95, 0.95, 0.05),
            4 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Phaser {
            rate_hz,
            depth,
            center_hz,
            stages,
            mix,
            ..
        } => match index {
            0 => adjust_scaled(rate_hz, delta, 0.01, 10.0, 0.05),
            1 => adjust_scaled(depth, delta, 0.0, 1.0, 0.05),
            2 => adjust_scaled(center_hz, delta, 20.0, 20_000.0, 100.0),
            3 => adjust_u8(stages, delta, 2, 12),
            4 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            mix,
            ..
        } => match index {
            0 => adjust_scaled(threshold_db, delta, -80.0, 0.0, 1.0),
            1 => adjust_scaled(ratio, delta, 1.0, 20.0, 0.5),
            2 => adjust_scaled(attack_ms, delta, 0.1, 200.0, 1.0),
            3 => adjust_scaled(release_ms, delta, 1.0, 2_000.0, 10.0),
            4 => adjust_scaled(mix, delta, 0.0, 1.0, 0.05),
            _ => false,
        },
        EffectDeviceKind::Gate {
            threshold_db,
            hysteresis_db,
            attack_ms,
            release_ms,
            ..
        } => match index {
            0 => adjust_scaled(threshold_db, delta, -80.0, 0.0, 1.0),
            1 => adjust_scaled(hysteresis_db, delta, 0.0, 24.0, 0.5),
            2 => adjust_scaled(attack_ms, delta, 0.1, 200.0, 1.0),
            3 => adjust_scaled(release_ms, delta, 1.0, 2_000.0, 10.0),
            _ => false,
        },
        EffectDeviceKind::Limiter {
            ceiling_db,
            input_gain_db,
            release_ms,
            lookahead_ms,
            ..
        } => match index {
            0 => adjust_scaled(ceiling_db, delta, -24.0, 0.0, 0.5),
            1 => adjust_scaled(input_gain_db, delta, -24.0, 24.0, 0.5),
            2 => adjust_scaled(release_ms, delta, 1.0, 2_000.0, 10.0),
            3 => adjust_scaled(lookahead_ms, delta, 0.0, 50.0, 1.0),
            _ => false,
        },
    }
}

fn adjust_float(
    value: &mut f32,
    delta: f32,
    min: f32,
    max: f32,
    step: f32,
    index: usize,
    expected_index: usize,
) -> bool {
    if index != expected_index {
        return false;
    }
    adjust_scaled(value, delta, min, max, step)
}

fn adjust_scaled(value: &mut f32, delta: f32, min: f32, max: f32, step: f32) -> bool {
    *value = (*value + delta.signum() * step).clamp(min, max);
    true
}

fn adjust_u8(value: &mut u8, delta: f32, min: u8, max: u8) -> bool {
    *value = value
        .saturating_add_signed(delta.signum() as i8)
        .clamp(min, max);
    true
}

fn toggle_bool(value: &mut bool) -> bool {
    *value = !*value;
    true
}

fn cycle_filter_mode(mode: &mut FilterMode, delta: f32) -> bool {
    const MODES: [FilterMode; 4] = [
        FilterMode::LowPass,
        FilterMode::HighPass,
        FilterMode::BandPass,
        FilterMode::Notch,
    ];
    *mode = cycle_copy(MODES.as_slice(), *mode, delta);
    true
}

fn cycle_drive_mode(mode: &mut DriveMode, delta: f32) -> bool {
    const MODES: [DriveMode; 4] = [
        DriveMode::Overdrive,
        DriveMode::Saturation,
        DriveMode::HardClip,
        DriveMode::SoftClip,
    ];
    *mode = cycle_copy(MODES.as_slice(), *mode, delta);
    true
}

fn cycle_copy<T: Copy + PartialEq>(values: &[T], current: T, delta: f32) -> T {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    let next = if delta.is_sign_negative() {
        index.saturating_sub(1)
    } else {
        (index + 1).min(values.len().saturating_sub(1))
    };
    values[next]
}
