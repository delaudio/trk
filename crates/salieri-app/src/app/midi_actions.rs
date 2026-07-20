use super::*;

impl App {
    pub(crate) fn refresh_midi_ports(&mut self) {
        match list_output_ports() {
            Ok(ports) => {
                self.midi_ports = ports;
                self.midi_port_cursor = self
                    .midi_port_cursor
                    .min(self.midi_ports.len().saturating_sub(1));
                if self.midi_ports.is_empty() {
                    self.midi_status = "MIDI No Outputs".to_string();
                    self.notify_warning("No MIDI output ports found");
                } else {
                    self.notify_info(format!("Found {} MIDI output(s)", self.midi_ports.len()));
                }
            }
            Err(error) => {
                self.midi_ports.clear();
                self.midi_port_cursor = 0;
                self.midi_status = format!("MIDI Error: {error}");
                self.notify_error(format!("MIDI output list failed: {error}"));
            }
        }
    }

    pub(crate) fn handle_midi_input_command(&mut self, values: &[&str]) {
        match values {
            ["ports"] | ["inputs"] => self.refresh_midi_input_ports(),
            ["settings"] | ["routing"] => self.show_midi_routing_settings(),
            ["connect", port] => {
                if let Ok(port_index) = port.parse::<usize>() {
                    self.connect_midi_input(port_index);
                } else {
                    self.notify_warning("Usage: :midi-input connect PORT_INDEX");
                }
            }
            ["disconnect"] => self.disconnect_midi_input(),
            ["record", "on"] | ["record", "arm"] => {
                self.midi_record_armed = true;
                self.notify_info("MIDI input record armed");
            }
            ["record", "off"] => {
                self.midi_record_armed = false;
                self.notify_info("MIDI input record off");
            }
            ["record", "notes", value] => {
                self.set_midi_recording_flag("notes", parse_on_off(value));
            }
            ["record", "velocity", value] | ["record", "vel", value] => {
                self.set_midi_recording_flag("velocity", parse_on_off(value));
            }
            ["record", "cc", value] => {
                self.set_midi_recording_flag("cc", parse_on_off(value));
            }
            ["clock", "on"] => {
                self.set_midi_clock_follow_legacy(true);
                self.midi_clock_ticks = 0;
            }
            ["clock", "off"] => {
                self.set_midi_clock_follow_legacy(false);
                self.midi_clock_ticks = 0;
            }
            ["clock", "in", value] => self.set_midi_routing_flag("clock in", parse_on_off(value)),
            ["clock", "out", value] => {
                self.set_midi_routing_flag("clock out", parse_on_off(value));
            }
            ["transport", "in", value] => {
                self.set_midi_routing_flag("transport in", parse_on_off(value));
            }
            ["transport", "out", value] => {
                self.set_midi_routing_flag("transport out", parse_on_off(value));
            }
            ["notes", "in", value] => self.set_midi_routing_flag("notes in", parse_on_off(value)),
            ["notes", "out", value] => {
                self.set_midi_routing_flag("notes out", parse_on_off(value));
            }
            ["cc", "in", value] => self.set_midi_routing_flag("cc in", parse_on_off(value)),
            ["cc", "out", value] => self.set_midi_routing_flag("cc out", parse_on_off(value)),
            ["channel", "in", channels @ ..] => self.set_midi_channel_filter("input", channels),
            ["channel", "out", channels @ ..] => self.set_midi_channel_filter("output", channels),
            ["middle-c", value] | ["middle", "c", value] => self.set_midi_middle_c(value),
            ["sync-delay", value] | ["clock-delay", value] => self.set_midi_clock_delay(value),
            _ => self.notify_warning(
                "Usage: :midi-input ports|connect PORT|disconnect|record on|off|record velocity on|off|clock|transport|notes|cc in|out on|off|channel in|out all|N..|middle-c NOTE|sync-delay MS",
            ),
        }
    }

    pub(crate) fn refresh_midi_input_ports(&mut self) {
        match list_input_ports() {
            Ok(ports) => {
                self.midi_input_ports = ports;
                if self.midi_input_ports.is_empty() {
                    self.midi_input_status = "MIDI In No Inputs".to_string();
                    self.notify_warning("No MIDI input ports found");
                } else {
                    self.notify_info(format!(
                        "Found {} MIDI input(s)",
                        self.midi_input_ports.len()
                    ));
                }
            }
            Err(error) => {
                self.midi_input_ports.clear();
                self.midi_input_status = format!("MIDI In Error: {error}");
                self.notify_error(format!("MIDI input list failed: {error}"));
            }
        }
    }

