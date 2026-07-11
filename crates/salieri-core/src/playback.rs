use crate::{NoteEvent, Pattern, Song, TrackId, TrackerCommand, TransportSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackPosition {
    pub row: usize,
    pub offset_micros: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackEvent {
    pub position: PlaybackPosition,
    pub track: TrackId,
    pub midi_channel: u8,
    pub kind: PlaybackEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackEventKind {
    NoteOn { pitch: u8, velocity: u8 },
    NoteOff { pitch: u8 },
}

#[must_use]
pub fn row_duration_micros(transport: &TransportSettings) -> u64 {
    let bpm = u64::from(transport.bpm.max(1));
    let lines_per_beat = u64::from(transport.lines_per_beat.max(1));
    60_000_000 / bpm / lines_per_beat
}

#[must_use]
pub fn pattern_events(song: &Song, pattern: &Pattern) -> Vec<PlaybackEvent> {
    let row_duration = row_duration_micros(&song.transport);
    let solo_active = song.tracks.iter().any(|track| track.solo);
    let mut active_notes = vec![None; song.tracks.len()];
    let mut events = Vec::new();

    for (row_index, row) in pattern.rows.iter().enumerate() {
        let position = PlaybackPosition {
            row: row_index,
            offset_micros: row_duration.saturating_mul(row_index as u64),
        };

        for (track_index, track) in song.tracks.iter().enumerate() {
            if !track_is_audible(track.muted, track.solo, solo_active) {
                continue;
            }

            let Some(cell) = row.cells.get(track_index) else {
                continue;
            };

            let position = apply_delay_command(position, row_duration, cell.command);

            match cell.note {
                Some(NoteEvent::Note { pitch }) => {
                    if let Some(active_pitch) = active_notes[track_index] {
                        events.push(note_off(
                            position,
                            track.id,
                            track.midi_channel,
                            active_pitch,
                        ));
                    }
                    let velocity = cell.velocity.unwrap_or(0x7f).min(0x7f);
                    let note_on = PlaybackEvent {
                        position,
                        track: track.id,
                        midi_channel: track.midi_channel,
                        kind: PlaybackEventKind::NoteOn { pitch, velocity },
                    };
                    events.push(note_on);
                    active_notes[track_index] = Some(pitch);
                    emit_retrigger_events(&mut events, note_on, row_duration, cell.command);
                }
                Some(NoteEvent::NoteOff | NoteEvent::NoteCut) => {
                    if let Some(active_pitch) = active_notes[track_index].take() {
                        events.push(note_off(
                            position,
                            track.id,
                            track.midi_channel,
                            active_pitch,
                        ));
                    }
                }
                None => {}
            }
        }
    }

    let end_position = PlaybackPosition {
        row: pattern.row_count(),
        offset_micros: row_duration.saturating_mul(pattern.row_count() as u64),
    };
    for (track_index, active_pitch) in active_notes.into_iter().enumerate() {
        if let Some(pitch) = active_pitch {
            let track = &song.tracks[track_index];
            events.push(note_off(end_position, track.id, track.midi_channel, pitch));
        }
    }

    events.sort_by_key(|event| event.position.offset_micros);
    events
}

fn apply_delay_command(
    position: PlaybackPosition,
    row_duration: u64,
    command: Option<TrackerCommand>,
) -> PlaybackPosition {
    let Some(command) = command else {
        return position;
    };
    if command.code.to_ascii_uppercase() != TrackerCommand::DELAY_CODE {
        return position;
    }

    PlaybackPosition {
        offset_micros: position
            .offset_micros
            .saturating_add(row_duration.saturating_mul(u64::from(command.value)) / 256),
        ..position
    }
}

fn emit_retrigger_events(
    events: &mut Vec<PlaybackEvent>,
    note_on: PlaybackEvent,
    row_duration: u64,
    command: Option<TrackerCommand>,
) {
    let Some(command) = command else {
        return;
    };
    if command.code.to_ascii_uppercase() != TrackerCommand::RETRIGGER_CODE {
        return;
    }

    let count = command.value.clamp(1, 16);
    let PlaybackEventKind::NoteOn { pitch, velocity } = note_on.kind else {
        return;
    };
    for step in 1..count {
        let offset = note_on
            .position
            .offset_micros
            .saturating_add(row_duration.saturating_mul(u64::from(step)) / u64::from(count));
        let position = PlaybackPosition {
            offset_micros: offset,
            ..note_on.position
        };
        events.push(note_off(
            position,
            note_on.track,
            note_on.midi_channel,
            pitch,
        ));
        events.push(PlaybackEvent {
            position,
            track: note_on.track,
            midi_channel: note_on.midi_channel,
            kind: PlaybackEventKind::NoteOn { pitch, velocity },
        });
    }
}

fn track_is_audible(muted: bool, solo: bool, solo_active: bool) -> bool {
    if solo_active {
        solo
    } else {
        !muted
    }
}

fn note_off(
    position: PlaybackPosition,
    track: TrackId,
    midi_channel: u8,
    pitch: u8,
) -> PlaybackEvent {
    PlaybackEvent {
        position,
        track,
        midi_channel,
        kind: PlaybackEventKind::NoteOff { pitch },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PatternId, TrackId};

    #[test]
    fn row_duration_uses_bpm_and_lines_per_beat() {
        let song = Song::empty();

        assert_eq!(row_duration_micros(&song.transport), 125_000);
    }

    #[test]
    fn row_duration_updates_when_bpm_or_lpb_changes() {
        let mut transport = Song::empty().transport;

        transport.bpm = 60;
        transport.lines_per_beat = 4;
        assert_eq!(row_duration_micros(&transport), 250_000);

        transport.bpm = 120;
        transport.lines_per_beat = 8;
        assert_eq!(row_duration_micros(&transport), 62_500);

        transport.bpm = 150;
        transport.lines_per_beat = 6;
        assert_eq!(row_duration_micros(&transport), 66_666);
    }

    #[test]
    fn pattern_event_offsets_follow_current_transport_settings() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(8, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        let initial_events = pattern_events(&song, song.current_pattern().expect("pattern"));
        assert_eq!(initial_events[0].position.offset_micros, 1_000_000);

        song.transport.bpm = 240;
        song.transport.lines_per_beat = 8;
        let faster_events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(faster_events[0].position.offset_micros, 250_000);
    }

    #[test]
    fn pattern_events_emit_note_on_and_end_note_off() {
        let mut song = Song::empty();
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
            .expect("set note");

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(
            events,
            vec![
                PlaybackEvent {
                    position: PlaybackPosition {
                        row: 0,
                        offset_micros: 0
                    },
                    track: TrackId(1),
                    midi_channel: 10,
                    kind: PlaybackEventKind::NoteOn {
                        pitch: 60,
                        velocity: 0x64
                    }
                },
                PlaybackEvent {
                    position: PlaybackPosition {
                        row: 64,
                        offset_micros: 8_000_000
                    },
                    track: TrackId(1),
                    midi_channel: 10,
                    kind: PlaybackEventKind::NoteOff { pitch: 60 }
                }
            ]
        );
    }

    #[test]
    fn note_off_cell_stops_active_note_at_that_row() {
        let mut song = Song::empty();
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set note");
        pattern
            .set_note(4, 1, NoteEvent::NoteOff, 0)
            .expect("set note off");

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[1],
            PlaybackEvent {
                position: PlaybackPosition {
                    row: 4,
                    offset_micros: 500_000
                },
                track: TrackId(2),
                midi_channel: 1,
                kind: PlaybackEventKind::NoteOff { pitch: 48 }
            }
        );
    }

    #[test]
    fn retriggering_same_track_emits_previous_note_off_first() {
        let mut song = Song::empty();
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x7f)
            .expect("set first note");
        pattern
            .set_note(2, 2, NoteEvent::Note { pitch: 67 }, 0x70)
            .expect("set second note");

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events[1].position.row, 2);
        assert_eq!(events[1].kind, PlaybackEventKind::NoteOff { pitch: 64 });
        assert_eq!(events[2].position.row, 2);
        assert_eq!(
            events[2].kind,
            PlaybackEventKind::NoteOn {
                pitch: 67,
                velocity: 0x70
            }
        );
    }

    #[test]
    fn delay_command_offsets_note_within_row() {
        let mut song = Song::empty();
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(2, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");
        pattern.cell_mut(2, 0).expect("cell").command = Some(TrackerCommand::delay(128));

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events[0].position.row, 2);
        assert_eq!(events[0].position.offset_micros, 312_500);
    }

    #[test]
    fn retrigger_command_emits_repeated_note_events() {
        let mut song = Song::empty();
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
            .expect("set note");
        pattern.cell_mut(0, 0).expect("cell").command = Some(TrackerCommand::retrigger(4));

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));
        let note_on_count = events
            .iter()
            .filter(|event| matches!(event.kind, PlaybackEventKind::NoteOn { .. }))
            .count();

        assert_eq!(note_on_count, 4);
        assert_eq!(events[1].position.offset_micros, 31_250);
        assert_eq!(events[2].position.offset_micros, 31_250);
        assert_eq!(events[3].position.offset_micros, 62_500);
    }

    #[test]
    fn muted_tracks_are_not_scheduled() {
        let mut song = Song::empty();
        song.toggle_mute(0).expect("mute");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert!(events.is_empty());
    }

    #[test]
    fn solo_tracks_exclude_non_solo_tracks() {
        let mut song = Song::empty();
        song.toggle_solo(1).expect("solo");
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set muted by solo note");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x7f)
            .expect("set solo note");

        let events = pattern_events(&song, song.current_pattern().expect("pattern"));

        assert_eq!(events[0].track, TrackId(2));
        assert!(events
            .iter()
            .all(|event| event.track == TrackId(2) && event.track != TrackId(1)));
    }

    #[test]
    fn can_schedule_non_current_pattern() {
        let song = Song::empty();
        let mut pattern = crate::Pattern::empty(PatternId(2), "Other", 16, song.tracks.len());
        pattern
            .set_note(1, 0, NoteEvent::Note { pitch: 36 }, 0x50)
            .expect("set note");

        let events = pattern_events(&song, &pattern);

        assert_eq!(events[0].position.row, 1);
        assert_eq!(events[0].position.offset_micros, 125_000);
    }
}
