use super::*;

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
        12,
        4,
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
        12,
        4,
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
        12,
        4,
    );

    let browser = app.sample_browser_view.as_ref().expect("browser");
    assert_eq!(browser.cursor, 1);
    assert_eq!(browser.message.as_deref(), Some("Unsupported file type"));
}
