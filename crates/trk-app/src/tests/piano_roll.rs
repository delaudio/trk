use super::*;

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn view_command_and_escape_toggle_the_piano_roll() {
    let mut app = App::default();

    enter_command(&mut app, "view roll");
    assert_eq!(app.mode, AppMode::PianoRoll);
    assert!(matches!(app.tui_active_view(), TuiView::PianoRoll { .. }));

    app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);
}

#[test]
fn piano_roll_edits_round_trip_through_tracker_cells_and_undo() {
    let mut app = App::default();
    app.open_piano_roll_view();
    app.piano_roll_pitch = 64;
    app.cursor.row = 4;

    app.handle_key(key(KeyCode::Char(' '), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Right, KeyModifiers::SHIFT));
    app.handle_key(key(KeyCode::Char('9'), KeyModifiers::NONE));

    let cell = app.song.patterns[0]
        .cell(4, 0)
        .expect("edited cell")
        .clone();
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 64 }));
    assert_eq!(cell.gate, Some(2));
    assert_eq!(cell.velocity, Some(127));

    app.open_tracker_view();
    assert_eq!(app.song.patterns[0].cell(4, 0), Some(&cell));
    app.undo();
    assert_eq!(
        app.song.patterns[0].cell(4, 0).expect("cell").velocity,
        Some(0x7f)
    );
}

#[test]
fn piano_roll_ghost_zoom_and_collision_safe_move_are_bounded() {
    let mut app = App::default();
    app.open_piano_roll_view();
    app.cursor.row = 1;
    app.piano_roll_pitch = 60;
    app.handle_key(key(KeyCode::Char(' '), KeyModifiers::NONE));
    app.song.patterns[0]
        .set_note(2, 0, NoteEvent::Note { pitch: 62 }, 100)
        .expect("destination note");

    app.handle_key(key(KeyCode::Right, KeyModifiers::ALT));
    assert_eq!(app.cursor.row, 1);
    assert_eq!(
        app.song.patterns[0].cell(1, 0).expect("source").note,
        Some(NoteEvent::Note { pitch: 60 })
    );

    app.handle_key(key(KeyCode::Char('g'), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char(']'), KeyModifiers::NONE));
    assert!(!app.piano_roll_ghosts);
    assert_eq!(app.piano_roll_rows, 32);
}
