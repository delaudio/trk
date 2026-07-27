use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver};

use midir::{Ignore, MidiInput as MidirInput, MidiInputConnection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiInputPacket {
    pub timestamp_micros: u64,
    pub event: MidiInputEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiInputPort {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiInputEvent {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    Clock(MidiClockMessage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiClockMessage {
    TimingClock,
    Start,
    Continue,
    Stop,
}

pub trait MidiInput {
    fn poll(&mut self) -> Result<Option<MidiInputPacket>, MidiInputError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MidiInputError {
    #[error("failed to initialize MIDI input: {0}")]
    Init(String),
    #[error("MIDI input port {index} is not available")]
    PortUnavailable { index: usize },
    #[error("failed to connect MIDI input: {0}")]
    Connect(String),
    #[error("MIDI input message is empty")]
    Empty,
    #[error("unsupported MIDI input status byte 0x{0:02X}")]
    UnsupportedStatus(u8),
    #[error("MIDI input message is too short for status byte 0x{status:02X}")]
    Truncated { status: u8 },
}

pub struct MidirMidiInput {
    _connection: MidiInputConnection<()>,
    packet_rx: Receiver<Result<MidiInputPacket, MidiInputError>>,
}

impl MidirMidiInput {
    pub fn connect(port_index: usize, connection_name: &str) -> Result<Self, MidiInputError> {
        let mut midi =
            MidirInput::new("trk").map_err(|error| MidiInputError::Init(error.to_string()))?;
        midi.ignore(Ignore::None);
        let ports = midi.ports();
        let port = ports
            .get(port_index)
            .ok_or(MidiInputError::PortUnavailable { index: port_index })?;
        let (packet_tx, packet_rx) = mpsc::channel();
        let connection = midi
            .connect(
                port,
                connection_name,
                move |timestamp_micros, bytes, _| {
                    let packet = parse_midi_input(bytes).map(|event| MidiInputPacket {
                        timestamp_micros,
                        event,
                    });
                    let _ = packet_tx.send(packet);
                },
                (),
            )
            .map_err(|error| MidiInputError::Connect(error.to_string()))?;
        Ok(Self {
            _connection: connection,
            packet_rx,
        })
    }
}

impl MidiInput for MidirMidiInput {
    fn poll(&mut self) -> Result<Option<MidiInputPacket>, MidiInputError> {
        match self.packet_rx.try_recv() {
            Ok(packet) => packet.map(Some),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(MidiInputError::Connect(
                "MIDI input disconnected".to_string(),
            )),
        }
    }
}

#[derive(Debug, Default)]
pub struct FakeMidiInput {
    packets: VecDeque<MidiInputPacket>,
}

impl FakeMidiInput {
    #[must_use]
    pub fn new(packets: impl IntoIterator<Item = MidiInputPacket>) -> Self {
        Self {
            packets: packets.into_iter().collect(),
        }
    }

    pub fn push(&mut self, packet: MidiInputPacket) {
        self.packets.push_back(packet);
    }
}

impl MidiInput for FakeMidiInput {
    fn poll(&mut self) -> Result<Option<MidiInputPacket>, MidiInputError> {
        Ok(self.packets.pop_front())
    }
}

pub fn list_input_ports() -> Result<Vec<MidiInputPort>, MidiInputError> {
    let midi = MidirInput::new("trk").map_err(|error| MidiInputError::Init(error.to_string()))?;
    midi.ports()
        .iter()
        .enumerate()
        .map(|(index, port)| {
            let name = midi
                .port_name(port)
                .unwrap_or_else(|_| format!("MIDI Input {index}"));
            Ok(MidiInputPort { index, name })
        })
        .collect()
}

pub fn parse_midi_input(bytes: &[u8]) -> Result<MidiInputEvent, MidiInputError> {
    let Some(status) = bytes.first().copied() else {
        return Err(MidiInputError::Empty);
    };

    match status {
        0x80..=0x8f => {
            require_len(bytes, 3, status)?;
            Ok(MidiInputEvent::NoteOff {
                channel: channel_from_status(status),
                note: bytes[1].min(127),
                velocity: bytes[2].min(127),
            })
        }
        0x90..=0x9f => {
            require_len(bytes, 3, status)?;
            let channel = channel_from_status(status);
            let note = bytes[1].min(127);
            let velocity = bytes[2].min(127);
            if velocity == 0 {
                Ok(MidiInputEvent::NoteOff {
                    channel,
                    note,
                    velocity,
                })
            } else {
                Ok(MidiInputEvent::NoteOn {
                    channel,
                    note,
                    velocity,
                })
            }
        }
        0xb0..=0xbf => {
            require_len(bytes, 3, status)?;
            Ok(MidiInputEvent::ControlChange {
                channel: channel_from_status(status),
                controller: bytes[1].min(127),
                value: bytes[2].min(127),
            })
        }
        0xc0..=0xcf => {
            require_len(bytes, 2, status)?;
            Ok(MidiInputEvent::ProgramChange {
                channel: channel_from_status(status),
                program: bytes[1].min(127),
            })
        }
        0xf8 => Ok(MidiInputEvent::Clock(MidiClockMessage::TimingClock)),
        0xfa => Ok(MidiInputEvent::Clock(MidiClockMessage::Start)),
        0xfb => Ok(MidiInputEvent::Clock(MidiClockMessage::Continue)),
        0xfc => Ok(MidiInputEvent::Clock(MidiClockMessage::Stop)),
        _ => Err(MidiInputError::UnsupportedStatus(status)),
    }
}

fn require_len(bytes: &[u8], len: usize, status: u8) -> Result<(), MidiInputError> {
    if bytes.len() < len {
        Err(MidiInputError::Truncated { status })
    } else {
        Ok(())
    }
}

fn channel_from_status(status: u8) -> u8 {
    (status & 0x0f) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_channel_voice_input_messages() {
        assert_eq!(
            parse_midi_input(&[0x90, 60, 100]).expect("note on"),
            MidiInputEvent::NoteOn {
                channel: 1,
                note: 60,
                velocity: 100,
            }
        );
        assert_eq!(
            parse_midi_input(&[0x91, 60, 0]).expect("zero velocity note off"),
            MidiInputEvent::NoteOff {
                channel: 2,
                note: 60,
                velocity: 0,
            }
        );
        assert_eq!(
            parse_midi_input(&[0xb2, 74, 64]).expect("cc"),
            MidiInputEvent::ControlChange {
                channel: 3,
                controller: 74,
                value: 64,
            }
        );
        assert_eq!(
            parse_midi_input(&[0xc3, 12]).expect("program"),
            MidiInputEvent::ProgramChange {
                channel: 4,
                program: 12,
            }
        );
    }

    #[test]
    fn parses_midi_clock_messages() {
        assert_eq!(
            parse_midi_input(&[0xf8]).expect("clock"),
            MidiInputEvent::Clock(MidiClockMessage::TimingClock)
        );
        assert_eq!(
            parse_midi_input(&[0xfa]).expect("start"),
            MidiInputEvent::Clock(MidiClockMessage::Start)
        );
        assert_eq!(
            parse_midi_input(&[0xfc]).expect("stop"),
            MidiInputEvent::Clock(MidiClockMessage::Stop)
        );
    }

    #[test]
    fn fake_midi_input_polls_packets_deterministically() {
        let packet = MidiInputPacket {
            timestamp_micros: 123,
            event: MidiInputEvent::Clock(MidiClockMessage::Start),
        };
        let mut input = FakeMidiInput::new([packet]);

        assert_eq!(input.poll().expect("poll"), Some(packet));
        assert_eq!(input.poll().expect("poll"), None);
    }

    #[test]
    fn reports_input_parse_failures() {
        assert!(matches!(parse_midi_input(&[]), Err(MidiInputError::Empty)));
        assert!(matches!(
            parse_midi_input(&[0x90, 60]),
            Err(MidiInputError::Truncated { status: 0x90 })
        ));
        assert!(matches!(
            parse_midi_input(&[0xf0]),
            Err(MidiInputError::UnsupportedStatus(0xf0))
        ));
    }
}
