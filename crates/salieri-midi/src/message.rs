#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiMessage {
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
}

impl MidiMessage {
    #[must_use]
    pub fn note_on(channel: u8, note: u8, velocity: u8) -> Self {
        Self::NoteOn {
            channel: normalize_channel(channel),
            note: note.min(127),
            velocity: velocity.min(127),
        }
    }

    #[must_use]
    pub fn note_off(channel: u8, note: u8, velocity: u8) -> Self {
        Self::NoteOff {
            channel: normalize_channel(channel),
            note: note.min(127),
            velocity: velocity.min(127),
        }
    }

    #[must_use]
    pub fn control_change(channel: u8, controller: u8, value: u8) -> Self {
        Self::ControlChange {
            channel: normalize_channel(channel),
            controller: controller.min(127),
            value: value.min(127),
        }
    }

    #[must_use]
    pub fn all_notes_off(channel: u8) -> Self {
        Self::control_change(channel, 123, 0)
    }

    #[must_use]
    pub fn to_bytes(self) -> [u8; 3] {
        match self {
            Self::NoteOn {
                channel,
                note,
                velocity,
            } => [0x90 | zero_based_channel(channel), note, velocity],
            Self::NoteOff {
                channel,
                note,
                velocity,
            } => [0x80 | zero_based_channel(channel), note, velocity],
            Self::ControlChange {
                channel,
                controller,
                value,
            } => [0xb0 | zero_based_channel(channel), controller, value],
        }
    }
}

fn normalize_channel(channel: u8) -> u8 {
    channel.clamp(1, 16)
}

fn zero_based_channel(channel: u8) -> u8 {
    normalize_channel(channel) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_on_is_encoded_as_three_midi_bytes() {
        assert_eq!(MidiMessage::note_on(1, 60, 127).to_bytes(), [0x90, 60, 127]);
        assert_eq!(
            MidiMessage::note_on(10, 36, 100).to_bytes(),
            [0x99, 36, 100]
        );
    }

    #[test]
    fn message_values_are_clamped_to_valid_midi_ranges() {
        assert_eq!(
            MidiMessage::control_change(99, 200, 250).to_bytes(),
            [0xbf, 127, 127]
        );
        assert_eq!(
            MidiMessage::note_off(0, 200, 200).to_bytes(),
            [0x80, 127, 127]
        );
    }

    #[test]
    fn all_notes_off_uses_controller_123() {
        assert_eq!(MidiMessage::all_notes_off(2).to_bytes(), [0xb1, 123, 0]);
    }
}
