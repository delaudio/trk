use crate::{pattern_events, NoteEvent, PlaybackEventKind, Song, TrackerCommand};

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
fn playback_applies_fx1_delay_and_fx2_retrigger() {
    let mut song = Song::empty();
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("set note");
    let cell = pattern.cell_mut(0, 0).expect("cell");
    cell.command = Some(TrackerCommand::delay(128));
    cell.command2 = Some(TrackerCommand::retrigger(2));

    let events = pattern_events(&song, song.current_pattern().expect("pattern"));

    assert_eq!(events[0].position.offset_micros, 62_500);
    assert_eq!(events[1].position.offset_micros, 125_000);
    assert_eq!(events[2].position.offset_micros, 125_000);
}
