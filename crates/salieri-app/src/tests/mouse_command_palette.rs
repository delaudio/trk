use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 100,
        terminal_height: 32,
    }
}

fn register_entry(app: &mut App, index: usize) {
    app.interaction_map.register_with_payload(
        interaction_region::COMMAND_PALETTE_ENTRY,
        ratatui::layout::Rect::new(10, 8, 70, 1),
        InteractionPayload::CommandPaletteEntry { index },
    );
}

#[test]
fn enabled_palette_entry_click_executes_existing_action_path() {
    let mut app = App::default();
    app.open_command_palette();
    app.command_palette_query = "sampler".to_string();
    register_entry(&mut app, 0);

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );

    assert_eq!(app.mode, AppMode::Sampler);
    assert_eq!(app.tui_active_view(), TuiView::Sampler);
    assert_eq!(
        app.command_palette_recent.first().map(String::as_str),
        Some("view.sampler")
    );
}

#[test]
fn disabled_palette_entry_click_selects_without_executing() {
    let mut app = App::default();
    app.open_command_palette();
    app.command_palette_query = "stop".to_string();
    register_entry(&mut app, 0);

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );

    assert_eq!(app.mode, AppMode::CommandPalette);
    assert_eq!(app.command_palette_selected, 0);
    assert!(!app.is_playing);
    assert!(app.command_palette_recent.is_empty());
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notification| notification.message.contains("Playback is stopped")));
}

#[test]
fn palette_wheel_is_scoped_to_result_list_and_clamped() {
    let mut app = App::default();
    app.open_command_palette();
    app.interaction_map.register(
        interaction_region::COMMAND_PALETTE_RESULTS,
        ratatui::layout::Rect::new(10, 8, 70, 10),
    );
    let last = app.command_palette_results().len().saturating_sub(1);

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 12,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
    assert_eq!(app.command_palette_selected, 3.min(last));

    app.command_palette_selected = last;
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 12,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
    assert_eq!(app.command_palette_selected, last);

    app.command_palette_selected = 1;
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
    assert_eq!(app.command_palette_selected, 1);
}

#[test]
fn palette_ignores_outside_secondary_drag_and_invalid_entry_targets() {
    let mut outside = App::default();
    outside.open_command_palette();
    register_entry(&mut outside, 1);
    outside.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
    assert_eq!(outside.mode, AppMode::CommandPalette);
    assert_eq!(outside.command_palette_selected, 0);
    assert!(!outside.is_playing);

    for kind in [
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        let mut app = App::default();
        app.open_command_palette();
        register_entry(&mut app, 1);
        app.handle_mouse(
            MouseEvent {
                kind,
                column: 12,
                row: 8,
                modifiers: KeyModifiers::NONE,
            },
            viewport(),
        );
        assert_eq!(app.mode, AppMode::CommandPalette);
        assert_eq!(app.command_palette_selected, 0);
    }

    let mut invalid = App::default();
    invalid.open_command_palette();
    register_entry(&mut invalid, usize::MAX);
    invalid.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
    assert_eq!(invalid.mode, AppMode::CommandPalette);
    assert_eq!(invalid.command_palette_selected, 0);
}