    pub(crate) fn connect_midi_input(&mut self, port_index: usize) {
        match MidirMidiInput::connect(port_index, "salieri-input") {
            Ok(input) => {
                self.midi_input = Some(AppMidiInput::new(input));
                self.midi_input_status = format!("MIDI In Connected {port_index}");
                self.notify_success(format!("MIDI input connected: {port_index}"));
            }
            Err(error) => {
                self.midi_input = None;
                self.midi_input_status = format!("MIDI In Error: {error}");
                self.notify_error(format!("MIDI input connect failed: {error}"));
            }
        }
    }

    pub(crate) fn connect_default_midi_input(&mut self, input_name: &str) {
        if input_name.trim().is_empty() {
            return;
        }

        match list_input_ports() {
            Ok(ports) => {
                self.midi_input_ports = ports;
                if let Some((_, port)) = resolve_midi_input_port(&self.midi_input_ports, input_name)
                {
                    let index = port.index;
                    let name = port.name.clone();
                    self.midi_input_status = format!("MIDI In Connecting {index} ({name})");
                    self.connect_midi_input(index);
                } else {
                    self.midi_input_status = format!("MIDI In Not Found ({input_name})");
                    self.notify_error(format!("MIDI input not found: {input_name}"));
                }
            }
            Err(error) => {
                self.midi_input_status = format!("MIDI In Error: {error}");
                self.notify_error(format!("MIDI input list failed: {error}"));
            }
        }
    }

    pub(crate) fn disconnect_midi_input(&mut self) {
        self.midi_input = None;
        self.midi_input_status = "MIDI In Disconnected".to_string();
        self.midi_record_armed = false;
        self.midi_clock_follow = false;
        self.midi_clock_ticks = 0;
        self.notify_info("MIDI input disconnected");
    }

    pub(crate) fn next_midi_port(&mut self) {
        self.midi_port_cursor = self
            .midi_port_cursor
            .saturating_add(1)
            .min(self.midi_ports.len().saturating_sub(1));
    }

    pub(crate) fn previous_midi_port(&mut self) {
        self.midi_port_cursor = self.midi_port_cursor.saturating_sub(1);
    }

    pub(crate) fn connect_selected_midi_port(&mut self) {
        if let Some(port) = self.midi_ports.get(self.midi_port_cursor) {
            self.connect_midi(port.index);
        } else {
            self.midi_status = "MIDI No Outputs".to_string();
            self.notify_warning("No MIDI output selected");
        }
    }

    pub(crate) fn connect_default_midi_output(&mut self, output_name: &str) {
        if output_name.trim().is_empty() {
            return;
        }

        match list_output_ports() {
            Ok(ports) => {
                self.midi_ports = ports;
                if let Some((position, port)) =
                    resolve_midi_output_port(&self.midi_ports, output_name)
                {
                    self.midi_port_cursor = position;
                    self.connect_midi(port.index);
                } else {
                    self.midi_status = format!("MIDI Output Not Found ({output_name})");
                    self.notify_error(format!("MIDI output not found: {output_name}"));
                }
            }
            Err(error) => {
                self.midi_status = format!("MIDI Error: {error}");
                self.notify_error(format!("MIDI output list failed: {error}"));
            }
        }
    }

