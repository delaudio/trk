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
            ["ports"] | ["inputs"] | ["settings"] => self.refresh_midi_input_ports(),
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
            ["clock", "on"] => {
                self.midi_clock_follow = true;
                self.midi_clock_ticks = 0;
                self.notify_info("MIDI clock follow ON");
            }
            ["clock", "off"] => {
                self.midi_clock_follow = false;
                self.midi_clock_ticks = 0;
                self.notify_info("MIDI clock follow OFF");
            }
            _ => self.notify_warning(
                "Usage: :midi-input ports|connect PORT|disconnect|record on|record off|clock on|clock off",
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
            MidiInputEvent::NoteOn { note, velocity, .. } => {
                if self.midi_record_armed {
                    self.record_midi_note(note, velocity);
                }
            }
            MidiInputEvent::Clock(message) => self.handle_midi_clock_message(message),
            MidiInputEvent::NoteOff { .. }
            | MidiInputEvent::ControlChange { .. }
            | MidiInputEvent::ProgramChange { .. } => {}
        }
    }

    pub(crate) fn record_midi_note(&mut self, note: u8, velocity: u8) {
        let pattern_index = self.pattern_index;
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
                    velocity.min(127),
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
        if !self.midi_clock_follow {
            return;
        }
        match message {
            MidiClockMessage::TimingClock => {
                self.midi_clock_ticks = self.midi_clock_ticks.saturating_add(1);
                self.midi_input_status = format!("MIDI In Clock {}", self.midi_clock_ticks);
            }
            MidiClockMessage::Start => {
                self.midi_clock_ticks = 0;
                self.start_playback();
            }
            MidiClockMessage::Continue => self.start_playback_from_cursor(),
            MidiClockMessage::Stop => self.stop_playback(),
        }
    }
}
