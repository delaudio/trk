use super::*;

const SLOT_COUNT: usize = 8;

#[derive(Debug, Clone)]
struct ParameterBinding {
    label: String,
    descriptor: Option<ParameterDescriptor>,
    target: Option<ParameterLockTarget>,
    base_value: Option<ParameterValue>,
    disabled_reason: Option<String>,
}

impl ParameterBinding {
    fn enabled(
        descriptor: ParameterDescriptor,
        target: ParameterLockTarget,
        base_value: ParameterValue,
    ) -> Self {
        let label = descriptor
            .short_name
            .clone()
            .unwrap_or_else(|| descriptor.name.clone());
        let disabled_reason = (!descriptor.flags.automatable || descriptor.flags.read_only)
            .then(|| "Not lockable".to_string());
        Self {
            label,
            descriptor: Some(descriptor),
            target: Some(target),
            base_value: Some(base_value),
            disabled_reason,
        }
    }

    fn disabled(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            descriptor: None,
            target: None,
            base_value: None,
            disabled_reason: Some(reason.into()),
        }
    }

    fn is_enabled(&self) -> bool {
        self.descriptor.is_some()
            && self.target.is_some()
            && self.base_value.is_some()
            && self.disabled_reason.is_none()
    }
}

impl App {
    pub(crate) fn handle_performance_surface_shortcut(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.mode, AppMode::Normal | AppMode::ParameterPage) {
            return false;
        }
        if key.modifiers == KeyModifiers::SHIFT {
            if self.mode == AppMode::Normal {
                match key.code {
                    KeyCode::Char('s' | 'S') => {
                        self.temp_save_performance_state();
                        return true;
                    }
                    KeyCode::Char('r' | 'R') => {
                        self.reload_temp_performance_state();
                        return true;
                    }
                    _ => {}
                }
            }
            if let Some(index) = shifted_track_index(key.code) {
                self.toggle_instant_track_mute(index);
                return true;
            }
        }
        if key.modifiers.is_empty() {
            if let KeyCode::F(number) = key.code {
                if let Some(page) = ParameterPage::from_function_key(number) {
                    self.open_parameter_page(page);
                    return true;
                }
            }
        }
        if key.modifiers.is_empty() {
            if let KeyCode::Char(number @ '1'..='6') = key.code {
                let number = number as u8 - b'0';
                if let Some(page) = ParameterPage::from_function_key(number) {
                    self.open_parameter_page(page);
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn open_parameter_page(&mut self, page: ParameterPage) {
        if self.mode == AppMode::ParameterPage && self.parameter_surface.page == page {
            self.open_parameter_page_deep_editor(page);
            return;
        }
        self.parameter_surface.page = page;
        self.parameter_surface.selected = self
            .parameter_surface
            .selected
            .min(SLOT_COUNT.saturating_sub(1));
        self.parameter_surface.clear_armed = false;
        self.mode = AppMode::ParameterPage;
        self.notify_info(format!("{} parameter page", page.label()));
    }

    pub(crate) fn handle_parameter_page_key(&mut self, key: KeyEvent) {
        if key.modifiers.is_empty() {
            if let KeyCode::F(number) = key.code {
                if let Some(page) = ParameterPage::from_function_key(number) {
                    self.open_parameter_page(page);
                    return;
                }
            }
        }
        if let KeyCode::Char(value) = key.code {
            if let Some(index) = trk_core::parameter_encoder_index(value) {
                if self.parameter_surface.clear_armed {
                    self.parameter_surface.clear_armed = false;
                    self.parameter_surface.selected = index;
                    self.clear_parameter_page_slot(index);
                } else if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.parameter_surface.selected = index;
                    self.adjust_parameter_page_slot(index, 1, true);
                } else {
                    self.parameter_surface.selected = index;
                }
                return;
            }
        }
        match key.code {
            KeyCode::Esc => {
                self.parameter_surface.clear_armed = false;
                self.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                self.parameter_surface.clear_armed = true;
                self.notify_info("Clear P-lock: press Q/W/E/R/A/S/D/F");
            }
            KeyCode::Up | KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_selected_parameter_page_slot(
                    1,
                    key.modifiers.contains(KeyModifiers::SHIFT),
                );
            }
            KeyCode::Down | KeyCode::Left | KeyCode::Char('-') => {
                self.adjust_selected_parameter_page_slot(
                    -1,
                    key.modifiers.contains(KeyModifiers::SHIFT),
                );
            }
            _ => {}
        }
    }

    pub(crate) fn select_parameter_page_slot(&mut self, index: usize) -> bool {
        if self.mode != AppMode::ParameterPage || index >= SLOT_COUNT {
            return false;
        }
        self.parameter_surface.selected = index;
        self.parameter_surface.clear_armed = false;
        true
    }

    pub(crate) fn adjust_parameter_page_slot_from_pointer(&mut self, index: usize, delta: i8) {
        if self.mode != AppMode::ParameterPage || index >= SLOT_COUNT || delta == 0 {
            return;
        }
        self.parameter_surface.selected = index;
        self.adjust_parameter_page_slot(index, delta.signum(), false);
    }

    pub(crate) fn tui_parameter_page_slots(&self) -> Vec<ParameterPageSlotView> {
        if self.mode != AppMode::ParameterPage {
            return Vec::new();
        }
        self.parameter_page_bindings()
            .into_iter()
            .take(SLOT_COUNT)
            .enumerate()
            .map(|(index, binding)| {
                let (value, locked) = self.binding_value_and_lock(&binding);
                let enabled = binding.is_enabled();
                let value_label = match (&binding.descriptor, &value) {
                    (Some(descriptor), Some(value)) => descriptor.format_value(value),
                    _ => "—".to_string(),
                };
                let meter_percent = match (&binding.descriptor, &value) {
                    (Some(descriptor), Some(value)) => parameter_meter(descriptor, value),
                    _ => 0,
                };
                ParameterPageSlotView {
                    key: trk_core::PARAMETER_ENCODER_KEYS[index],
                    label: binding.label,
                    value: value_label,
                    meter_percent,
                    locked,
                    enabled,
                    disabled_reason: binding.disabled_reason,
                }
            })
            .collect()
    }

    pub(crate) fn tui_parameter_page<'a>(
        &'a self,
        slots: &'a [ParameterPageSlotView],
    ) -> Option<ParameterPageViewState<'a>> {
        (self.mode == AppMode::ParameterPage).then(|| {
            let track = self.song.tracks.get(self.cursor.track);
            ParameterPageViewState {
                page: self.parameter_surface.page,
                selected: self.parameter_surface.selected,
                row: self.cursor.row,
                track_number: self.cursor.track + 1,
                track_name: track.map_or("<missing>", |track| track.name.as_str()),
                slots,
                has_snapshot: self.parameter_surface.snapshot.is_some(),
                reload_pending: self.parameter_surface.pending_reload.is_some(),
            }
        })
    }

    pub(crate) fn temp_save_performance_state(&mut self) {
        self.parameter_surface.snapshot = Some(self.song.clone());
        self.parameter_surface.pending_reload = None;
        self.notify_success("Temporary performance state saved");
    }

    pub(crate) fn clear_performance_state_for_project_change(&mut self) {
        self.parameter_surface.snapshot = None;
        self.parameter_surface.pending_reload = None;
    }

    pub(crate) fn reload_temp_performance_state(&mut self) {
        let Some(snapshot) = self.parameter_surface.snapshot.clone() else {
            self.notify_warning("No temporary performance state saved");
            return;
        };
        if self.is_playing {
            let token = self.parameter_surface.next_reload_token;
            self.parameter_surface.next_reload_token = token.saturating_add(1);
            self.parameter_surface.pending_reload = Some((token, snapshot.clone()));
            self.playback.reload_song_at_next_beat(snapshot, token);
            self.notify_info("Performance reload queued for next beat");
            return;
        }
        self.apply_performance_snapshot(snapshot, "Reload temporary performance state");
        self.notify_success("Temporary performance state restored");
    }

    pub(crate) fn apply_pending_performance_reload(&mut self, token: u64) {
        let Some((pending_token, snapshot)) = self.parameter_surface.pending_reload.take() else {
            return;
        };
        if pending_token != token {
            self.parameter_surface.pending_reload = Some((pending_token, snapshot));
            return;
        }
        self.apply_performance_snapshot(snapshot, "Reload temporary performance state");
        self.notify_success("Temporary performance state restored on beat");
    }

    pub(crate) fn toggle_instant_track_mute(&mut self, index: usize) {
        if index >= self.song.tracks.len() {
            return;
        }
        let mut muted = false;
        self.mutate_song_with(
            TransactionSpec::new("Toggle performance mute"),
            |song, _| {
                if song.toggle_mute(index).is_ok() {
                    muted = song.tracks[index].muted;
                }
            },
        );
        if self.is_playing {
            self.playback
                .apply_live_mute(self.song.tracks[index].id, muted);
        }
        self.notify_success(format!(
            "Track {:02} {}",
            index + 1,
            if muted { "muted" } else { "unmuted" }
        ));
    }

    fn apply_performance_snapshot(&mut self, snapshot: Song, label: &'static str) {
        self.mutate_song_with(TransactionSpec::new(label), |song, _| *song = snapshot);
        self.clamp_cursor();
        self.clamp_sequence_cursor();
        self.clamp_clip_cursor();
    }

    fn adjust_selected_parameter_page_slot(&mut self, direction: i8, coarse: bool) {
        self.adjust_parameter_page_slot(self.parameter_surface.selected, direction, coarse);
    }

    fn adjust_parameter_page_slot(&mut self, index: usize, direction: i8, coarse: bool) {
        let Some(binding) = self.parameter_page_bindings().get(index).cloned() else {
            return;
        };
        if !binding.is_enabled() {
            self.notify_warning(
                binding
                    .disabled_reason
                    .as_deref()
                    .unwrap_or("Parameter unavailable"),
            );
            return;
        }
        let descriptor = binding.descriptor.as_ref().expect("enabled descriptor");
        let target = binding.target.clone().expect("enabled target");
        let current = self
            .binding_value_and_lock(&binding)
            .0
            .expect("enabled value");
        let Some(value) = adjusted_parameter_value(descriptor, &current, direction, coarse) else {
            return;
        };
        if value == current {
            return;
        }
        self.set_current_parameter_lock(
            ParameterLock {
                target,
                parameter: descriptor.id.clone(),
                action: ParameterLockAction::Set { value },
            },
            descriptor,
        );
        self.update_live_playback_pattern(self.pattern_index);
    }

    fn clear_parameter_page_slot(&mut self, index: usize) {
        let Some(binding) = self.parameter_page_bindings().get(index).cloned() else {
            return;
        };
        if !binding.is_enabled() {
            self.notify_warning(
                binding
                    .disabled_reason
                    .as_deref()
                    .unwrap_or("Parameter unavailable"),
            );
            return;
        }
        let (Some(target), Some(descriptor)) = (binding.target, binding.descriptor) else {
            self.notify_warning("Parameter unavailable");
            return;
        };
        self.clear_current_parameter_lock(target, descriptor.id);
        self.update_live_playback_pattern(self.pattern_index);
    }

    fn binding_value_and_lock(&self, binding: &ParameterBinding) -> (Option<ParameterValue>, bool) {
        let (Some(target), Some(descriptor), Some(base)) = (
            binding.target.as_ref(),
            binding.descriptor.as_ref(),
            binding.base_value.as_ref(),
        ) else {
            return (None, false);
        };
        let lock = self
            .song
            .pattern(self.pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .and_then(|cell| {
                cell.parameter_locks
                    .iter()
                    .find(|lock| &lock.target == target && lock.parameter == descriptor.id)
            });
        match lock.map(|lock| &lock.action) {
            Some(ParameterLockAction::Set { value }) => (Some(value.clone()), true),
            Some(ParameterLockAction::Reset) => (Some(base.clone()), true),
            None => (Some(base.clone()), false),
        }
    }

    fn parameter_page_bindings(&self) -> Vec<ParameterBinding> {
        let mut bindings = match self.parameter_surface.page {
            ParameterPage::Source => self.source_page_bindings(),
            ParameterPage::Filter => self.effect_page_bindings(effect_is_filter, |_| true),
            ParameterPage::Amp => self.amp_page_bindings(),
            ParameterPage::Effects => self.effects_page_bindings(),
            ParameterPage::Lfo => self.effect_page_bindings(effect_is_modulation, lfo_parameter),
            ParameterPage::Algorithm => algorithm_bindings(),
        };
        let fallback = match self.parameter_surface.page {
            ParameterPage::Source => "No assigned sample",
            ParameterPage::Filter => "Add a filter in DSP",
            ParameterPage::Amp => "No amp parameter",
            ParameterPage::Effects => "Add an effect or send",
            ParameterPage::Lfo => "Add a modulation effect",
            ParameterPage::Algorithm => "Open ALG again",
        };
        while bindings.len() < SLOT_COUNT {
            bindings.push(ParameterBinding::disabled(
                format!("Slot {}", bindings.len() + 1),
                fallback,
            ));
        }
        bindings.truncate(SLOT_COUNT);
        bindings
    }

    fn source_page_bindings(&self) -> Vec<ParameterBinding> {
        let Some(track) = self.song.tracks.get(self.cursor.track) else {
            return Vec::new();
        };
        let Some(sample) = self.song.sample_for_track(track.id) else {
            return Vec::new();
        };
        let target = ParameterLockTarget::Sample { sample: sample.id };
        let mut bindings = [
            sample_gain_descriptor(),
            trk_core::sample_root_note_descriptor(),
            trk_core::sample_start_frame_descriptor(),
            trk_core::sample_end_frame_descriptor(),
            trk_core::sample_playback_mode_descriptor(),
            trk_core::sample_loop_start_frame_descriptor(),
        ]
        .into_iter()
        .map(|descriptor| {
            if descriptor.id.as_str() == trk_core::SAMPLE_END_FRAME_PARAMETER_ID
                && sample.playback.end_frame.is_none()
            {
                return ParameterBinding::disabled("End", "Set sample end in Sampler");
            }
            let value = sample_parameter_value(sample, descriptor.id.as_str());
            ParameterBinding::enabled(descriptor, target.clone(), value)
        })
        .collect::<Vec<_>>();
        bindings.push(ParameterBinding::disabled("Sample", "Open SRC again"));
        bindings.push(ParameterBinding::disabled(
            "Polyphony",
            "Voice mode unavailable",
        ));
        bindings
    }

    fn amp_page_bindings(&self) -> Vec<ParameterBinding> {
        let mut bindings = Vec::new();
        if let Some(track) = self.song.tracks.get(self.cursor.track) {
            if let Some(sample) = self.song.sample_for_track(track.id) {
                let target = ParameterLockTarget::Sample { sample: sample.id };
                for descriptor in [
                    sample_envelope_attack_descriptor(),
                    sample_envelope_decay_descriptor(),
                    sample_envelope_sustain_descriptor(),
                    sample_envelope_release_descriptor(),
                ] {
                    let value = sample_parameter_value(sample, descriptor.id.as_str());
                    bindings.push(ParameterBinding::enabled(descriptor, target.clone(), value));
                }
            }
            let mixer = self.song.track_mixer_for_track(track.id);
            bindings.push(ParameterBinding::enabled(
                mixer_track_gain_descriptor(),
                ParameterLockTarget::TrackMixer { track: track.id },
                ParameterValue::Float(mixer.gain),
            ));
            bindings.push(ParameterBinding::enabled(
                mixer_track_pan_descriptor(),
                ParameterLockTarget::TrackMixer { track: track.id },
                ParameterValue::Bipolar(mixer.pan),
            ));
        }
        let mut drive = self.effect_page_bindings(effect_is_drive, |descriptor| {
            matches!(
                descriptor.id.as_str(),
                trk_core::NATIVE_DRIVE_DRIVE_PARAMETER_ID | trk_core::NATIVE_DRIVE_MIX_PARAMETER_ID
            )
        });
        bindings.append(&mut drive);
        bindings
    }

    fn effects_page_bindings(&self) -> Vec<ParameterBinding> {
        let mut bindings = Vec::new();
        if let Some(track) = self.song.tracks.get(self.cursor.track) {
            let mixer = self.song.track_mixer_for_track(track.id);
            for send in mixer.sends {
                bindings.push(ParameterBinding::enabled(
                    mixer_send_gain_descriptor(),
                    ParameterLockTarget::TrackSend {
                        track: track.id,
                        send: send.send,
                    },
                    ParameterValue::Float(send.gain),
                ));
            }
        }
        let mut effects = self.effect_page_bindings(effect_is_space_or_color, |_| true);
        bindings.append(&mut effects);
        bindings
    }

    fn effect_page_bindings(
        &self,
        include_effect: fn(&EffectDeviceKind) -> bool,
        include_parameter: fn(&ParameterDescriptor) -> bool,
    ) -> Vec<ParameterBinding> {
        let Some(track) = self.song.tracks.get(self.cursor.track) else {
            return Vec::new();
        };
        let Some(mixer) = self
            .song
            .mixer
            .tracks
            .iter()
            .find(|mixer| mixer.track == track.id)
        else {
            return Vec::new();
        };
        let mut bindings = Vec::new();
        for effect in mixer
            .effects
            .iter()
            .filter(|effect| include_effect(&effect.kind))
        {
            let target = ParameterLockTarget::TrackEffect {
                track: track.id,
                device: effect.id,
            };
            for descriptor in effect
                .native_module_descriptor()
                .parameters
                .into_iter()
                .filter(include_parameter)
            {
                if let Some(value) = effect.parameter_value(&descriptor.id) {
                    bindings.push(ParameterBinding::enabled(descriptor, target.clone(), value));
                }
                if bindings.len() == SLOT_COUNT {
                    return bindings;
                }
            }
        }
        bindings
    }

    fn open_parameter_page_deep_editor(&mut self, page: ParameterPage) {
        self.parameter_surface.clear_armed = false;
        match page {
            ParameterPage::Source => self.open_sampler_view(),
            ParameterPage::Filter
            | ParameterPage::Amp
            | ParameterPage::Effects
            | ParameterPage::Lfo => self.open_dsp_rack_view(),
            ParameterPage::Algorithm => self.open_strudel_live(String::new()),
        }
    }
}

