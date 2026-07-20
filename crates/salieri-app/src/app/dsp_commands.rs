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
            _ => self.notify_warning(
                "Usage: :dsp master gain|pan|balance|width VALUE | phase LEFT RIGHT | filter MODE CUTOFF RES DRIVE MIX | delay sync|free LEFT_MS RIGHT_MS FEEDBACK MIX [ping] | :dsp track [TRACK] ... | :dsp ... clear",
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
