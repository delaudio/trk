use super::*;

fn select_rectangle(
    app: &mut App,
    row_start: usize,
    row_end: usize,
    track_start: usize,
    track_end: usize,
) {
    app.cursor.row = row_end;
    app.cursor.track = track_end;
    app.selection = Some(TrackerSelection::rectangle(
        SelectionEndpoint::new(row_start, track_start, CellField::Note),
        SelectionEndpoint::new(row_end, track_end, CellField::Note),
    ));
}

fn set_note(app: &mut App, row: usize, track: usize, pitch: u8) {
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(row, track, NoteEvent::Note { pitch }, 0x7f)
        .expect("set note");
}

fn note_at(app: &App, row: usize, track: usize) -> Option<NoteEvent> {
    app.song
        .current_pattern()
        .expect("pattern")
        .cell(row, track)
        .and_then(|cell| cell.note)
}

#[test]
fn pattern_fill_selection_is_undoable_and_redoable() {
    let mut app = App::default();
    set_note(&mut app, 0, 0, 60);
    select_rectangle(&mut app, 0, 1, 0, 1);

    type_command(&mut app, "pattern fill");

    assert_eq!(note_at(&app, 1, 1), Some(NoteEvent::Note { pitch: 60 }));
    app.undo();
    assert_eq!(note_at(&app, 0, 0), Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(note_at(&app, 1, 1), None);
    app.redo();
    assert_eq!(note_at(&app, 1, 1), Some(NoteEvent::Note { pitch: 60 }));
}

#[test]
fn pattern_invert_selection_reverses_rows_and_supports_undo_redo() {
    let mut app = App::default();
    set_note(&mut app, 0, 0, 60);
    set_note(&mut app, 2, 0, 64);
    select_rectangle(&mut app, 0, 2, 0, 0);

    type_command(&mut app, "pattern invert");

    assert_eq!(note_at(&app, 0, 0), Some(NoteEvent::Note { pitch: 64 }));
    assert_eq!(note_at(&app, 2, 0), Some(NoteEvent::Note { pitch: 60 }));
    app.undo();
    assert_eq!(note_at(&app, 0, 0), Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(note_at(&app, 2, 0), Some(NoteEvent::Note { pitch: 64 }));
    app.redo();
    assert_eq!(note_at(&app, 0, 0), Some(NoteEvent::Note { pitch: 64 }));
}

#[test]
fn pattern_duplicate_selection_inserts_copied_rows_and_supports_undo_redo() {
    let mut app = App::default();
    let initial_rows = app.current_row_count();
    set_note(&mut app, 0, 0, 60);
    set_note(&mut app, 1, 0, 62);
    select_rectangle(&mut app, 0, 1, 0, 0);

    type_command(&mut app, "pattern duplicate-selection");

    assert_eq!(app.current_row_count(), initial_rows + 2);
    assert_eq!(note_at(&app, 2, 0), Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(note_at(&app, 3, 0), Some(NoteEvent::Note { pitch: 62 }));
    app.undo();
    assert_eq!(app.current_row_count(), initial_rows);
    app.redo();
    assert_eq!(app.current_row_count(), initial_rows + 2);
    assert_eq!(note_at(&app, 3, 0), Some(NoteEvent::Note { pitch: 62 }));
}

#[test]
fn pattern_expand_selection_inserts_blank_rows_and_supports_undo_redo() {
    let mut app = App::default();
    let initial_rows = app.current_row_count();
    set_note(&mut app, 0, 0, 60);
    set_note(&mut app, 1, 0, 62);
    select_rectangle(&mut app, 0, 1, 0, 0);

    type_command(&mut app, "pattern expand");

    assert_eq!(app.current_row_count(), initial_rows + 2);
    assert_eq!(note_at(&app, 0, 0), Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(note_at(&app, 1, 0), None);
    assert_eq!(note_at(&app, 2, 0), Some(NoteEvent::Note { pitch: 62 }));
    app.undo();
    assert_eq!(app.current_row_count(), initial_rows);
    app.redo();
    assert_eq!(note_at(&app, 1, 0), None);
}

#[test]
fn pattern_shrink_selection_removes_every_second_row_and_supports_undo_redo() {
    let mut app = App::default();
    let initial_rows = app.current_row_count();
    set_note(&mut app, 0, 0, 60);
    set_note(&mut app, 1, 0, 61);
    set_note(&mut app, 2, 0, 62);
    select_rectangle(&mut app, 0, 3, 0, 0);

    type_command(&mut app, "pattern shrink");

    assert_eq!(app.current_row_count(), initial_rows - 2);
    assert_eq!(note_at(&app, 0, 0), Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(note_at(&app, 1, 0), Some(NoteEvent::Note { pitch: 62 }));
    app.undo();
    assert_eq!(app.current_row_count(), initial_rows);
    assert_eq!(note_at(&app, 1, 0), Some(NoteEvent::Note { pitch: 61 }));
    app.redo();
    assert_eq!(app.current_row_count(), initial_rows - 2);
}

#[test]
fn pattern_copy_and_paste_commands_use_clipboard_and_undo_stack() {
    let mut app = App::default();
    set_note(&mut app, 0, 0, 60);
    set_note(&mut app, 0, 1, 64);
    select_rectangle(&mut app, 0, 0, 0, 1);

    type_command(&mut app, "pattern copy");
    app.selection = None;
    app.cursor.row = 4;
    app.cursor.track = 1;
    type_command(&mut app, "pattern paste");

    assert_eq!(note_at(&app, 4, 1), Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(note_at(&app, 4, 2), Some(NoteEvent::Note { pitch: 64 }));
    app.undo();
    assert_eq!(note_at(&app, 4, 1), None);
    app.redo();
    assert_eq!(note_at(&app, 4, 2), Some(NoteEvent::Note { pitch: 64 }));
}