    pub(crate) fn disconnect_midi(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::DisconnectMidi));
    }

    pub(crate) fn panic_midi(&mut self) {
        self.dispatch_intent(AppIntent::Transport(TransportIntent::PanicMidi));
    }

    pub(crate) fn drain_midi_input(&mut self) {
        let mut packets = Vec::new();
        if let Some(input) = &mut self.midi_input {
            loop {
                match input.poll() {
                    Ok(Some(packet)) => packets.push(packet),
                    Ok(None) => break,
                    Err(error) => {
                        self.dispatch_event(AppEvent::Runtime(RuntimeEvent::MidiInputFailed(
                            error.to_string(),
                        )));
                        break;
                    }
                }
            }
        }

        for packet in packets {
            self.handle_midi_input_packet(packet);
        }
    }

    pub(crate) fn handle_midi_input_packet(&mut self, packet: MidiInputPacket) {
        self.dispatch_event(AppEvent::Runtime(RuntimeEvent::MidiInput(packet)));
    }

    pub(crate) fn apply_midi_input_packet(&mut self, packet: MidiInputPacket) {
        match packet.event {
            MidiInputEvent::NoteOn {
                channel,
                note,
                velocity,
            } => {
                if self.song.midi.notes_in
                    && midi_channel_allowed(&self.song.midi.input_channels, channel)
                    && self.midi_record_armed
                    && self.song.midi.recording.notes
                {
                    self.record_midi_note(note, velocity);
                }
            }
            MidiInputEvent::Clock(message) => self.handle_midi_clock_message(message),
            MidiInputEvent::ControlChange { channel, .. } => {
                if self.song.midi.cc_in
                    && self.song.midi.recording.cc
                    && midi_channel_allowed(&self.song.midi.input_channels, channel)
                {
                    self.notify_info("MIDI CC recording has no target yet");
                }
            }
            MidiInputEvent::NoteOff { .. } | MidiInputEvent::ProgramChange { .. } => {}
        }
    }

    pub(crate) fn record_midi_note(&mut self, note: u8, velocity: u8) {
        let pattern_index = self.pattern_index;
        let velocity = if self.song.midi.recording.velocity {
            velocity.min(127)
        } else {
            DEFAULT_NOTE_VELOCITY
        };
        let mut recorded = false;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            if pattern
                .set_note(
                    cursor.row,
                    cursor.track,
                    NoteEvent::Note {
                        pitch: note.min(127),
                    },
                    velocity,
                )
                .is_ok()
            {
                recorded = true;
            }
        });
        if recorded {
            self.advance_after_edit();
            self.notify_info(format!("Recorded MIDI note {note}"));
        }
    }

    pub(crate) fn handle_midi_clock_message(&mut self, message: MidiClockMessage) {
        match message {
            MidiClockMessage::TimingClock => {
                if !self.song.midi.clock_in {
                    return;
                }
                self.midi_clock_ticks = self.midi_clock_ticks.saturating_add(1);
                self.midi_input_status = format!("MIDI In Clock {}", self.midi_clock_ticks);
            }
            MidiClockMessage::Start => {
                if !self.song.midi.transport_in {
                    return;
                }
                self.midi_clock_ticks = 0;
                self.start_playback();
            }
            MidiClockMessage::Continue => {
                if self.song.midi.transport_in {
                    self.start_playback_from_cursor();
                }
            }
            MidiClockMessage::Stop => {
                if self.song.midi.transport_in {
                    self.stop_playback();
                }
            }
        }
    }

    fn show_midi_routing_settings(&mut self) {
        self.notify_info(format_midi_routing(&self.song.midi));
    }

    fn set_midi_clock_follow_legacy(&mut self, value: bool) {
        self.mutate_song_with(TransactionSpec::new("Edit MIDI routing"), |song, _| {
            song.midi.clock_in = value;
            song.midi.transport_in = value;
        });
        self.midi_clock_follow = value;
        self.notify_success(format!("MIDI clock+transport follow {}", on_off(value)));
    }

    fn set_midi_routing_flag(&mut self, field: &str, value: Option<bool>) {
        let Some(value) = value else {
            self.notify_warning("Usage: :midi-input clock|transport|notes|cc in|out on|off");
            return;
        };
        self.mutate_song_with(
            TransactionSpec::new("Edit MIDI routing"),
            |song, _| match field {
                "clock in" => song.midi.clock_in = value,
                "clock out" => song.midi.clock_out = value,
                "transport in" => song.midi.transport_in = value,
                "transport out" => song.midi.transport_out = value,
                "notes in" => song.midi.notes_in = value,
                "notes out" => song.midi.notes_out = value,
                "cc in" => song.midi.cc_in = value,
                "cc out" => song.midi.cc_out = value,
                _ => {}
            },
        );
        self.midi_clock_follow = self.song.midi.clock_in || self.song.midi.transport_in;
        self.notify_success(format!("MIDI {field} {}", on_off(value)));
    }

    fn set_midi_recording_flag(&mut self, field: &str, value: Option<bool>) {
        let Some(value) = value else {
            self.notify_warning("Usage: :midi-input record notes|velocity|cc on|off");
            return;
        };
        self.mutate_song_with(
            TransactionSpec::new("Edit MIDI recording"),
            |song, _| match field {
                "notes" => song.midi.recording.notes = value,
                "velocity" => song.midi.recording.velocity = value,
                "cc" => song.midi.recording.cc = value,
                _ => {}
            },
        );
        self.notify_success(format!("MIDI record {field} {}", on_off(value)));
    }

    fn set_midi_channel_filter(&mut self, direction: &str, values: &[&str]) {
        let Some(channels) = parse_channel_filter(values) else {
            self.notify_warning("Usage: :midi-input channel in|out all|CHANNEL...");
            return;
        };
        self.mutate_song_with(
            TransactionSpec::new("Edit MIDI channel filter"),
            |song, _| match direction {
                "input" => song.midi.input_channels = channels.clone(),
                "output" => song.midi.output_channels = channels.clone(),
                _ => {}
            },
        );
        self.notify_success(format!(
            "MIDI {direction} channels {}",
            format_channel_filter(match direction {
                "input" => &self.song.midi.input_channels,
                _ => &self.song.midi.output_channels,
            })
        ));
    }

    fn set_midi_middle_c(&mut self, value: &str) {
        let Some(middle_c) = value.parse::<u8>().ok().filter(|value| *value <= 127) else {
            self.notify_warning("Usage: :midi-input middle-c 0..127");
            return;
        };
        self.mutate_song_with(TransactionSpec::new("Edit MIDI middle C"), |song, _| {
            song.midi.middle_c = middle_c;
        });
        self.notify_success(format!("MIDI middle C {middle_c}"));
    }

    fn set_midi_clock_delay(&mut self, value: &str) {
        let Some(delay) = value
            .parse::<i16>()
            .ok()
            .filter(|value| (-1000..=1000).contains(value))
        else {
            self.notify_warning("Usage: :midi-input sync-delay -1000..1000");
            return;
        };
        self.mutate_song_with(TransactionSpec::new("Edit MIDI sync delay"), |song, _| {
            song.midi.clock_sync_delay_ms = delay;
        });
        self.notify_success(format!("MIDI sync delay {delay}ms"));
    }
}

