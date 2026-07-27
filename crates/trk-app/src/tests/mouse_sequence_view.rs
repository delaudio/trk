use super::*;

fn large_mouse_viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 160,
        terminal_height: 40,
    }
}

fn sequence_app_with_target(position: usize) -> App {
    let mut app = App::default();
    let second = app.song.create_pattern(64);
    let third = app.song.create_pattern(64);
    app.song
        .push_sequence_pattern(second)
        .expect("second sequence slot");
    app.song
        .push_sequence_pattern(third)
        .expect("third sequence slot");
    app.open_sequence_view();
    app.interaction_map.register_with_payload(
        interaction_region::SEQUENCE_EDITOR_ROW,
        ratatui::layout::Rect::new(1, 8, 40, 1),
        InteractionPayload::SequenceEditorRow { position },
    );
    app
}

#[test]
fn sequence_editor_primary_click_selects_and_retains_sequence_view() {
    let mut app = sequence_app_with_target(2);
    app.pattern_index = 1;

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Sequence);
    assert_eq!(app.tui_active_view(), TuiView::Sequence);
    assert_eq!(app.sequence_cursor, 2);
    assert_eq!(app.pattern_index, 1);
    assert!(!app.is_playing);
    assert_eq!(app.sequence_position, None);
    assert_eq!(
        app.notification
            .as_ref()
            .map(|value| value.message.as_str()),
        Some("Sequence position 02")
    );
}

#[test]
fn sequence_editor_secondary_click_selects_plays_and_retains_sequence_view() {
    let mut app = sequence_app_with_target(1);

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 2,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Sequence);
    assert_eq!(app.tui_active_view(), TuiView::Sequence);
    assert_eq!(app.sequence_cursor, 1);
    assert_eq!(app.pattern_index, 1);
    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));
    assert_eq!(app.sequence_position, Some(1));
    assert_eq!(
        app.notification
            .as_ref()
            .map(|value| value.message.as_str()),
        Some("Playing sequence from 1")
    );
}

#[test]
fn sequence_editor_rows_ignore_drag_and_invalid_payloads() {
    let mut dragged = sequence_app_with_target(1);
    dragged.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 2,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );
    assert_eq!(dragged.sequence_cursor, 0);
    assert!(!dragged.is_playing);

    let mut invalid = sequence_app_with_target(usize::MAX);
    invalid.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 2,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );
    assert_eq!(invalid.sequence_cursor, 0);
    assert!(!invalid.is_playing);
    assert_eq!(invalid.sequence_position, None);
}
