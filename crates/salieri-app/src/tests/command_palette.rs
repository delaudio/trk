use super::*;

#[test]
fn command_palette_search_executes_and_tracks_recents() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(app.mode, AppMode::CommandPalette);
    assert_eq!(app.focus.capture(), Some(FocusCapture::CommandPalette));

    for value in "sampler".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    assert_eq!(app.command_palette_results()[0].action.id, "view.sampler");

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Sampler);
    assert_eq!(app.tui_active_view(), TuiView::Sampler);
    assert_eq!(app.command_palette_recent[0], "view.sampler");

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(app.command_palette_results()[0].action.id, "view.sampler");
}

#[test]
fn command_palette_cancel_restores_prior_focus() {
    let mut app = App::default();
    app.open_tracks_view();

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(app.mode, AppMode::CommandPalette);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Tracks);
    assert_eq!(app.tui_active_view(), TuiView::Tracks);
}

#[test]
fn command_palette_disabled_actions_explain_without_executing() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    for value in "stop".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    assert_eq!(
        app.command_palette_results()[0].disabled_reason,
        Some("Playback is stopped")
    );

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::CommandPalette);
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notification| notification.message.contains("Playback is stopped")));
}

#[test]
fn command_palette_prompt_actions_handoff_to_command_mode() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    for value in "save as".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    assert_eq!(app.command_palette_results()[0].action.id, "save-as");

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "saveas ");
}

#[test]
fn command_palette_prompts_for_midi_import() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    for value in "import midi".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    assert_eq!(
        app.command_palette_results()[0].action.id,
        "project.import-midi"
    );

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "midi import ");
}

#[test]
fn command_palette_selection_action_clears_selection_region() {
    let mut app = App::default();
    {
        let pattern = app.song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");
        pattern
            .set_note(1, 1, NoteEvent::Note { pitch: 62 }, 0x7f)
            .expect("set note");
    }

    app.start_selection();
    app.cursor.row = 1;
    app.cursor.track = 1;

    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    for value in "clear selection".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    assert_eq!(
        app.command_palette_results()[0].action.id,
        "selection.clear"
    );

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.selection_rect(), None);
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell"),
        &PatternCell::default()
    );
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(1, 1)
            .expect("cell"),
        &PatternCell::default()
    );
}