fn sample_parameter_value(sample: &SampleReference, parameter: &str) -> ParameterValue {
    match parameter {
        SAMPLE_GAIN_PARAMETER_ID => ParameterValue::Float(sample.gain),
        trk_core::SAMPLE_ROOT_NOTE_PARAMETER_ID => ParameterValue::Note(sample.root_pitch),
        trk_core::SAMPLE_PLAYBACK_MODE_PARAMETER_ID => {
            ParameterValue::Enum(sample_mode_id(sample.playback.mode).to_string())
        }
        trk_core::SAMPLE_START_FRAME_PARAMETER_ID => {
            ParameterValue::Integer(sample.playback.start_frame.unwrap_or(0) as i64)
        }
        trk_core::SAMPLE_END_FRAME_PARAMETER_ID => {
            ParameterValue::Integer(sample.playback.end_frame.unwrap_or(0) as i64)
        }
        trk_core::SAMPLE_LOOP_START_FRAME_PARAMETER_ID => {
            ParameterValue::Integer(sample.playback.loop_start_frame.unwrap_or(0) as i64)
        }
        trk_core::SAMPLE_LOOP_END_FRAME_PARAMETER_ID => {
            ParameterValue::Integer(sample.playback.loop_end_frame.unwrap_or(0) as i64)
        }
        trk_core::SAMPLE_ENVELOPE_ATTACK_PARAMETER_ID => {
            ParameterValue::Seconds(sample.playback.envelope.attack_seconds)
        }
        trk_core::SAMPLE_ENVELOPE_DECAY_PARAMETER_ID => {
            ParameterValue::Seconds(sample.playback.envelope.decay_seconds)
        }
        trk_core::SAMPLE_ENVELOPE_SUSTAIN_PARAMETER_ID => {
            ParameterValue::Percentage(sample.playback.envelope.sustain)
        }
        trk_core::SAMPLE_ENVELOPE_RELEASE_PARAMETER_ID => {
            ParameterValue::Seconds(sample.playback.envelope.release_seconds)
        }
        _ => ParameterValue::Unknown {
            value_type: "unavailable".to_string(),
            raw: String::new(),
        },
    }
}

