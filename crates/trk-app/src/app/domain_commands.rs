use super::*;

impl App {
    pub(crate) fn handle_fx_command(&mut self, values: &[&str]) {
        match values {
            ["clear"] | ["off"] | ["none"] => {
                self.set_current_fx(None);
                self.notify_success("Effect cleared");
            }
            [packed] if packed.len() >= 2 => {
                let mut chars = packed.chars();
                let Some(code) = chars.next() else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                    return;
                };
                let value = chars.collect::<String>();
                if let Some(value) = parse_hex_byte(&value) {
                    let Some(command) = self.parse_current_fx(code, value) else {
                        return;
                    };
                    self.set_current_fx(Some(command));
                    self.notify_success(format!("Effect {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                }
            }
            [code, value] => {
                let Some(code) = code.chars().next() else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                    return;
                };
                if let Some(value) = parse_hex_byte(value) {
                    let Some(command) = self.parse_current_fx(code, value) else {
                        return;
                    };
                    self.set_current_fx(Some(command));
                    self.notify_success(format!("Effect {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                }
            }
            _ => self.notify_warning("Usage: :fx CODE VALUE or :fx clear"),
        }
    }

    pub(crate) fn handle_fx2_command(&mut self, values: &[&str]) {
        match values {
            ["clear"] | ["off"] | ["none"] => {
                self.set_current_fx_slot(None, CellField::Effect2);
                self.notify_success("FX2 cleared");
            }
            [packed] if packed.len() >= 2 => {
                let mut chars = packed.chars();
                let Some(code) = chars.next() else {
                    self.notify_warning("Usage: :fx2 CODE VALUE");
                    return;
                };
                let value = chars.collect::<String>();
                if let Some(value) = parse_hex_byte(&value) {
                    let Some(command) = self.parse_current_fx(code, value) else {
                        return;
                    };
                    self.set_current_fx_slot(Some(command), CellField::Effect2);
                    self.notify_success(format!("FX2 {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :fx2 CODE VALUE");
                }
            }
            [code, value] => {
                let Some(code) = code.chars().next() else {
                    self.notify_warning("Usage: :fx2 CODE VALUE");
                    return;
                };
                if let Some(value) = parse_hex_byte(value) {
                    let Some(command) = self.parse_current_fx(code, value) else {
                        return;
                    };
                    self.set_current_fx_slot(Some(command), CellField::Effect2);
                    self.notify_success(format!("FX2 {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :fx2 CODE VALUE");
                }
            }
            _ => self.notify_warning("Usage: :fx2 CODE VALUE or :fx2 clear"),
        }
    }

    pub(crate) fn set_current_fx(&mut self, command: Option<TrackerCommand>) {
        let field = self.cursor.field;
        self.set_current_fx_slot(command, field);
    }

    pub(crate) fn set_current_fx_slot(
        &mut self,
        command: Option<TrackerCommand>,
        field: CellField,
    ) {
        self.mutate_song(|song, cursor| {
            if let Some(pattern) = song.current_pattern_mut() {
                if let Some(cell) = pattern.cell_mut(cursor.row, cursor.track) {
                    if field == CellField::Effect2 {
                        cell.command2 = command;
                    } else {
                        cell.command = command;
                    }
                }
            }
        });
    }

    fn parse_current_fx(&mut self, code: char, value: u8) -> Option<TrackerCommand> {
        match parse_tracker_command(code, value) {
            Ok(command) => Some(command),
            Err(error) => {
                self.notify_warning(error.to_string());
                None
            }
        }
    }

    pub(crate) fn handle_cell_command(&mut self, values: &[&str]) {
        match values {
            ["instrument" | "inst", "clear" | "off" | "none"] => {
                self.set_current_cell_field(|cell| cell.instrument = None);
                self.notify_success("Instrument column cleared");
            }
            ["instrument" | "inst", value] => {
                if let Some(value) = parse_cell_byte(value) {
                    if value == 0 {
                        self.set_current_cell_field(|cell| cell.instrument = None);
                        self.notify_success("Instrument column cleared");
                    } else {
                        self.set_current_cell_field(|cell| {
                            cell.instrument = Some(InstrumentId(u32::from(value)))
                        });
                        self.notify_success(format!("Instrument column {value:02X}"));
                    }
                } else {
                    self.notify_warning("Usage: :cell instrument HEX|clear");
                }
            }
            ["volume" | "vol", "clear" | "off" | "none"] => {
                self.set_current_cell_field(|cell| cell.volume = None);
                self.notify_success("Volume column cleared");
            }
            ["volume" | "vol", value] => {
                if let Some(value) = parse_cell_byte(value) {
                    self.set_current_cell_field(|cell| cell.volume = Some(value.min(0x7f)));
                    self.notify_success(format!("Volume column {:02X}", value.min(0x7f)));
                } else {
                    self.notify_warning("Usage: :cell volume HEX|clear");
                }
            }
            ["pan", "clear" | "off" | "none"] => {
                self.set_current_cell_field(|cell| cell.pan = None);
                self.notify_success("Pan column cleared");
            }
            ["pan", value] => {
                if let Some(value) = parse_cell_byte(value) {
                    self.set_current_cell_field(|cell| cell.pan = Some(value.min(0x7f)));
                    self.notify_success(format!("Pan column {:02X}", value.min(0x7f)));
                } else {
                    self.notify_warning("Usage: :cell pan HEX|clear");
                }
            }
            ["delay" | "dly", "clear" | "off" | "none"] => {
                self.set_current_cell_field(|cell| cell.delay = None);
                self.notify_success("Delay column cleared");
            }
            ["delay" | "dly", value] => {
                if let Some(value) = parse_cell_byte(value) {
                    self.set_current_cell_field(|cell| cell.delay = Some(value));
                    self.notify_success(format!("Delay column {value:02X}"));
                } else {
                    self.notify_warning("Usage: :cell delay HEX|clear");
                }
            }
            ["effect" | "fx", "clear" | "off" | "none"] => {
                self.set_current_cell_field(|cell| cell.command = None);
                self.notify_success("FX1 column cleared");
            }
            ["effect2" | "fx2", "clear" | "off" | "none"] => {
                self.set_current_cell_field(|cell| cell.command2 = None);
                self.notify_success("FX2 column cleared");
            }
            ["effect" | "fx", code, value] => {
                let Some(code) = code.chars().next() else {
                    self.notify_warning("Usage: :cell effect CODE HEX");
                    return;
                };
                if let Some(value) = parse_cell_byte(value) {
                    let Some(command) = self.parse_current_fx(code, value) else {
                        return;
                    };
                    self.set_current_cell_field(|cell| {
                        cell.command = Some(command);
                    });
                    self.notify_success(format!("FX1 {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :cell effect CODE HEX");
                }
            }
            ["effect2" | "fx2", code, value] => {
                let Some(code) = code.chars().next() else {
                    self.notify_warning("Usage: :cell effect2 CODE HEX");
                    return;
                };
                if let Some(value) = parse_cell_byte(value) {
                    let Some(command) = self.parse_current_fx(code, value) else {
                        return;
                    };
                    self.set_current_cell_field(|cell| {
                        cell.command2 = Some(command);
                    });
                    self.notify_success(format!("FX2 {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :cell effect2 CODE HEX");
                }
            }
            _ => self.notify_warning(
                "Usage: :cell instrument|volume|pan|delay|effect|effect2 VALUE or :cell FIELD clear",
            ),
        }
    }

    pub(crate) fn set_current_cell_field(&mut self, mut edit: impl FnMut(&mut PatternCell)) {
        self.mutate_song(|song, cursor| {
            if let Some(pattern) = song.current_pattern_mut() {
                if let Some(cell) = pattern.cell_mut(cursor.row, cursor.track) {
                    edit(cell);
                }
            }
        });
    }

    pub(crate) fn handle_automation_command(&mut self, values: &[&str]) {
        match values {
            ["sample-gain", "clear"] => {
                self.clear_sample_gain_automation(self.cursor.track, self.cursor.row);
            }
            ["sample-gain", "clear", row] => {
                if let Ok(row) = row.parse::<usize>() {
                    self.clear_sample_gain_automation(self.cursor.track, row);
                } else {
                    self.notify_warning("Usage: :automation sample-gain [ROW] VALUE");
                }
            }
            ["sample-gain", value] => {
                if let Ok(value) = value.parse::<f32>() {
                    self.set_sample_gain_automation(self.cursor.track, self.cursor.row, value);
                } else {
                    self.notify_warning("Usage: :automation sample-gain [ROW] VALUE");
                }
            }
            ["sample-gain", row, value] => {
                let row = row.parse::<usize>().ok();
                let value = value.parse::<f32>().ok();
                if let (Some(row), Some(value)) = (row, value) {
                    self.set_sample_gain_automation(self.cursor.track, row, value);
                } else {
                    self.notify_warning("Usage: :automation sample-gain [ROW] VALUE");
                }
            }
            _ => self.notify_warning(
                "Usage: :automation sample-gain [ROW] VALUE or :automation sample-gain clear [ROW]",
            ),
        }
    }

    pub(crate) fn set_sample_gain_automation(
        &mut self,
        track_index: usize,
        row: usize,
        value: f32,
    ) {
        if !value.is_finite() || value < 0.0 {
            self.notify_warning("Automation value must be a non-negative number");
            return;
        }
        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        let Some(sample) = self.song.sample_for_track(track.id) else {
            self.notify_warning("Assign a sample to the track before automating sample gain");
            return;
        };
        let sample_id = sample.id;
        let sample_name = sample.name.clone();

        let result = self.try_mutate_song(
            TransactionSpec::merged(
                "Adjust sample automation",
                format!("automation.sample-gain.{sample_id:?}.{row}"),
            ),
            |song, _| {
                if let Some(pattern) = song.current_pattern_mut() {
                    pattern.set_automation_point(
                        AutomationTarget::SampleGain { sample: sample_id },
                        row,
                        value,
                    )?;
                }
                Ok::<(), trk_core::EditError>(())
            },
        );
        match result {
            Ok(_) => self.notify_success(format!(
                "Sample gain automation {sample_name} row {row:02} = {value:.3}"
            )),
            Err(error) => self.notify_warning(format!("Automation failed: {error}")),
        }
    }

    pub(crate) fn clear_sample_gain_automation(&mut self, track_index: usize, row: usize) {
        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        let Some(sample) = self.song.sample_for_track(track.id) else {
            self.notify_warning("Assign a sample to the track before editing automation");
            return;
        };
        let sample_id = sample.id;
        let sample_name = sample.name.clone();

        let result = self.try_mutate_song(
            TransactionSpec::new("Clear sample automation"),
            |song, _| {
                if let Some(pattern) = song.current_pattern_mut() {
                    pattern.clear_automation_point(
                        AutomationTarget::SampleGain { sample: sample_id },
                        row,
                    )?;
                }
                Ok::<(), trk_core::EditError>(())
            },
        );
        match result {
            Ok(_) => self.notify_success(format!(
                "Sample gain automation cleared for {sample_name} row {row:02}"
            )),
            Err(error) => self.notify_warning(format!("Automation failed: {error}")),
        }
    }

    pub(crate) fn handle_parameter_lock_command(&mut self, values: &[&str]) {
        let Some(edit) = self.parse_parameter_lock_edit(values) else {
            self.notify_warning(
                "Usage: :plock sample-gain|mixer gain|mixer pan|master gain|send SEND|dsp track gain|pan|balance|width|phase-left|phase-right|filter-*|delay-* VALUE|reset|clear",
            );
            return;
        };
        match edit {
            ParameterLockEdit::Set { lock, descriptor } => {
                self.set_current_parameter_lock(lock, descriptor.as_ref());
            }
            ParameterLockEdit::Clear { target, parameter } => {
                self.clear_current_parameter_lock(target, parameter);
            }
        }
    }

    fn parse_parameter_lock_edit(&self, values: &[&str]) -> Option<ParameterLockEdit> {
        match values {
            ["sample-gain", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                let sample = self.song.sample_for_track(track.id)?;
                parameter_lock_edit(
                    ParameterLockTarget::Sample { sample: sample.id },
                    SAMPLE_GAIN_PARAMETER_ID,
                    sample_gain_descriptor(),
                    action,
                )
            }
            ["mixer", "gain", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackMixer { track: track.id },
                    MIXER_TRACK_GAIN_PARAMETER_ID,
                    mixer_track_gain_descriptor(),
                    action,
                )
            }
            ["mixer", "pan", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackMixer { track: track.id },
                    MIXER_TRACK_PAN_PARAMETER_ID,
                    mixer_track_pan_descriptor(),
                    action,
                )
            }
            ["master", "gain", action] => parameter_lock_edit(
                ParameterLockTarget::MasterMixer,
                MIXER_MASTER_GAIN_PARAMETER_ID,
                mixer_master_gain_descriptor(),
                action,
            ),
            ["send", send, action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                let send = send.parse::<u32>().ok()?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackSend {
                        track: track.id,
                        send,
                    },
                    MIXER_SEND_GAIN_PARAMETER_ID,
                    mixer_send_gain_descriptor(),
                    action,
                )
            }
            ["dsp", ..] => self.parse_dsp_parameter_lock_edit(values),
            _ => None,
        }
    }

    pub(crate) fn set_current_parameter_lock(
        &mut self,
        lock: ParameterLock,
        descriptor: &ParameterDescriptor,
    ) {
        let row = self.cursor.row;
        let track = self.cursor.track;
        let label = format_parameter_lock_target(&lock.target);
        let value_label = match &lock.action {
            ParameterLockAction::Set { value } => descriptor.format_value(value),
            ParameterLockAction::Reset => "reset".to_string(),
        };
        let result = self.try_mutate_song(
            TransactionSpec::merged(
                "Adjust parameter lock",
                format!("parameter-lock.{row}.{track}.{}", lock.parameter),
            ),
            |song, cursor| {
                song.validate_parameter_lock(&lock)
                    .map_err(|error| error.to_string())?;
                if let Some(pattern) = song.current_pattern_mut() {
                    pattern
                        .set_parameter_lock(cursor.row, cursor.track, lock)
                        .map_err(|error| error.to_string())?;
                }
                Ok::<(), String>(())
            },
        );
        match result {
            Ok(_) => self.notify_success(format!(
                "Parameter lock {label} {} = {value_label}",
                descriptor.name
            )),
            Err(error) => self.notify_warning(format!("Parameter lock failed: {error}")),
        }
    }

    pub(crate) fn clear_current_parameter_lock(
        &mut self,
        target: ParameterLockTarget,
        parameter: ParameterId,
    ) {
        let label = format_parameter_lock_target(&target);
        let result = self.try_mutate_song(
            TransactionSpec::new("Clear parameter lock"),
            |song, cursor| {
                if let Some(pattern) = song.current_pattern_mut() {
                    pattern.clear_parameter_lock(cursor.row, cursor.track, &target, &parameter)?;
                }
                Ok::<(), trk_core::EditError>(())
            },
        );
        match result {
            Ok(_) => self.notify_success(format!("Parameter lock cleared {label} {parameter}")),
            Err(error) => self.notify_warning(format!("Parameter lock failed: {error}")),
        }
    }

    pub(crate) fn handle_mixer_command(&mut self, values: &[&str]) {
        match values {
            ["master", gain] => {
                if let Ok(gain) = gain.parse::<f32>() {
                    self.set_master_gain(gain);
                } else {
                    self.notify_warning("Usage: :mixer master GAIN");
                }
            }
            ["gain", value] => {
                if let Ok(gain) = value.parse::<f32>() {
                    self.set_track_mixer_gain(self.cursor.track, gain);
                } else {
                    self.notify_warning("Usage: :mixer gain [TRACK] GAIN");
                }
            }
            ["gain", track, value] => {
                let track = track.parse::<usize>().ok().map(|value| value.saturating_sub(1));
                let gain = value.parse::<f32>().ok();
                if let (Some(track), Some(gain)) = (track, gain) {
                    self.set_track_mixer_gain(track, gain);
                } else {
                    self.notify_warning("Usage: :mixer gain [TRACK] GAIN");
                }
            }
            ["pan", value] => {
                if let Ok(pan) = value.parse::<f32>() {
                    self.set_track_mixer_pan(self.cursor.track, pan);
                } else {
                    self.notify_warning("Usage: :mixer pan [TRACK] PAN");
                }
            }
            ["pan", track, value] => {
                let track = track.parse::<usize>().ok().map(|value| value.saturating_sub(1));
                let pan = value.parse::<f32>().ok();
                if let (Some(track), Some(pan)) = (track, pan) {
                    self.set_track_mixer_pan(track, pan);
                } else {
                    self.notify_warning("Usage: :mixer pan [TRACK] PAN");
                }
            }
            ["mute"] => self.toggle_track_mixer_mute(self.cursor.track),
            ["mute", track] => {
                if let Some(track) = parse_track_number(track) {
                    self.toggle_track_mixer_mute(track);
                } else {
                    self.notify_warning("Usage: :mixer mute [TRACK]");
                }
            }
            ["solo"] => self.toggle_track_mixer_solo(self.cursor.track),
            ["solo", track] => {
                if let Some(track) = parse_track_number(track) {
                    self.toggle_track_mixer_solo(track);
                } else {
                    self.notify_warning("Usage: :mixer solo [TRACK]");
                }
            }
            ["send", "list" | "status"] => self.show_mixer_sends(),
            ["send", "delay"] => self.ensure_mixer_send(
                1,
                "Delay",
                EffectDevice::delay(1, DelaySpec::default()),
            ),
            ["send", "reverb"] => self.ensure_mixer_send(
                2,
                "Reverb",
                EffectDevice::reverb(1, ReverbSpec::default()),
            ),
            ["send", send, "pre"] => self.set_mixer_send_mode(send, true),
            ["send", send, "post"] => self.set_mixer_send_mode(send, false),
            ["send", send, "clear"] => self.clear_mixer_send(send),
            ["send", send, "gain", value] => {
                if let Ok(gain) = value.parse::<f32>() {
                    self.set_track_send_gain(send, self.cursor.track, gain);
                } else {
                    self.notify_warning("Usage: :mixer send SEND gain [TRACK] GAIN");
                }
            }
            ["send", send, "gain", track, value] => {
                let track = parse_track_number(track);
                let gain = value.parse::<f32>().ok();
                if let (Some(track), Some(gain)) = (track, gain) {
                    self.set_track_send_gain(send, track, gain);
                } else {
                    self.notify_warning("Usage: :mixer send SEND gain [TRACK] GAIN");
                }
            }
            _ => self.notify_warning(
                "Usage: :mixer master GAIN | gain [TRACK] GAIN | pan [TRACK] PAN | mute [TRACK] | solo [TRACK] | send delay|reverb|list | send SEND pre|post|clear|gain [TRACK] GAIN",
            ),
        }
    }

    pub(crate) fn set_track_mixer_gain(&mut self, track_index: usize, gain: f32) {
        let name = self
            .song
            .tracks
            .get(track_index)
            .map(|track| track.name.clone());
        let result = self.try_mutate_song(
            TransactionSpec::merged(
                "Adjust mixer gain",
                format!("mixer.track.{track_index}.gain"),
            ),
            |song, _| song.set_track_mixer_gain(track_index, gain),
        );
        match (result, name) {
            (Ok(_), Some(name)) => self.notify_success(format!("Mixer gain {name} = {gain:.2}")),
            (Err(error), _) => self.notify_warning(format!("Mixer failed: {error}")),
            (Ok(_), None) => self.notify_warning("Track out of range"),
        }
    }

    pub(crate) fn set_track_mixer_pan(&mut self, track_index: usize, pan: f32) {
        let name = self
            .song
            .tracks
            .get(track_index)
            .map(|track| track.name.clone());
        let result = self.try_mutate_song(
            TransactionSpec::merged("Adjust mixer pan", format!("mixer.track.{track_index}.pan")),
            |song, _| song.set_track_mixer_pan(track_index, pan),
        );
        match (result, name) {
            (Ok(_), Some(name)) => self.notify_success(format!("Mixer pan {name} = {pan:+.2}")),
            (Err(error), _) => self.notify_warning(format!("Mixer failed: {error}")),
            (Ok(_), None) => self.notify_warning("Track out of range"),
        }
    }

    pub(crate) fn toggle_track_mixer_mute(&mut self, track_index: usize) {
        let name = self
            .song
            .tracks
            .get(track_index)
            .map(|track| track.name.clone());
        let mut muted = false;
        let result = self.try_mutate_song(TransactionSpec::new("Toggle mixer mute"), |song, _| {
            song.toggle_track_mixer_mute(track_index)?;
            muted = song
                .tracks
                .get(track_index)
                .map(|track| song.track_mixer_for_track(track.id).muted)
                .unwrap_or(false);
            Ok::<(), trk_core::EditError>(())
        });
        match (result, name) {
            (Ok(_), Some(name)) => self.notify_success(format!(
                "Mixer mute {name} {}",
                if muted { "ON" } else { "OFF" }
            )),
            (Err(error), _) => self.notify_warning(format!("Mixer failed: {error}")),
            (Ok(_), None) => self.notify_warning("Track out of range"),
        }
    }

    pub(crate) fn toggle_track_mixer_solo(&mut self, track_index: usize) {
        let name = self
            .song
            .tracks
            .get(track_index)
            .map(|track| track.name.clone());
        let mut solo = false;
        let result = self.try_mutate_song(TransactionSpec::new("Toggle mixer solo"), |song, _| {
            song.toggle_track_mixer_solo(track_index)?;
            solo = song
                .tracks
                .get(track_index)
                .map(|track| song.track_mixer_for_track(track.id).solo)
                .unwrap_or(false);
            Ok::<(), trk_core::EditError>(())
        });
        match (result, name) {
            (Ok(_), Some(name)) => self.notify_success(format!(
                "Mixer solo {name} {}",
                if solo { "ON" } else { "OFF" }
            )),
            (Err(error), _) => self.notify_warning(format!("Mixer failed: {error}")),
            (Ok(_), None) => self.notify_warning("Track out of range"),
        }
    }

    pub(crate) fn set_master_gain(&mut self, gain: f32) {
        let result = self.try_mutate_song(
            TransactionSpec::merged("Adjust master gain", "mixer.master.gain"),
            |song, _| song.set_master_gain(gain),
        );
        match result {
            Ok(_) => self.notify_success(format!("Master gain = {gain:.2}")),
            Err(error) => self.notify_warning(format!("Mixer failed: {error}")),
        }
    }

    pub(crate) fn ensure_mixer_send(&mut self, id: u32, name: &str, device: EffectDevice) {
        if !effect_device_is_valid(&device) {
            self.notify_warning("Send DSP parameter out of range");
            return;
        }
        let name = name.to_string();
        self.mutate_song(|song, _| {
            if let Some(send) = song.mixer.sends.iter_mut().find(|send| send.id == id) {
                send.name = name.clone();
                upsert_effect_device(&mut send.effects, device.clone());
            } else {
                song.mixer.sends.push(MixerSend {
                    id,
                    name: name.clone(),
                    pre_fader: false,
                    effects: vec![device.clone()],
                });
            }
        });
        self.notify_success(format!("Mixer send {id} {name} ready"));
    }

    pub(crate) fn set_mixer_send_mode(&mut self, send: &str, pre_fader: bool) {
        let Some(send_id) = parse_send_id(send) else {
            self.notify_warning("Usage: :mixer send SEND pre|post");
            return;
        };
        let mut updated = false;
        self.mutate_song(|song, _| {
            if let Some(send) = song.mixer.sends.iter_mut().find(|send| send.id == send_id) {
                send.pre_fader = pre_fader;
                updated = true;
            }
        });
        if updated {
            self.notify_success(format!(
                "Mixer send {send_id} {}",
                if pre_fader { "pre-fader" } else { "post-fader" }
            ));
        } else {
            self.notify_warning("Mixer send not found");
        }
    }

    pub(crate) fn clear_mixer_send(&mut self, send: &str) {
        let Some(send_id) = parse_send_id(send) else {
            self.notify_warning("Usage: :mixer send SEND clear");
            return;
        };
        self.mutate_song(|song, _| {
            song.mixer.sends.retain(|send| send.id != send_id);
            for track in &mut song.mixer.tracks {
                track.sends.retain(|send| send.send != send_id);
            }
        });
        self.notify_success(format!("Mixer send {send_id} cleared"));
    }

    pub(crate) fn set_track_send_gain(&mut self, send: &str, track_index: usize, gain: f32) {
        let Some(send_id) = parse_send_id(send) else {
            self.notify_warning("Usage: :mixer send SEND gain [TRACK] GAIN");
            return;
        };
        if !mixer_send_gain_descriptor().validate_f32(gain) {
            self.notify_warning("Mixer send gain out of range");
            return;
        }
        let mut track_name = None;
        let mut updated = false;
        self.mutate_song(|song, _| {
            if !song.mixer.sends.iter().any(|send| send.id == send_id) {
                return;
            }
            let Some(track) = song.tracks.get(track_index) else {
                return;
            };
            let track_id = track.id;
            track_name = Some(track.name.clone());
            song.ensure_mixer_for_tracks();
            let Some(mixer) = song
                .mixer
                .tracks
                .iter_mut()
                .find(|mixer| mixer.track == track_id)
            else {
                return;
            };
            if let Some(send) = mixer.sends.iter_mut().find(|send| send.send == send_id) {
                send.gain = gain;
            } else {
                mixer.sends.push(TrackSendLevel {
                    send: send_id,
                    gain,
                });
            }
            updated = true;
        });
        if updated {
            self.notify_success(format!(
                "Mixer send {send_id} {} = {gain:.2}",
                track_name.unwrap_or_else(|| format!("{:02}", track_index + 1))
            ));
        } else {
            self.notify_warning("Mixer send or track not found");
        }
    }

    pub(crate) fn show_mixer_sends(&mut self) {
        if self.song.mixer.sends.is_empty() {
            self.notify_info("Mixer sends: none");
            return;
        }
        let sends = self
            .song
            .mixer
            .sends
            .iter()
            .map(|send| {
                let mode = if send.pre_fader { "pre" } else { "post" };
                let routed_tracks = self
                    .song
                    .mixer
                    .tracks
                    .iter()
                    .filter_map(|track| {
                        let level = track.sends.iter().find(|level| level.send == send.id)?;
                        if level.gain <= 0.0 {
                            return None;
                        }
                        Some(format!("T{}={:.2}", track.track.0, level.gain))
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let routed_tracks = if routed_tracks.is_empty() {
                    "no tracks".to_string()
                } else {
                    routed_tracks
                };
                format!(
                    "{}:{} {} {} devices {}",
                    send.id,
                    send.name,
                    mode,
                    send.effects.len(),
                    routed_tracks
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        self.notify_info(format!("Mixer sends: {sends}"));
    }

    pub(crate) fn upsert_master_dsp_device(&mut self, device: EffectDevice) {
        if !effect_device_is_valid(&device) {
            self.notify_warning("DSP parameter out of range");
            return;
        }
        let label = format_effect_device(&device);
        self.mutate_song(|song, _| {
            upsert_effect_device(&mut song.mixer.master_effects, device.clone());
        });
        self.notify_success(format!("Master DSP {label}"));
    }

    pub(crate) fn upsert_track_dsp_device(&mut self, track_index: usize, device: EffectDevice) {
        if !effect_device_is_valid(&device) {
            self.notify_warning("DSP parameter out of range");
            return;
        }
        let label = format_effect_device(&device);
        let mut track_name = None;
        let mut updated = false;
        self.mutate_song(|song, _| {
            let Some(track) = song.tracks.get(track_index) else {
                return;
            };
            let track_id = track.id;
            track_name = Some(track.name.clone());
            song.ensure_mixer_for_tracks();
            if let Some(mixer) = song
                .mixer
                .tracks
                .iter_mut()
                .find(|mixer| mixer.track == track_id)
            {
                upsert_effect_device(&mut mixer.effects, device.clone());
                updated = true;
            }
        });
        if updated {
            self.notify_success(format!(
                "Track DSP {} {label}",
                track_name.unwrap_or_else(|| format!("{:02}", track_index + 1))
            ));
        } else {
            self.notify_warning("Track out of range");
        }
    }

    pub(crate) fn clear_master_dsp_chain(&mut self) {
        self.mutate_song(|song, _| {
            song.mixer.master_effects.clear();
        });
        self.notify_success("Master DSP cleared");
    }

    pub(crate) fn clear_track_dsp_chain(&mut self, track_index: usize) {
        let mut updated = false;
        self.mutate_song(|song, _| {
            let Some(track) = song.tracks.get(track_index) else {
                return;
            };
            let track_id = track.id;
            song.ensure_mixer_for_tracks();
            if let Some(mixer) = song
                .mixer
                .tracks
                .iter_mut()
                .find(|mixer| mixer.track == track_id)
            {
                mixer.effects.clear();
                updated = true;
            }
        });
        if updated {
            self.notify_success("Track DSP cleared");
        } else {
            self.notify_warning("Track out of range");
        }
    }
}

pub(super) enum ParameterLockEdit {
    Set {
        lock: ParameterLock,
        descriptor: Box<ParameterDescriptor>,
    },
    Clear {
        target: ParameterLockTarget,
        parameter: ParameterId,
    },
}

pub(super) fn parameter_lock_edit(
    target: ParameterLockTarget,
    parameter: &str,
    descriptor: ParameterDescriptor,
    action: &str,
) -> Option<ParameterLockEdit> {
    let parameter = ParameterId::from(parameter);
    match action.to_ascii_lowercase().as_str() {
        "clear" | "off" | "none" => Some(ParameterLockEdit::Clear { target, parameter }),
        "reset" => Some(ParameterLockEdit::Set {
            lock: ParameterLock {
                target,
                parameter,
                action: ParameterLockAction::Reset,
            },
            descriptor: Box::new(descriptor),
        }),
        _ => parse_parameter_lock_value(&descriptor, action).map(|value| ParameterLockEdit::Set {
            lock: ParameterLock {
                target,
                parameter,
                action: ParameterLockAction::Set { value },
            },
            descriptor: Box::new(descriptor),
        }),
    }
}

fn parse_parameter_lock_value(
    descriptor: &ParameterDescriptor,
    input: &str,
) -> Option<trk_core::ParameterValue> {
    descriptor.parse_value(input).ok().or_else(|| {
        let value = descriptor.value_from_f32(input.parse::<f32>().ok()?);
        descriptor.validate(&value).ok()?;
        Some(value)
    })
}
