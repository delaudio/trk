use super::*;

fn note_at(app: &App, row: usize, track: usize) -> Option<u8> {
    match app
        .song
        .pattern(app.pattern_index)
        .and_then(|pattern| pattern.cell(row, track))
        .and_then(|cell| cell.note)
    {
        Some(NoteEvent::Note { pitch }) => Some(pitch),
        _ => None,
    }
}

#[test]
fn strudel_command_is_atomic_and_undoable() {
    let mut app = App::default();

    type_command(&mut app, "strudel c4 d4 ~ e4");

    assert_eq!(note_at(&app, 0, 0), Some(60));
    assert_eq!(note_at(&app, 16, 0), Some(62));
    assert_eq!(note_at(&app, 32, 0), None);
    assert_eq!(note_at(&app, 48, 0), Some(64));
    assert_eq!(app.history.undo_len(), 1);

    app.undo();
    assert_eq!(note_at(&app, 0, 0), None);
    assert_eq!(note_at(&app, 16, 0), None);
}

#[test]
fn invalid_strudel_command_preserves_pattern() {
    let mut app = App::default();
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(3, 0, NoteEvent::Note { pitch: 71 }, 88)
        .expect("seed note");
    let before = app.song.clone();

    type_command(&mut app, "strudel [c4 d4");

    assert_eq!(app.song, before);
    assert_eq!(app.history.undo_len(), 0);
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notification| notification.message.contains("mini-notation error")));
}

#[test]
fn live_editor_keeps_last_valid_preview_and_accepts_one_undo_step() {
    let mut app = App::default();
    enter_command(&mut app, "strudel live c4");
    assert_eq!(app.mode, AppMode::Strudel);
    assert_eq!(note_at(&app, 0, 0), Some(60));

    app.handle_key(KeyEvent::new(KeyCode::Char('*'), KeyModifiers::NONE));
    assert_eq!(note_at(&app, 0, 0), Some(60));
    assert!(app
        .strudel_live
        .as_ref()
        .is_some_and(|session| session.error.is_some()));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Strudel);
    assert_eq!(app.history.undo_len(), 0);

    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    assert_eq!(note_at(&app, 0, 0), Some(60));
    assert_eq!(note_at(&app, 32, 0), Some(60));
    assert!(app
        .command_line()
        .is_some_and(|line| line.starts_with("strudel live c4*2")));

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.history.undo_len(), 1);
    app.undo();
    assert_eq!(note_at(&app, 0, 0), None);
    assert_eq!(note_at(&app, 32, 0), None);
}

#[test]
fn empty_live_buffer_keeps_last_valid_preview_until_cancelled() {
    let mut app = App::default();
    enter_command(&mut app, "strudel live c4");
    app.strudel_live
        .as_mut()
        .expect("live session")
        .buffer
        .clear();

    app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

    assert_eq!(note_at(&app, 0, 0), Some(60));
    assert!(app
        .strudel_live
        .as_ref()
        .is_some_and(|session| session.error.is_some()));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(note_at(&app, 0, 0), None);
}

#[test]
fn live_editor_escape_restores_entry_snapshot_without_history() {
    let mut app = App::default();
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 71 }, 88)
        .expect("seed note");
    let before = app.song.clone();

    enter_command(&mut app, "strudel live c4*4");
    assert_ne!(app.song, before);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.song, before);
    assert_eq!(app.history.undo_len(), 0);
}

#[test]
fn live_editor_freezes_its_entry_pattern_and_track() {
    let mut app = App::default();
    let second_pattern = app.song.create_pattern(64);
    let second_index = app
        .song
        .patterns
        .iter()
        .position(|pattern| pattern.id == second_pattern)
        .expect("second pattern");
    enter_command(&mut app, "strudel live c4");

    app.pattern_index = second_index;
    app.cursor.track = 1;
    app.handle_key(KeyEvent::new(KeyCode::Char('*'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));

    assert_eq!(
        app.song
            .pattern(0)
            .and_then(|pattern| pattern.cell(32, 0))
            .and_then(|cell| cell.note),
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert_eq!(
        app.song
            .pattern(second_index)
            .and_then(|pattern| pattern.cell(0, 1))
            .and_then(|cell| cell.note),
        None
    );
}

#[test]
fn live_editor_replaces_only_its_target_tracks() {
    let mut app = App::default();
    enter_command(&mut app, "strudel live [c4,e4]");
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(7, 2, NoteEvent::Note { pitch: 72 }, 90)
        .expect("concurrent edit outside target tracks");
    app.strudel_live.as_mut().expect("live session").buffer = "g4".to_string();

    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert_eq!(note_at(&app, 0, 0), Some(67));
    assert_eq!(note_at(&app, 0, 1), None);
    assert_eq!(note_at(&app, 7, 2), Some(72));
}
