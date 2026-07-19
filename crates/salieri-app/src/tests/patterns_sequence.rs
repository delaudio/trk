use super::*;

#[test]
fn command_mode_renames_current_pattern() {
    let mut app = App::default();

    type_command(&mut app, "pattern rename Intro Verse");

    assert_eq!(app.song.patterns[0].name, "Intro Verse");
    assert!(app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern renamed")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.patterns[0].name, "Pattern 01");
}

#[test]
fn f3_prefills_current_pattern_rename_command() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "pattern rename ");

    for value in "Intro Verse".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.song.patterns[0].name, "Intro Verse");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern renamed")
    );
}

#[test]
fn command_mode_reports_invalid_pattern_rename() {
    let mut app = App::default();

    type_command(&mut app, "pattern rename     ");

    assert_eq!(app.song.patterns[0].name, "Pattern 01");
    assert!(!app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern rename failed: name cannot be empty")
    );
}

#[test]
fn command_mode_resizes_current_pattern_and_clamps_cursor() {
    let mut app = App {
        cursor: Cursor {
            row: 63,
            ..Cursor::new()
        },
        row_offset: 44,
        ..App::default()
    };

    type_command(&mut app, "pattern length 16");

    assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 16);
    assert_eq!(app.cursor.row, 15);
    assert_eq!(app.row_offset, 15);
    assert!(app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern length set to 16")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
}

#[test]
fn f6_prefills_current_pattern_length_command() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "pattern length ");

    for value in "32".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 32);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern length set to 32")
    );
}

#[test]
fn command_mode_reports_invalid_pattern_length() {
    let mut app = App::default();

    type_command(&mut app, "pattern length 0");

    assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
    assert!(!app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern length failed: invalid pattern length: 0")
    );
}

#[test]
fn command_mode_adds_and_removes_sequence_positions() {
    let mut app = App::default();

    type_command(&mut app, "pattern new");
    type_command(&mut app, "sequence add");
    assert_eq!(
        app.song.sequence,
        vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
    );
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence added pattern 02")
    );

    type_command(&mut app, "sequence remove 0");
    assert_eq!(app.song.sequence, vec![salieri_core::PatternId(2)]);
    assert_eq!(app.song.patterns.len(), 2);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence removed position 00")
    );
}

#[test]
fn command_mode_reports_sequence_add_pattern_out_of_range() {
    let mut app = App::default();

    type_command(&mut app, "sequence add 99");

    assert_eq!(app.song.sequence, vec![salieri_core::PatternId(1)]);
    assert!(!app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern out of range")
    );
}

#[test]
fn command_mode_duplicates_sets_and_moves_sequence_positions() {
    let mut app = App::default();

    type_command(&mut app, "pattern new");
    type_command(&mut app, "pattern new");
    type_command(&mut app, "sequence add 2");
    type_command(&mut app, "sequence add 3");

    type_command(&mut app, "sequence duplicate 1");
    assert_eq!(
        app.song.sequence,
        vec![
            salieri_core::PatternId(1),
            salieri_core::PatternId(2),
            salieri_core::PatternId(2),
            salieri_core::PatternId(3)
        ]
    );
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence duplicated position 01")
    );

    type_command(&mut app, "sequence set 0 3");
    assert_eq!(app.song.sequence[0], salieri_core::PatternId(3));
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence position 00 set to pattern 03")
    );

    type_command(&mut app, "sequence move 3 1");
    assert_eq!(
        app.song.sequence,
        vec![
            salieri_core::PatternId(3),
            salieri_core::PatternId(3),
            salieri_core::PatternId(2),
            salieri_core::PatternId(2)
        ]
    );
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence moved position 03 to 01")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.sequence[1], salieri_core::PatternId(2));
}