fn sample_mode_id(mode: SamplePlaybackMode) -> &'static str {
    match mode {
        SamplePlaybackMode::OneShot => "oneShot",
        SamplePlaybackMode::Loop => "loop",
        SamplePlaybackMode::ForwardLoop => "forwardLoop",
        SamplePlaybackMode::BackwardLoop => "backwardLoop",
        SamplePlaybackMode::PingPongLoop => "pingPongLoop",
        SamplePlaybackMode::Reverse => "reverse",
    }
}

fn parameter_meter(descriptor: &ParameterDescriptor, value: &ParameterValue) -> u8 {
    match value {
        ParameterValue::Bool(value) => u8::from(*value) * 100,
        ParameterValue::Enum(selected) => match &descriptor.range {
            ParameterRange::Enum { choices } => {
                let index = choices
                    .iter()
                    .position(|choice| choice.id == *selected)
                    .unwrap_or(0);
                if choices.len() <= 1 {
                    0
                } else {
                    u8::try_from(index * 100 / (choices.len() - 1)).unwrap_or(100)
                }
            }
            _ => 0,
        },
        _ => descriptor
            .plain_to_normalized(value)
            .map(|normalized| (normalized * 100.0).round() as u8)
            .unwrap_or(0),
    }
}

fn adjusted_parameter_value(
    descriptor: &ParameterDescriptor,
    current: &ParameterValue,
    direction: i8,
    coarse: bool,
) -> Option<ParameterValue> {
    match &descriptor.range {
        ParameterRange::Boolean => Some(ParameterValue::Bool(direction > 0)),
        ParameterRange::Enum { choices } => {
            if choices.is_empty() {
                return None;
            }
            let current_index = match current {
                ParameterValue::Enum(value) => choices
                    .iter()
                    .position(|choice| choice.id == *value)
                    .unwrap_or(0),
                _ => 0,
            };
            let next = current_index
                .saturating_add_signed(isize::from(direction))
                .min(choices.len() - 1);
            Some(ParameterValue::Enum(choices[next].id.clone()))
        }
        ParameterRange::Integer { min, max, step } => {
            let step = i64::try_from(step.unwrap_or(1)).unwrap_or(i64::MAX);
            let multiplier = if coarse { 10 } else { 1 };
            let current = current.as_f32()? as i64;
            let candidate = current
                .saturating_add(
                    i64::from(direction)
                        .saturating_mul(step)
                        .saturating_mul(multiplier),
                )
                .clamp(*min, *max);
            Some(descriptor.clamp(&descriptor.value_from_f32(candidate as f32)))
        }
        ParameterRange::Continuous { min, max, step } => {
            if let Some(step) = step.filter(|step| *step > 0.0) {
                let multiplier = if coarse { 10.0 } else { 1.0 };
                let candidate = current
                    .as_f32()?
                    .mul_add(1.0, f32::from(direction) * step * multiplier);
                return Some(descriptor.clamp(&descriptor.value_from_f32(candidate)));
            }
            let normalized = descriptor.plain_to_normalized(current).ok()?;
            let amount = if coarse { 0.1 } else { 0.01 };
            descriptor
                .normalized_to_plain((normalized + f32::from(direction) * amount).clamp(0.0, 1.0))
                .ok()
                .or_else(|| Some(descriptor.clamp(&descriptor.value_from_f32(*min))))
                .map(|value| descriptor.clamp(&value))
                .filter(|_| min < max)
        }
        ParameterRange::Unbounded => {
            let amount = if coarse { 10.0 } else { 1.0 };
            Some(current.with_numeric_value(current.as_f32()? + f32::from(direction) * amount))
        }
    }
}

