use trk_midi::{FakeMidiOutput, MidiError, MidiMessage, MidiOutput, MidirMidiOutput};

#[cfg(test)]
use super::fake_backends::{FailingMidiOutput, RecordingMidiOutput};

pub(super) enum PlaybackOutput {
    Fake(FakeMidiOutput),
    Midir(MidirMidiOutput),
    #[cfg(test)]
    Recording(RecordingMidiOutput),
    #[cfg(test)]
    Failing(FailingMidiOutput),
}

impl PlaybackOutput {
    pub(super) fn fake() -> Self {
        Self::Fake(FakeMidiOutput::new())
    }

    #[cfg(test)]
    pub(super) fn failing() -> Self {
        Self::Failing(FailingMidiOutput)
    }

    #[cfg(test)]
    pub(super) fn recording(messages: std::sync::Arc<std::sync::Mutex<Vec<MidiMessage>>>) -> Self {
        Self::Recording(RecordingMidiOutput { messages })
    }
}

impl MidiOutput for PlaybackOutput {
    fn send(&mut self, message: MidiMessage) -> Result<(), MidiError> {
        match self {
            Self::Fake(output) => output.send(message),
            Self::Midir(output) => output.send(message),
            #[cfg(test)]
            Self::Recording(output) => output.send(message),
            #[cfg(test)]
            Self::Failing(output) => output.send(message),
        }
    }
}
