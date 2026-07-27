use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 100,
        terminal_height: 32,
    }
}

fn click(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
}

#[test]
fn help_tab_clicks_open_every_page_and_reset_scroll() {
    for (index, expected) in HelpTab::ALL.iter().copied().enumerate() {
        let mut app = App::default();
        app.open_help();
        app.help_scroll = 12;
        app.interaction_map.register_with_payload(
            interaction_region::HELP_TAB,
            ratatui::layout::Rect::new(10, 5, 12, 1),
            InteractionPayload::HelpTab { index },
        );

        click(&mut app, MouseEventKind::Down(MouseButton::Left), 12, 5);

        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.help_tab, expected);
        assert_eq!(app.help_scroll, 0);
    }
}

#[test]
fn help_wheel_scrolls_only_over_rendered_content() {
    let mut app = App::default();
    app.open_help();
    app.interaction_map.register(
        interaction_region::HELP_CONTENT,
        ratatui::layout::Rect::new(10, 8, 70, 14),
    );

    click(&mut app, MouseEventKind::ScrollDown, 12, 8);
    assert_eq!(app.help_scroll, 3);

    click(&mut app, MouseEventKind::ScrollDown, 1, 1);
    assert_eq!(app.help_scroll, 3);

    click(&mut app, MouseEventKind::ScrollUp, 12, 8);
    assert_eq!(app.help_scroll, 0);
}

#[test]
fn help_close_click_releases_capture_to_underlying_view() {
    let mut app = App::default();
    app.open_sampler_view();
    app.open_help();
    app.interaction_map.register(
        interaction_region::HELP_CLOSE,
        ratatui::layout::Rect::new(80, 4, 9, 1),
    );

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 82, 4);

    assert_eq!(app.mode, AppMode::Sampler);
    assert_eq!(app.tui_active_view(), TuiView::Sampler);
}

#[test]
fn help_ignores_outside_secondary_drag_and_invalid_tab_targets() {
    let mut outside = App::default();
    outside.open_help();
    click(&mut outside, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert_eq!(outside.mode, AppMode::Help);
    assert!(!outside.is_playing);

    for kind in [
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        let mut app = App::default();
        app.open_help();
        app.interaction_map.register(
            interaction_region::HELP_CLOSE,
            ratatui::layout::Rect::new(80, 4, 9, 1),
        );
        click(&mut app, kind, 82, 4);
        assert_eq!(app.mode, AppMode::Help);
    }

    let mut invalid = App::default();
    invalid.open_help();
    invalid.help_scroll = 7;
    invalid.interaction_map.register_with_payload(
        interaction_region::HELP_TAB,
        ratatui::layout::Rect::new(10, 5, 12, 1),
        InteractionPayload::HelpTab { index: usize::MAX },
    );
    click(&mut invalid, MouseEventKind::Down(MouseButton::Left), 12, 5);
    assert_eq!(invalid.help_tab, HelpTab::Basics);
    assert_eq!(invalid.help_scroll, 7);
}
