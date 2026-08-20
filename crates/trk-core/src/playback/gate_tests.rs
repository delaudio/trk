use super::*;
use crate::TrackId;

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
        .expect("note");
    assert_eq!(
        pattern_events(&song, song.current_pattern().expect("pattern"))[0]
            .position
            .offset_micros,
        1_000_000
    );
    song.transport.bpm = 240;
    song.transport.lines_per_beat = 8;
    assert_eq!(
        pattern_events(&song, song.current_pattern().expect("pattern"))[0]
            .position
            .offset_micros,
        250_000
    );
}

#[test]
fn pattern_events_emit_note_on_and_end_note_off() {
    let mut song = Song::empty();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
        .expect("note");
    let events = pattern_events(&song, song.current_pattern().expect("pattern"));
    assert_eq!(
        events[0].kind,
        PlaybackEventKind::NoteOn {
            pitch: 60,
            velocity: 0x64
        }
    );
    assert_eq!(
        events[1].position,
        PlaybackPosition {
            row: 64,
            offset_micros: 8_000_000
        }
    );
    assert_eq!(events[1].kind, PlaybackEventKind::NoteOff { pitch: 60 });
}

#[test]
fn explicit_gate_releases_after_delayed_onset() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(2, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("note");
    pattern.cell_mut(2, 0).expect("cell").delay = Some(128);
    pattern.set_gate(2, 0, Some(3)).expect("gate");
    let events = pattern_events(&song, song.current_pattern().expect("pattern"));
    assert_eq!(events[0].position.offset_micros, 312_500);
    assert_eq!(events[1].position.offset_micros, 687_500);
    assert_eq!(events[1].kind, PlaybackEventKind::NoteOff { pitch: 60 });
}

#[test]
fn expired_gate_releases_before_a_delayed_retrigger() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("first note");
    pattern.set_gate(0, 0, Some(1)).expect("gate");
    pattern
        .set_note(1, 0, NoteEvent::Note { pitch: 67 }, 100)
        .expect("retrigger");
    pattern.cell_mut(1, 0).expect("cell").delay = Some(128);

    let events = pattern_events(&song, song.current_pattern().expect("pattern"));

    assert_eq!(events[1].kind, PlaybackEventKind::NoteOff { pitch: 60 });
    assert_eq!(events[1].position.offset_micros, 125_000);
    assert_eq!(
        events[2].kind,
        PlaybackEventKind::NoteOn {
            pitch: 67,
            velocity: 100,
        }
    );
    assert_eq!(events[2].position.offset_micros, 187_500);
}

#[test]
fn midi_cc_automation_emits_routed_control_changes() {
    let mut song = Song::empty();
    let track = song.tracks[1].id;
    song.current_pattern_mut()
        .expect("pattern")
        .set_automation_point(
            AutomationTarget::MidiCc {
                track,
                controller: 74,
            },
            4,
            0.5,
        )
        .expect("CC point");
    let events = pattern_events(&song, song.current_pattern().expect("pattern"));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].track, track);
    assert_eq!(events[0].midi_channel, 1);
    assert_eq!(
        events[0].kind,
        PlaybackEventKind::ControlChange {
            controller: 74,
            value: 64
        }
    );
}

#[test]
fn note_off_cell_stops_active_note_at_that_row() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x7f)
        .expect("note");
    pattern
        .set_note(4, 1, NoteEvent::NoteOff, 0)
        .expect("note off");
    let events = pattern_events(&song, song.current_pattern().expect("pattern"));
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].position.row, 4);
    assert_eq!(events[1].track, TrackId(2));
    assert_eq!(events[1].kind, PlaybackEventKind::NoteOff { pitch: 48 });
}

#[test]
fn retriggering_same_track_emits_previous_note_off_first() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x7f)
        .expect("first");
    pattern
        .set_note(2, 2, NoteEvent::Note { pitch: 67 }, 0x70)
        .expect("second");
    let events = pattern_events(&song, song.current_pattern().expect("pattern"));
    assert_eq!(events[1].kind, PlaybackEventKind::NoteOff { pitch: 64 });
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
        .expect("note");
    pattern.cell_mut(2, 0).expect("cell").command = Some(TrackerCommand::delay(128));
    let events = pattern_events(&song, song.current_pattern().expect("pattern"));
    assert_eq!(events[0].position.offset_micros, 312_500);
}

#[test]
fn muted_tracks_are_not_scheduled() {
    let mut song = Song::empty();
    song.toggle_mute(0).expect("mute");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("note");
    assert!(pattern_events(&song, song.current_pattern().expect("pattern")).is_empty());
}
