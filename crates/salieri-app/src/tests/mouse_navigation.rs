use super::*;

fn large_mouse_viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 160,
        terminal_height: 40,
        visible_rows: 12,
        visible_tracks: 4,
    }
}

#[test]
fn mouse_wheel_moves_tracker_cursor_through_shared_viewport() {
    let mut app = App::default();

    app.handle_mouse_wheel(MouseEventKind::ScrollDown);
    app.keep_cursor_visible(2);
    app.keep_track_visible(app.cursor.track, 4);

    assert_eq!(app.cursor.row, 3);
    assert_eq!(app.row_offset, 2);

    app.handle_mouse_wheel(MouseEventKind::ScrollUp);
    app.keep_cursor_visible(2);
    app.keep_track_visible(app.cursor.track, 4);

    assert_eq!(app.cursor.row, 0);
    assert_eq!(app.row_offset, 0);
}

#[test]
fn mouse_click_moves_tracker_cursor_to_grid_cell() {
    let mut app = App::default();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 45,
            row: 14,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.cursor.row, 4);
    assert_eq!(app.cursor.track, 2);
}

#[test]
fn mouse_click_selects_project_browser_entries() {
    let mut app = App {
        mode: AppMode::ProjectBrowser,
        project_browser_view: Some(AppProjectBrowserView {
            current_dir: PathBuf::from("/tmp/projects"),
            entries: vec![
                AppProjectBrowserEntry {
                    path: PathBuf::from("/tmp/projects/a.salieri"),
                    name: "a.salieri".to_string(),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "A".to_string(),
                },
                AppProjectBrowserEntry {
                    path: PathBuf::from("/tmp/projects/b.salieri"),
                    name: "b.salieri".to_string(),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "B".to_string(),
                },
            ],
            cursor: 0,
            message: None,
        }),
        ..App::default()
    };

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(
        app.project_browser_view
            .as_ref()
            .map(|browser| browser.cursor),
        Some(1)
    );
}

#[test]
fn mouse_click_selects_sample_browser_entries() {
    let mut app = App {
        mode: AppMode::SampleBrowser,
        sample_browser_view: Some(AppSampleBrowserView {
            current_dir: PathBuf::from("/tmp/samples"),
            entries: vec![
                AppSampleBrowserEntry {
                    path: PathBuf::from("/tmp/samples/kick.wav"),
                    name: "kick.wav".to_string(),
                    kind: SampleBrowserEntryKind::SupportedSample,
                },
                AppSampleBrowserEntry {
                    path: PathBuf::from("/tmp/samples/readme.txt"),
                    name: "readme.txt".to_string(),
                    kind: SampleBrowserEntryKind::UnsupportedFile,
                },
            ],
            cursor: 0,
            preview: None,
            message: None,
        }),
        ..App::default()
    };

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    let browser = app.sample_browser_view.as_ref().expect("browser");
    assert_eq!(browser.cursor, 1);
    assert_eq!(browser.message.as_deref(), Some("Unsupported file type"));
}

#[test]
fn mouse_click_renoise_sidebar_tabs_open_sections() {
    let mut app = App::default();
    let viewport = large_mouse_viewport();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 139,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );
    assert_eq!(app.mode, AppMode::SampleBrowser);

    app.open_tracker_view();
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 131,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );
    assert_eq!(app.mode, AppMode::Tracks);
}

#[test]
fn mouse_click_renoise_sidebar_pattern_selects_pattern() {
    let mut app = App::default();
    app.create_pattern();
    app.create_pattern();
    app.select_pattern(0);
    let viewport = large_mouse_viewport();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 126,
            row: 24,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );

    assert_eq!(app.pattern_index, 1);
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn mouse_click_renoise_sidebar_sample_selects_assigned_track() {
    let mut app = App::default();
    let sample = app.song.upsert_sample_reference("samples/kick.wav", "Kick");
    let track = app.song.tracks[1].id;
    app.song
        .assign_sample_to_track(track, sample)
        .expect("assign sample");
    app.cursor.track = 0;
    let viewport = large_mouse_viewport();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 126,
            row: 22,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );

    assert_eq!(app.cursor.track, 1);
    assert_eq!(app.mode, AppMode::Normal);
}