#[test]
fn keyboard_sequence_shortcuts_edit_selected_position() {
    let mut app = App::default();
    type_command(&mut app, "pattern new");
    type_command(&mut app, "pattern new");
    app.pattern_index = 1;

    app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert_eq!(
        app.song.sequence,
        vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
    );
    assert_eq!(app.sequence_cursor, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char(','), KeyModifiers::NONE));
    assert_eq!(app.sequence_cursor, 0);
    app.handle_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
    assert_eq!(app.sequence_cursor, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT));
    assert_eq!(
        app.song.sequence,
        vec![
            salieri_core::PatternId(1),
            salieri_core::PatternId(2),
            salieri_core::PatternId(2)
        ]
    );
    assert_eq!(app.sequence_cursor, 2);

    app.pattern_index = 2;
    app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
    assert_eq!(app.song.sequence[2], salieri_core::PatternId(3));

    app.handle_key(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::SHIFT));
    assert_eq!(app.sequence_cursor, 1);
    assert_eq!(
        app.song.sequence,
        vec![
            salieri_core::PatternId(1),
            salieri_core::PatternId(3),
            salieri_core::PatternId(2)
        ]
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::SHIFT));
    assert_eq!(app.sequence_cursor, 2);
    assert_eq!(
        app.song.sequence,
        vec![
            salieri_core::PatternId(1),
            salieri_core::PatternId(2),
            salieri_core::PatternId(3)
        ]
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));
    assert_eq!(
        app.song.sequence,
        vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
    );
    assert_eq!(app.sequence_cursor, 1);
    assert!(app.dirty);
}

#[test]
fn sequence_view_navigation_edits_and_playback() {
    let mut app = App::default();
    type_command(&mut app, "pattern new");
    type_command(&mut app, "pattern new");
    app.pattern_index = 1;

    app.handle_key(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Sequence);
    assert_eq!(app.tui_active_view(), TuiView::Sequence);

    app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert_eq!(
        app.song.sequence,
        vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
    );
    assert_eq!(app.sequence_cursor, 1);

    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.sequence_cursor, 0);
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.sequence_cursor, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT));
    assert_eq!(
        app.song.sequence,
        vec![
            salieri_core::PatternId(1),
            salieri_core::PatternId(2),
            salieri_core::PatternId(2)
        ]
    );
    assert_eq!(app.sequence_cursor, 2);

    app.pattern_index = 2;
    app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
    assert_eq!(app.song.sequence[2], salieri_core::PatternId(3));

    app.handle_key(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::SHIFT));
    assert_eq!(app.sequence_cursor, 1);
    app.handle_key(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::SHIFT));
    assert_eq!(app.sequence_cursor, 2);

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(app.is_playing);
    assert_eq!(app.sequence_position, Some(2));

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);
}

#[test]
fn sequence_panel_tracks_automatic_pattern_playback() {
    let mut app = App::default();
    type_command(&mut app, "pattern new");
    app.add_sequence_pattern(1);

    app.is_playing = true;
    app.pattern_index = 1;
    app.sequence_position = None;
    app.sequence_cursor = 0;

    assert_eq!(app.tui_sequence_position(), Some(1));
    assert_eq!(app.sequence_cursor, 0);
}

#[test]
fn command_mode_reports_sequence_position_errors() {
    let mut app = App::default();

    type_command(&mut app, "sequence remove 99");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence remove failed: sequence out of bounds: position 99")
    );
    assert!(!app.dirty);

    type_command(&mut app, "sequence duplicate 99");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence duplicate failed: sequence out of bounds: position 99")
    );

    type_command(&mut app, "sequence set 99 1");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence set failed: sequence out of bounds: position 99")
    );

    type_command(&mut app, "sequence set 0 99");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern out of range")
    );

    type_command(&mut app, "sequence move 99 0");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Sequence move failed: sequence out of bounds: position 99")
    );
}

#[test]
fn help_mode_opens_and_closes_without_mutating_state() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Help);
    assert_eq!(app.help_tab, HelpTab::Basics);
    assert_eq!(app.help_scroll, 1);
    assert_eq!(app.cursor.row, 0);
    assert!(!app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(app.help_scroll, 11);
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.help_tab, HelpTab::Editing);
    assert_eq!(app.help_scroll, 0);
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
    assert_eq!(app.help_tab, HelpTab::Basics);
    app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    assert_eq!(app.help_scroll, 0);
    app.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(app.help_scroll, 0);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);
    assert!(!app.should_quit);

    app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));
    assert_eq!(app.mode, AppMode::Help);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Help);
}
