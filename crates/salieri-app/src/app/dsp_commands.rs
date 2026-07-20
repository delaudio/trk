use super::*;

impl App {
    pub(crate) fn handle_dsp_command(&mut self, values: &[&str]) {
        match values {
            ["master", "clear"] => self.clear_master_dsp_chain(),
            ["master", "gain", value] => self.set_master_dsp_value(
                value,
                |gain| EffectDevice::gain(1, gain),
                "Usage: :dsp master gain GAIN",
            ),
            ["master", "pan", value] => self.set_master_dsp_value(
                value,
                |pan| EffectDevice::pan(2, pan),
                "Usage: :dsp master pan PAN",
            ),
            ["master", "balance" | "bal", value] => self.set_master_dsp_value(
                value,
                |balance| EffectDevice::balance(3, balance),
                "Usage: :dsp master balance BALANCE",
            ),
            ["master", "width" | "stereo-width", value] => self.set_master_dsp_value(
                value,
                |width| EffectDevice::stereo_width(4, width),
                "Usage: :dsp master width WIDTH",
            ),
            ["master", "phase", left, right] => {
                if let (Some(left), Some(right)) = (parse_bool_flag(left), parse_bool_flag(right)) {
                    self.upsert_master_dsp_device(EffectDevice::phase_invert(5, left, right));
                } else {
                    self.notify_warning("Usage: :dsp master phase LEFT RIGHT");
                }
            }
            ["master", "filter", mode, cutoff, resonance, drive, mix] => {
                if let Some(device) = parse_filter_device(mode, cutoff, resonance, drive, mix) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master filter MODE CUTOFF RES DRIVE MIX");
                }
            }
            ["master", "delay", sync, left, right, feedback, mix] => {
                if let Some(device) = parse_delay_device(sync, left, right, feedback, mix, None) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp master delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping]",
                    );
                }
            }
            ["master", "delay", sync, left, right, feedback, mix, ping] => {
                if let Some(device) = parse_delay_device(sync, left, right, feedback, mix, Some(ping)) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp master delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping]",
                    );
                }
            }
            ["master", "reverb", size, predelay, decay, mix] => {
                if let Some(device) = parse_reverb_device(size, predelay, decay, mix) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master reverb SIZE PREDELAY_MS DECAY_S MIX");
                }
            }
            ["master", "drive", mode, drive, tone, mix] => {
                if let Some(device) = parse_drive_device(mode, drive, tone, mix) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master drive MODE DRIVE_DB TONE MIX");
                }
            }
            ["master", "bitcrusher" | "crusher", bit_depth, reduction, mix] => {
                if let Some(device) = parse_bitcrusher_device(bit_depth, reduction, mix, None) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp master bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither]",
                    );
                }
            }
            ["master", "bitcrusher" | "crusher", bit_depth, reduction, mix, dither] => {
                if let Some(device) =
                    parse_bitcrusher_device(bit_depth, reduction, mix, Some(dither))
                {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp master bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither]",
                    );
                }
            }
            ["master", "chorus", rate, depth, delay, voices, spread, mix] => {
                if let Some(device) = parse_chorus_device(rate, depth, delay, voices, spread, mix) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master chorus RATE_HZ DEPTH DELAY_MS VOICES SPREAD MIX");
                }
            }
            ["master", "flanger", rate, depth, manual, feedback, phase, mix] => {
                if let Some(device) =
                    parse_flanger_device(rate, depth, manual, feedback, phase, mix)
                {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master flanger RATE_HZ DEPTH MANUAL FEEDBACK PHASE MIX");
                }
            }
            ["master", "phaser", rate, depth, center, stages, feedback, phase, mix] => {
                if let Some(device) =
                    parse_phaser_device(rate, depth, center, stages, feedback, phase, mix)
                {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master phaser RATE_HZ DEPTH CENTER_HZ STAGES FEEDBACK PHASE MIX");
                }
            }
            ["master", "compressor", threshold, ratio, attack, release, knee, makeup, mix] => {
                if let Some(device) =
                    parse_compressor_device(threshold, ratio, attack, release, knee, makeup, mix)
                {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master compressor THRESHOLD_DB RATIO ATTACK_MS RELEASE_MS KNEE_DB MAKEUP_DB MIX");
                }
            }
            ["master", "gate", threshold, hysteresis, attack, hold, release, range] => {
                if let Some(device) =
                    parse_gate_device(threshold, hysteresis, attack, hold, release, range)
                {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master gate THRESHOLD_DB HYSTERESIS_DB ATTACK_MS HOLD_MS RELEASE_MS RANGE_DB");
                }
            }
            ["master", "limiter", ceiling, input, release, lookahead] => {
                if let Some(device) = parse_limiter_device(ceiling, input, release, lookahead) {
                    self.upsert_master_dsp_device(device);
                } else {
                    self.notify_warning("Usage: :dsp master limiter CEILING_DB INPUT_GAIN_DB RELEASE_MS LOOKAHEAD_MS");
                }
            }
            ["track", "clear"] => self.clear_track_dsp_chain(self.cursor.track),
            ["track", "gain", value] => self.set_current_track_dsp_value(
                value,
                |gain| EffectDevice::gain(1, gain),
                "Usage: :dsp track [TRACK] gain GAIN",
            ),
            ["track", "pan", value] => self.set_current_track_dsp_value(
                value,
                |pan| EffectDevice::pan(2, pan),
                "Usage: :dsp track [TRACK] pan PAN",
            ),
            ["track", "balance" | "bal", value] => self.set_current_track_dsp_value(
                value,
                |balance| EffectDevice::balance(3, balance),
                "Usage: :dsp track [TRACK] balance BALANCE",
            ),
            ["track", "width" | "stereo-width", value] => self.set_current_track_dsp_value(
                value,
                |width| EffectDevice::stereo_width(4, width),
                "Usage: :dsp track [TRACK] width WIDTH",
            ),
            ["track", "phase", left, right] => {
                if let (Some(left), Some(right)) = (parse_bool_flag(left), parse_bool_flag(right)) {
                    self.upsert_track_dsp_device(
                        self.cursor.track,
                        EffectDevice::phase_invert(5, left, right),
                    );
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] phase LEFT RIGHT");
                }
            }
            ["track", "filter", mode, cutoff, resonance, drive, mix] => {
                if let Some(device) = parse_filter_device(mode, cutoff, resonance, drive, mix) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] filter MODE CUTOFF RES DRIVE MIX");
                }
            }
            ["track", "delay", sync, left, right, feedback, mix] => {
                if let Some(device) = parse_delay_device(sync, left, right, feedback, mix, None) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping]",
                    );
                }
            }
            ["track", "delay", sync, left, right, feedback, mix, ping] => {
                if let Some(device) = parse_delay_device(sync, left, right, feedback, mix, Some(ping)) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping]",
                    );
                }
            }
            ["track", "reverb", size, predelay, decay, mix] => {
                if let Some(device) = parse_reverb_device(size, predelay, decay, mix) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] reverb SIZE PREDELAY_MS DECAY_S MIX");
                }
            }
            ["track", "drive", mode, drive, tone, mix] => {
                if let Some(device) = parse_drive_device(mode, drive, tone, mix) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] drive MODE DRIVE_DB TONE MIX");
                }
            }
            ["track", "bitcrusher" | "crusher", bit_depth, reduction, mix] => {
                if let Some(device) = parse_bitcrusher_device(bit_depth, reduction, mix, None) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither]",
                    );
                }
            }
            ["track", "bitcrusher" | "crusher", bit_depth, reduction, mix, dither] => {
                if let Some(device) =
                    parse_bitcrusher_device(bit_depth, reduction, mix, Some(dither))
                {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither]",
                    );
                }
            }
            ["track", "chorus", rate, depth, delay, voices, spread, mix] => {
                if let Some(device) = parse_chorus_device(rate, depth, delay, voices, spread, mix) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] chorus RATE_HZ DEPTH DELAY_MS VOICES SPREAD MIX");
                }
            }
            ["track", "flanger", rate, depth, manual, feedback, phase, mix] => {
                if let Some(device) =
                    parse_flanger_device(rate, depth, manual, feedback, phase, mix)
                {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] flanger RATE_HZ DEPTH MANUAL FEEDBACK PHASE MIX");
                }
            }
            ["track", "phaser", rate, depth, center, stages, feedback, phase, mix] => {
                if let Some(device) =
                    parse_phaser_device(rate, depth, center, stages, feedback, phase, mix)
                {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] phaser RATE_HZ DEPTH CENTER_HZ STAGES FEEDBACK PHASE MIX");
                }
            }
            ["track", "compressor", threshold, ratio, attack, release, knee, makeup, mix] => {
                if let Some(device) =
                    parse_compressor_device(threshold, ratio, attack, release, knee, makeup, mix)
                {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] compressor THRESHOLD_DB RATIO ATTACK_MS RELEASE_MS KNEE_DB MAKEUP_DB MIX");
                }
            }
            ["track", "gate", threshold, hysteresis, attack, hold, release, range] => {
                if let Some(device) =
                    parse_gate_device(threshold, hysteresis, attack, hold, release, range)
                {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] gate THRESHOLD_DB HYSTERESIS_DB ATTACK_MS HOLD_MS RELEASE_MS RANGE_DB");
                }
            }
            ["track", "limiter", ceiling, input, release, lookahead] => {
                if let Some(device) = parse_limiter_device(ceiling, input, release, lookahead) {
                    self.upsert_track_dsp_device(self.cursor.track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] limiter CEILING_DB INPUT_GAIN_DB RELEASE_MS LOOKAHEAD_MS");
                }
            }
            ["track", track, "clear"] => {
                if let Some(track) = parse_track_number(track) {
                    self.clear_track_dsp_chain(track);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] clear");
                }
            }
            ["track", track, "gain", value] => self.set_numbered_track_dsp_value(
                track,
                value,
                |gain| EffectDevice::gain(1, gain),
                "Usage: :dsp track [TRACK] gain GAIN",
            ),
            ["track", track, "pan", value] => self.set_numbered_track_dsp_value(
                track,
                value,
                |pan| EffectDevice::pan(2, pan),
                "Usage: :dsp track [TRACK] pan PAN",
            ),
            ["track", track, "balance" | "bal", value] => self.set_numbered_track_dsp_value(
                track,
                value,
                |balance| EffectDevice::balance(3, balance),
                "Usage: :dsp track [TRACK] balance BALANCE",
            ),
            ["track", track, "width" | "stereo-width", value] => self.set_numbered_track_dsp_value(
                track,
                value,
                |width| EffectDevice::stereo_width(4, width),
                "Usage: :dsp track [TRACK] width WIDTH",
            ),
            ["track", track, "phase", left, right] => {
                let track = parse_track_number(track);
                let left = parse_bool_flag(left);
                let right = parse_bool_flag(right);
                if let (Some(track), Some(left), Some(right)) = (track, left, right) {
                    self.upsert_track_dsp_device(track, EffectDevice::phase_invert(5, left, right));
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] phase LEFT RIGHT");
                }
            }
            ["track", track, "filter", mode, cutoff, resonance, drive, mix] => {
                let track = parse_track_number(track);
                let device = parse_filter_device(mode, cutoff, resonance, drive, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] filter MODE CUTOFF RES DRIVE MIX");
                }
            }
            ["track", track, "delay", sync, left, right, feedback, mix] => {
                let track = parse_track_number(track);
                let device = parse_delay_device(sync, left, right, feedback, mix, None);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping]",
                    );
                }
            }
            ["track", track, "delay", sync, left, right, feedback, mix, ping] => {
                let track = parse_track_number(track);
                let device = parse_delay_device(sync, left, right, feedback, mix, Some(ping));
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping]",
                    );
                }
            }
            ["track", track, "reverb", size, predelay, decay, mix] => {
                let track = parse_track_number(track);
                let device = parse_reverb_device(size, predelay, decay, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] reverb SIZE PREDELAY_MS DECAY_S MIX");
                }
            }
            ["track", track, "drive", mode, drive, tone, mix] => {
                let track = parse_track_number(track);
                let device = parse_drive_device(mode, drive, tone, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] drive MODE DRIVE_DB TONE MIX");
                }
            }
            ["track", track, "bitcrusher" | "crusher", bit_depth, reduction, mix] => {
                let track = parse_track_number(track);
                let device = parse_bitcrusher_device(bit_depth, reduction, mix, None);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither]",
                    );
                }
            }
            ["track", track, "bitcrusher" | "crusher", bit_depth, reduction, mix, dither] => {
                let track = parse_track_number(track);
                let device = parse_bitcrusher_device(bit_depth, reduction, mix, Some(dither));
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning(
                        "Usage: :dsp track [TRACK] bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither]",
                    );
                }
            }
            ["track", track, "chorus", rate, depth, delay, voices, spread, mix] => {
                let track = parse_track_number(track);
                let device = parse_chorus_device(rate, depth, delay, voices, spread, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] chorus RATE_HZ DEPTH DELAY_MS VOICES SPREAD MIX");
                }
            }
            ["track", track, "flanger", rate, depth, manual, feedback, phase, mix] => {
                let track = parse_track_number(track);
                let device = parse_flanger_device(rate, depth, manual, feedback, phase, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] flanger RATE_HZ DEPTH MANUAL FEEDBACK PHASE MIX");
                }
            }
            ["track", track, "phaser", rate, depth, center, stages, feedback, phase, mix] => {
                let track = parse_track_number(track);
                let device = parse_phaser_device(rate, depth, center, stages, feedback, phase, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] phaser RATE_HZ DEPTH CENTER_HZ STAGES FEEDBACK PHASE MIX");
                }
            }
            ["track", track, "compressor", threshold, ratio, attack, release, knee, makeup, mix] => {
                let track = parse_track_number(track);
                let device =
                    parse_compressor_device(threshold, ratio, attack, release, knee, makeup, mix);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] compressor THRESHOLD_DB RATIO ATTACK_MS RELEASE_MS KNEE_DB MAKEUP_DB MIX");
                }
            }
            ["track", track, "gate", threshold, hysteresis, attack, hold, release, range] => {
                let track = parse_track_number(track);
                let device = parse_gate_device(threshold, hysteresis, attack, hold, release, range);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] gate THRESHOLD_DB HYSTERESIS_DB ATTACK_MS HOLD_MS RELEASE_MS RANGE_DB");
                }
            }
            ["track", track, "limiter", ceiling, input, release, lookahead] => {
                let track = parse_track_number(track);
                let device = parse_limiter_device(ceiling, input, release, lookahead);
                if let (Some(track), Some(device)) = (track, device) {
                    self.upsert_track_dsp_device(track, device);
                } else {
                    self.notify_warning("Usage: :dsp track [TRACK] limiter CEILING_DB INPUT_GAIN_DB RELEASE_MS LOOKAHEAD_MS");
                }
            }
            _ => self.notify_warning(
                "Usage: :dsp master gain|pan|balance|width VALUE | phase LEFT RIGHT | filter MODE CUTOFF RES DRIVE MIX | delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping] | reverb SIZE PREDELAY_MS DECAY_S MIX | drive MODE DRIVE_DB TONE MIX | bitcrusher BIT_DEPTH REDUCTION_RATIO MIX [dither] | chorus|flanger|phaser|compressor|gate|limiter ... | :dsp track [TRACK] ... | :dsp ... clear",
            ),
        }
    }

    fn set_master_dsp_value(
        &mut self,
        value: &str,
        make_device: impl FnOnce(f32) -> EffectDevice,
        usage: &str,
    ) {
        if let Ok(value) = value.parse::<f32>() {
            self.upsert_master_dsp_device(make_device(value));
        } else {
            self.notify_warning(usage);
        }
    }

    fn set_current_track_dsp_value(
        &mut self,
        value: &str,
        make_device: impl FnOnce(f32) -> EffectDevice,
        usage: &str,
    ) {
        if let Ok(value) = value.parse::<f32>() {
            self.upsert_track_dsp_device(self.cursor.track, make_device(value));
        } else {
            self.notify_warning(usage);
        }
    }

    fn set_numbered_track_dsp_value(
        &mut self,
        track: &str,
        value: &str,
        make_device: impl FnOnce(f32) -> EffectDevice,
        usage: &str,
    ) {
        let track = parse_track_number(track);
        let value = value.parse::<f32>().ok();
        if let (Some(track), Some(value)) = (track, value) {
            self.upsert_track_dsp_device(track, make_device(value));
        } else {
            self.notify_warning(usage);
        }
    }
}

