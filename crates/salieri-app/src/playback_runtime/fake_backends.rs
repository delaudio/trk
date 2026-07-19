use std::sync::{Arc, Mutex};

use salieri_midi::{MidiError, MidiMessage, MidiOutput};

pub(super) struct RecordingMidiOutput {
    pub(super) messages: Arc<Mutex<Vec<MidiMessage>>>,
}

#[cfg(test)]
impl MidiOutput for RecordingMidiOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        self.messages
            .lock()
            .expect("recorded MIDI messages")
            .push(message);
        Ok(())
    }
}

#[cfg(test)]
pub(super) struct FailingMidiOutput;

#[cfg(test)]
impl MidiOutput for FailingMidiOutput {
    fn send(&mut self, _message: MidiMessage) -> Result<(), MidiError> {
        Err(MidiError::Send("simulated disconnected MIDI port".into()))
    }
}
