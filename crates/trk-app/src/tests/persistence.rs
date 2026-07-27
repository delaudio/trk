use super::*;

#[test]
fn keyboard_note_maps_tracker_keys_to_midi_pitches() {
    assert_eq!(keyboard_note('z', 4), Some(60));
    assert_eq!(keyboard_note('s', 4), Some(61));
    assert_eq!(keyboard_note('q', 4), Some(72));
    assert_eq!(keyboard_note('u', 4), Some(83));
}

#[test]
fn ctrl_s_saves_project_and_clears_dirty_state() {
    let path = std::env::temp_dir().join(format!("trk-app-save-{}.trk", std::process::id()));
    let mut app = App {
        mode: AppMode::Edit,
        project_path: Some(path.clone()),
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

    let saved = load_project(&path).expect("saved project loads");
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved, app.song);
    assert!(!app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Project saved")
    );
}

#[test]
fn ctrl_shift_s_opens_save_as_prompt_with_current_path() {
    let path = PathBuf::from("current-song.trk");
    let mut app = App {
        project_path: Some(path.clone()),
        ..App::default()
    };

    app.handle_key(KeyEvent::new(
        KeyCode::Char('S'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));

    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, format!("saveas {}", path.display()));
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Save As: edit path and press Enter")
    );
}

#[test]
fn save_as_prompt_can_save_to_selected_path() {
    let path =
        std::env::temp_dir().join(format!("trk-shortcut-save-as-{}.trk", std::process::id()));
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(
        KeyCode::Char('S'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    app.command_buffer = format!("saveas {}", path.display());
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let saved = load_project(&path).expect("saved project loads");
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved, app.song);
    assert_eq!(app.project_path, Some(path));
    assert!(!app.dirty);
}

#[test]
fn save_as_bare_name_uses_workspace_project_library() {
    let dir = std::env::temp_dir().join(format!("trk-save-as-library-{}", std::process::id()));
    let expected_path = dir.join("library-song.trk");
    let mut app = App::new(AppConfig {
        workspace: config::WorkspaceConfig {
            project_library: Some(dir.clone()),
            ..config::WorkspaceConfig::default()
        },
        ..AppConfig::default()
    });
    app.set_bpm(132);
    assert!(app.dirty);

    enter_command(&mut app, "saveas library-song");

    let saved = load_project(&expected_path).expect("saved project loads");
    assert_eq!(saved, app.song);
    assert_eq!(app.project_path, Some(expected_path));
    assert!(!app.dirty);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_without_project_uses_workspace_project_library() {
    let dir = std::env::temp_dir().join(format!("trk-write-library-{}", std::process::id()));
    let expected_path = dir.join("untitled.trk");
    let mut app = App::new(AppConfig {
        workspace: config::WorkspaceConfig {
            project_library: Some(dir.clone()),
            ..config::WorkspaceConfig::default()
        },
        ..AppConfig::default()
    });
    app.set_bpm(134);
    assert!(app.dirty);

    enter_command(&mut app, "write");

    let saved = load_project(&expected_path).expect("saved project loads");
    assert_eq!(saved, app.song);
    assert_eq!(app.project_path, Some(expected_path));
    assert!(!app.dirty);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_as_prompt_reports_errors_and_keeps_dirty_state() {
    let missing_dir =
        std::env::temp_dir().join(format!("trk-missing-save-as-dir-{}", std::process::id()));
    let path = missing_dir.join("song.trk");
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(
        KeyCode::Char('S'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    app.command_buffer = format!("saveas {}", path.display());
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let notification = app.notification.as_ref().expect("notification");
    assert_eq!(notification.kind, NotificationKind::Error);
    assert!(notification.message.starts_with("Save failed:"));
    assert_eq!(app.project_path, None);
    assert!(app.dirty);
}
