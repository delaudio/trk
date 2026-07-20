use super::*;

#[test]
fn layout_fields_command_preserves_cursor_and_offsets() {
    let mut app = App::default();
    app.cursor.row = 12;
    app.cursor.track = 2;
    app.cursor.field = CellField::Instrument;
    app.row_offset = 8;
    app.track_offset = 1;

    type_command(&mut app, "layout fields note-fx");

    assert_eq!(app.cursor.row, 12);
    assert_eq!(app.cursor.track, 2);
    assert_eq!(app.cursor.field, CellField::Instrument);
    assert_eq!(app.row_offset, 8);
    assert_eq!(app.track_offset, 1);
    assert_eq!(
        app.tracker_layout.pattern_fields,
        salieri_tui::PatternFieldLayout::NoteFx
    );
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("note+fx"));
}