fn parse_filter_device(
    mode: &str,
    cutoff: &str,
    resonance: &str,
    drive: &str,
    mix: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::filter(
        6,
        FilterSpec {
            mode: parse_filter_mode(mode)?,
            cutoff_hz: cutoff.parse::<f32>().ok()?,
            resonance: resonance.parse::<f32>().ok()?,
            drive_db: drive.parse::<f32>().ok()?,
            key_track: 0.0,
            env_amount: 0.0,
            mix: mix.parse::<f32>().ok()?,
        },
    ))
}

fn parse_bool_flag(input: &str) -> Option<bool> {
    match input.to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" | "ping" | "pong" | "ping-pong" | "ping_pong" => Some(true),
        "0" | "off" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn parse_delay_device(
    sync: &str,
    left: &str,
    right: &str,
    feedback: &str,
    mix: &str,
    ping: Option<&&str>,
) -> Option<EffectDevice> {
    let sync = parse_sync_flag(sync)?;
    let mut spec = DelaySpec {
        sync,
        time_left_ms: left.parse::<f32>().ok()?,
        time_right_ms: right.parse::<f32>().ok()?,
        feedback: feedback.parse::<f32>().ok()?,
        mix: mix.parse::<f32>().ok()?,
        ..DelaySpec::default()
    };
    spec.link_times = (spec.time_left_ms - spec.time_right_ms).abs() < f32::EPSILON;
    if let Some(ping) = ping {
        spec.ping_pong = parse_bool_flag(ping)?;
    }
    Some(EffectDevice::delay(7, spec))
}

fn parse_reverb_device(size: &str, predelay: &str, decay: &str, mix: &str) -> Option<EffectDevice> {
    Some(EffectDevice::reverb(
        8,
        ReverbSpec {
            size: size.parse::<f32>().ok()?,
            predelay_ms: predelay.parse::<f32>().ok()?,
            decay_s: decay.parse::<f32>().ok()?,
            mix: mix.parse::<f32>().ok()?,
            ..ReverbSpec::default()
        },
    ))
}

fn parse_drive_device(mode: &str, drive: &str, tone: &str, mix: &str) -> Option<EffectDevice> {
    Some(EffectDevice::drive(
        9,
        DriveSpec {
            mode: parse_drive_mode(mode)?,
            drive_db: drive.parse::<f32>().ok()?,
            tone: tone.parse::<f32>().ok()?,
            mix: mix.parse::<f32>().ok()?,
            ..DriveSpec::default()
        },
    ))
}

fn parse_bitcrusher_device(
    bit_depth: &str,
    reduction: &str,
    mix: &str,
    dither: Option<&&str>,
) -> Option<EffectDevice> {
    let mut spec = BitcrusherSpec {
        bit_depth: bit_depth.parse::<u8>().ok()?,
        reduction_ratio: reduction.parse::<f32>().ok()?,
        mix: mix.parse::<f32>().ok()?,
        ..BitcrusherSpec::default()
    };
    if let Some(dither) = dither {
        spec.dither = parse_bool_flag(dither)?;
    }
    Some(EffectDevice::bitcrusher(10, spec))
}

fn parse_chorus_device(
    rate: &str,
    depth: &str,
    delay: &str,
    voices: &str,
    spread: &str,
    mix: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::chorus(
        11,
        ChorusSpec {
            rate_hz: rate.parse::<f32>().ok()?,
            depth: depth.parse::<f32>().ok()?,
            delay_ms: delay.parse::<f32>().ok()?,
            voices: voices.parse::<u8>().ok()?,
            spread: spread.parse::<f32>().ok()?,
            mix: mix.parse::<f32>().ok()?,
            ..ChorusSpec::default()
        },
    ))
}

