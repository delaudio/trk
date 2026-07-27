use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 160,
        terminal_height: 40,
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
fn disabled_sampler_chrome_is_non_mutating_for_all_click_kinds() {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        let mut app = App::default();
        app.open_sampler_view();
        app.interaction_map.register(
            interaction_region::VIEW_SAMPLER,
            ratatui::layout::Rect::new(0, 3, 160, 36),
        );
        let cursor = app.cursor;
        let dirty = app.dirty;
        let pattern_count = app.song.patterns.len();
        let track_count = app.song.tracks.len();

        for (column, row) in [(3, 4), (31, 7), (35, 16)] {
            click(&mut app, kind, column, row);
        }

        assert_eq!(app.mode, AppMode::Sampler);
        assert_eq!(app.cursor, cursor);
        assert_eq!(app.dirty, dirty);
        assert_eq!(app.song.patterns.len(), pattern_count);
        assert_eq!(app.song.tracks.len(), track_count);
    }
}

#[test]
fn disabled_pattern_other_tab_never_mutates_or_navigates() {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        let mut app = App::default();
        let cursor = app.cursor;
        let dirty = app.dirty;
        let pattern_count = app.song.patterns.len();
        let track_count = app.song.tracks.len();

        click(&mut app, kind, 150, 10);

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.cursor, cursor);
        assert_eq!(app.dirty, dirty);
        assert_eq!(app.song.patterns.len(), pattern_count);
        assert_eq!(app.song.tracks.len(), track_count);
    }
}
