use super::*;

#[test]
fn command_mode_wq_saves_and_quits() {
    let path =
        std::env::temp_dir().join(format!("salieri-command-wq-{}.salieri", std::process::id()));
    let mut app = App {
        mode: AppMode::Edit,
        project_path: Some(path.clone()),
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    type_command(&mut app, "wq");

    let saved = load_project(&path).expect("saved project loads");
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved, app.song);
    assert!(!app.dirty);
    assert!(app.should_quit);
}

#[test]
fn command_mode_creates_duplicates_selects_and_deletes_patterns() {
    let mut app = App::default();

    type_command(&mut app, "pattern new");
    assert_eq!(app.song.patterns.len(), 2);
    assert_eq!(app.pattern_index, 1);

    type_command(&mut app, "pattern 1");
    assert_eq!(app.pattern_index, 0);

    type_command(&mut app, "pattern duplicate");
    assert_eq!(app.song.patterns.len(), 3);
    assert_eq!(app.pattern_index, 2);

    enter_command(&mut app, "pattern delete");
    assert_eq!(app.mode, AppMode::Dialog);
    assert!(matches!(
        app.dialog,
        Some(Dialog::DeletePattern {
            pattern_index: 2,
            ..
        })
    ));

    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert_eq!(app.song.patterns.len(), 2);
    assert_eq!(app.pattern_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.patterns.len(), 3);
    assert_eq!(app.pattern_index, 1);
}

#[test]
fn bracket_keys_select_previous_and_next_pattern() {
    let mut app = App::default();
    type_command(&mut app, "pattern new");
    type_command(&mut app, "pattern new");

    assert_eq!(app.pattern_index, 2);

    app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 0);

    app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 0);

    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 2);

    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 2);
}

#[test]
fn uppercase_pattern_shortcuts_create_duplicate_and_delete() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
    assert_eq!(app.song.patterns.len(), 2);
    assert_eq!(app.pattern_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));
    assert_eq!(app.song.patterns.len(), 3);
    assert_eq!(app.pattern_index, 2);

    app.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT));
    assert_eq!(app.mode, AppMode::Dialog);
    assert!(matches!(
        app.dialog,
        Some(Dialog::DeletePattern {
            pattern_index: 2,
            ..
        })
    ));

    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert_eq!(app.song.patterns.len(), 2);
    assert_eq!(app.pattern_index, 1);
    assert!(app.dirty);
}

#[test]
fn patterns_view_guides_pattern_management_and_presets() {
    let mut app = App {
        cursor: Cursor {
            row: 63,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Patterns);
    assert_eq!(app.tui_active_view(), TuiView::Patterns);

    app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
    assert_eq!(app.song.patterns.len(), 2);
    assert_eq!(app.pattern_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 0);
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.pattern_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));
    assert_eq!(app.song.patterns.len(), 3);
    assert_eq!(app.pattern_index, 2);

    app.cursor.row = 63;
    app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
    assert_eq!(app.song.patterns[2].row_count(), 16);
    assert_eq!(app.cursor.row, 15);

    app.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
    assert_eq!(app.song.patterns[2].row_count(), 256);

    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "pattern rename ");
    app.command_buffer.push_str("Breakdown");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.song.patterns[2].name, "Breakdown");

    app.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Dialog);
    assert!(matches!(
        app.dialog,
        Some(Dialog::DeletePattern {
            pattern_index: 2,
            ..
        })
    ));
}