fn parse_on_off(value: &str) -> Option<bool> {
    match value {
        "on" | "yes" | "true" | "1" => Some(true),
        "off" | "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

fn midi_channel_allowed(filter: &[u8], channel: u8) -> bool {
    filter.is_empty() || filter.contains(&channel)
}

fn parse_channel_filter(values: &[&str]) -> Option<Vec<u8>> {
    if matches!(values, ["all"]) {
        return Some(Vec::new());
    }
    let mut channels = Vec::new();
    for value in values {
        for part in value.split(',').filter(|part| !part.is_empty()) {
            let channel = part.parse::<u8>().ok()?;
            if !(1..=16).contains(&channel) || channels.contains(&channel) {
                return None;
            }
            channels.push(channel);
        }
    }
    Some(channels)
}

fn format_channel_filter(channels: &[u8]) -> String {
    if channels.is_empty() {
        "all".to_string()
    } else {
        channels
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

pub(crate) fn format_midi_routing(settings: &MidiRoutingSettings) -> String {
    format!(
        "MIDI routing clock in/out {}/{}, transport in/out {}/{}, notes in/out {}/{}, cc in/out {}/{}, channels in/out {}/{}, middle C {}, sync delay {}ms, record notes/velocity/cc {}/{}/{}",
        on_off(settings.clock_in),
        on_off(settings.clock_out),
        on_off(settings.transport_in),
        on_off(settings.transport_out),
        on_off(settings.notes_in),
        on_off(settings.notes_out),
        on_off(settings.cc_in),
        on_off(settings.cc_out),
        format_channel_filter(&settings.input_channels),
        format_channel_filter(&settings.output_channels),
        settings.middle_c,
        settings.clock_sync_delay_ms,
        on_off(settings.recording.notes),
        on_off(settings.recording.velocity),
        on_off(settings.recording.cc)
    )
}