fn parse_flanger_device(
    rate: &str,
    depth: &str,
    manual: &str,
    feedback: &str,
    phase: &str,
    mix: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::flanger(
        12,
        FlangerSpec {
            rate_hz: rate.parse::<f32>().ok()?,
            depth: depth.parse::<f32>().ok()?,
            manual: manual.parse::<f32>().ok()?,
            feedback: feedback.parse::<f32>().ok()?,
            stereo_phase: phase.parse::<f32>().ok()?,
            mix: mix.parse::<f32>().ok()?,
            ..FlangerSpec::default()
        },
    ))
}

fn parse_phaser_device(
    rate: &str,
    depth: &str,
    center: &str,
    stages: &str,
    feedback: &str,
    phase: &str,
    mix: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::phaser(
        13,
        PhaserSpec {
            rate_hz: rate.parse::<f32>().ok()?,
            depth: depth.parse::<f32>().ok()?,
            center_hz: center.parse::<f32>().ok()?,
            stages: stages.parse::<u8>().ok()?,
            feedback: feedback.parse::<f32>().ok()?,
            stereo_phase: phase.parse::<f32>().ok()?,
            mix: mix.parse::<f32>().ok()?,
            ..PhaserSpec::default()
        },
    ))
}

fn parse_compressor_device(
    threshold: &str,
    ratio: &str,
    attack: &str,
    release: &str,
    knee: &str,
    makeup: &str,
    mix: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::compressor(
        14,
        CompressorSpec {
            threshold_db: threshold.parse::<f32>().ok()?,
            ratio: ratio.parse::<f32>().ok()?,
            attack_ms: attack.parse::<f32>().ok()?,
            release_ms: release.parse::<f32>().ok()?,
            knee_db: knee.parse::<f32>().ok()?,
            makeup_db: makeup.parse::<f32>().ok()?,
            mix: mix.parse::<f32>().ok()?,
            ..CompressorSpec::default()
        },
    ))
}