fn effect_is_filter(kind: &EffectDeviceKind) -> bool {
    matches!(kind, EffectDeviceKind::Filter { .. })
}

fn effect_is_drive(kind: &EffectDeviceKind) -> bool {
    matches!(kind, EffectDeviceKind::Drive { .. })
}

fn effect_is_modulation(kind: &EffectDeviceKind) -> bool {
    matches!(
        kind,
        EffectDeviceKind::Chorus { .. }
            | EffectDeviceKind::Flanger { .. }
            | EffectDeviceKind::Phaser { .. }
            | EffectDeviceKind::Delay { .. }
    )
}

fn effect_is_space_or_color(kind: &EffectDeviceKind) -> bool {
    matches!(
        kind,
        EffectDeviceKind::Delay { .. }
            | EffectDeviceKind::Reverb { .. }
            | EffectDeviceKind::Drive { .. }
            | EffectDeviceKind::Bitcrusher { .. }
            | EffectDeviceKind::Chorus { .. }
            | EffectDeviceKind::Flanger { .. }
            | EffectDeviceKind::Phaser { .. }
    )
}

fn lfo_parameter(descriptor: &ParameterDescriptor) -> bool {
    let id = descriptor.id.as_str().to_ascii_lowercase();
    [
        "rate", "depth", "manual", "feedback", "center", "phase", "sync",
    ]
    .iter()
    .any(|token| id.contains(token))
}

