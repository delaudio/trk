use midir::{MidiOutput as MidirOutput, MidiOutputConnection};

use crate::MidiMessage;

pub trait MidiOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiOutputPort {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MidiError {
    #[error("failed to initialize MIDI output: {0}")]
    Init(String),
    #[error("MIDI output port {index} is not available")]
    PortUnavailable { index: usize },
    #[error("failed to connect MIDI output: {0}")]
    Connect(String),
    #[error("failed to send MIDI message: {0}")]
    Send(String),
}

#[derive(Debug, Default)]
pub struct FakeMidiOutput {
    sent: Vec<MidiMessage>,
}

impl FakeMidiOutput {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn sent(&self) -> &[MidiMessage] {
        &self.sent
    }
}

impl MidiOutput for FakeMidiOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        self.sent.push(message);
        Ok(())
    }
}

pub struct MidirMidiOutput {
    connection: MidiOutputConnection,
}

impl MidirMidiOutput {
    pub fn connect(port_index: usize, connection_name: &str) -> Result<Self, MidiError> {
        let midi =
            MidirOutput::new("salieri").map_err(|error| MidiError::Init(error.to_string()))?;
        let ports = midi.ports();
        let port = ports
            .get(port_index)
            .ok_or(MidiError::PortUnavailable { index: port_index })?;
        let connection = midi
            .connect(port, connection_name)
            .map_err(|error| MidiError::Connect(error.to_string()))?;
        Ok(Self { connection })
    }
}

impl MidiOutput for MidirMidiOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        self.connection
            .send(&message.to_bytes())
            .map_err(|error| MidiError::Send(error.to_string()))
    }
}

pub fn list_output_ports() -> Result<Vec<MidiOutputPort>, MidiError> {
    let midi = MidirOutput::new("salieri").map_err(|error| MidiError::Init(error.to_string()))?;
    midi.ports()
        .iter()
        .enumerate()
        .map(|(index, port)| {
            let name = midi
                .port_name(port)
                .unwrap_or_else(|_| format!("MIDI Output {index}"));
            Ok(MidiOutputPort { index, name })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_output_records_sent_messages() {
        let mut output = FakeMidiOutput::new();

        output
            .send(MidiMessage::note_on(1, 60, 100))
            .expect("send note on");
        output
            .send(MidiMessage::note_off(1, 60, 0))
            .expect("send note off");

        assert_eq!(
            output.sent(),
            &[
                MidiMessage::note_on(1, 60, 100),
                MidiMessage::note_off(1, 60, 0)
            ]
        );
    }
}