fn parse_gate_device(
    threshold: &str,
    hysteresis: &str,
    attack: &str,
    hold: &str,
    release: &str,
    range: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::gate(
        15,
        GateSpec {
            threshold_db: threshold.parse::<f32>().ok()?,
            hysteresis_db: hysteresis.parse::<f32>().ok()?,
            attack_ms: attack.parse::<f32>().ok()?,
            hold_ms: hold.parse::<f32>().ok()?,
            release_ms: release.parse::<f32>().ok()?,
            range_db: range.parse::<f32>().ok()?,
            ..GateSpec::default()
        },
    ))
}

fn parse_limiter_device(
    ceiling: &str,
    input: &str,
    release: &str,
    lookahead: &str,
) -> Option<EffectDevice> {
    Some(EffectDevice::limiter(
        16,
        LimiterSpec {
            ceiling_db: ceiling.parse::<f32>().ok()?,
            input_gain_db: input.parse::<f32>().ok()?,
            release_ms: release.parse::<f32>().ok()?,
            lookahead_ms: lookahead.parse::<f32>().ok()?,
            ..LimiterSpec::default()
        },
    ))
}

fn parse_sync_flag(input: &str) -> Option<bool> {
    match input.to_ascii_lowercase().as_str() {
        "sync" | "synced" | "tempo" => Some(true),
        "free" | "ms" | "time" => Some(false),
        _ => parse_bool_flag(input),
    }
}

fn parse_filter_mode(input: &str) -> Option<FilterMode> {
    match input.to_ascii_lowercase().replace('-', "_").as_str() {
        "lp" | "low" | "low_pass" | "lowpass" => Some(FilterMode::LowPass),
        "hp" | "high" | "high_pass" | "highpass" => Some(FilterMode::HighPass),
        "bp" | "band" | "band_pass" | "bandpass" => Some(FilterMode::BandPass),
        "notch" | "br" | "band_reject" | "bandreject" => Some(FilterMode::Notch),
        _ => None,
    }
}

fn parse_drive_mode(input: &str) -> Option<DriveMode> {
    match input.to_ascii_lowercase().replace('-', "_").as_str() {
        "overdrive" | "od" => Some(DriveMode::Overdrive),
        "saturation" | "sat" => Some(DriveMode::Saturation),
        "hard_clip" | "hardclip" | "clip" | "distortion" | "dist" => Some(DriveMode::HardClip),
        "soft_clip" | "softclip" | "soft" => Some(DriveMode::SoftClip),
        _ => None,
    }
}