fn algorithm_bindings() -> Vec<ParameterBinding> {
    [
        "Pulses", "Steps", "Rotate", "Strudel", "Scale", "Root", "Humanize", "Swing",
    ]
    .into_iter()
    .map(|label| ParameterBinding::disabled(label, "Open ALG again"))
    .collect()
}

fn shifted_track_index(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Char('1' | '!') => Some(0),
        KeyCode::Char('2' | '@') => Some(1),
        KeyCode::Char('3' | '#') => Some(2),
        KeyCode::Char('4' | '$') => Some(3),
        KeyCode::Char('5' | '%') => Some(4),
        KeyCode::Char('6' | '^') => Some(5),
        KeyCode::Char('7' | '&') => Some(6),
        KeyCode::Char('8' | '*') => Some(7),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_and_enum_adjustments_are_bounded() {
        let boolean = trk_core::native_delay_sync_descriptor();
        assert_eq!(
            adjusted_parameter_value(&boolean, &ParameterValue::Bool(false), 1, false),
            Some(ParameterValue::Bool(true))
        );
        assert_eq!(
            adjusted_parameter_value(&boolean, &ParameterValue::Bool(true), -1, false),
            Some(ParameterValue::Bool(false))
        );

        let mode = trk_core::sample_playback_mode_descriptor();
        assert_eq!(
            adjusted_parameter_value(
                &mode,
                &ParameterValue::Enum("oneShot".to_string()),
                -1,
                false,
            ),
            Some(ParameterValue::Enum("oneShot".to_string()))
        );
    }

    #[test]
    fn algorithm_page_exposes_eight_explicitly_disabled_slots() {
        let slots = algorithm_bindings();
        assert_eq!(slots.len(), 8);
        assert!(slots.iter().all(|slot| !slot.is_enabled()));
    }
}
