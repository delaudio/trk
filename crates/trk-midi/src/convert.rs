use trk_core::{PlaybackEvent, PlaybackEventKind};

use crate::MidiMessage;

#[must_use]
pub fn playback_event_to_midi(event: PlaybackEvent) -> MidiMessage {
    match event.kind {
        PlaybackEventKind::NoteOn { pitch, velocity } => {
            MidiMessage::note_on(event.midi_channel, pitch, velocity)
        }
        PlaybackEventKind::NoteOff { pitch } => MidiMessage::note_off(event.midi_channel, pitch, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trk_core::{PlaybackPosition, TrackId};

    #[test]
    fn converts_note_on_playback_event_to_midi_message() {
        let event = PlaybackEvent {
            position: PlaybackPosition {
                row: 0,
                offset_micros: 0,
            },
            track: TrackId(1),
            midi_channel: 10,
            kind: PlaybackEventKind::NoteOn {
                pitch: 36,
                velocity: 100,
            },
        };

        assert_eq!(
            playback_event_to_midi(event),
            MidiMessage::note_on(10, 36, 100)
        );
    }

    #[test]
    fn converts_note_off_playback_event_to_midi_message() {
        let event = PlaybackEvent {
            position: PlaybackPosition {
                row: 4,
                offset_micros: 500_000,
            },
            track: TrackId(2),
            midi_channel: 1,
            kind: PlaybackEventKind::NoteOff { pitch: 48 },
        };

        assert_eq!(
            playback_event_to_midi(event),
            MidiMessage::note_off(1, 48, 0)
        );
    }
}
