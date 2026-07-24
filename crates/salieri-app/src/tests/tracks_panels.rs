use super::*;

#[test]
fn ctrl_t_creates_track_and_undo_restores_previous_shape() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));

    assert_eq!(app.song.tracks.len(), 5);
    assert_eq!(app.cursor.track, 4);
    assert!(app.dirty);
    assert!(app
        .song
        .current_pattern()
        .expect("pattern")
        .rows
        .iter()
        .all(|row| row.cells.len() == 5));

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

    assert_eq!(app.song.tracks.len(), 4);
    assert_eq!(app.cursor.track, 3);
    assert!(!app.dirty);
}

#[test]
fn command_mode_duplicates_track_and_undo_restores_previous_shape() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x64)
        .expect("set note");

    type_command(&mut app, "track duplicate");

    assert_eq!(app.song.tracks.len(), 5);
    assert_eq!(app.song.tracks[4].name, "Bass Copy");
    assert_eq!(app.cursor.track, 4);
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 4)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 48 })
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

    assert_eq!(app.song.tracks.len(), 4);
    assert_eq!(app.cursor.track, 3);
}

#[test]
fn uppercase_d_duplicates_current_track() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));

    assert_eq!(app.song.tracks.len(), 5);
    assert_eq!(app.song.tracks[4].name, "Bass Copy");
    assert_eq!(app.cursor.track, 4);
    assert!(app.dirty);
}

#[test]
fn command_mode_moves_track_and_undo_restores_order() {
    let mut app = App::default();
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
        .expect("set bass note");
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x70)
        .expect("set lead note");

    type_command(&mut app, "track move 2 3");

    assert_eq!(app.song.tracks[1].name, "Lead");
    assert_eq!(app.song.tracks[2].name, "Bass");
    assert_eq!(app.cursor.track, 2);
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 1)
            .expect("lead cell")
            .note,
        Some(NoteEvent::Note { pitch: 64 })
    );
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 2)
            .expect("bass cell")
            .note,
        Some(NoteEvent::Note { pitch: 48 })
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

    assert_eq!(app.song.tracks[1].name, "Bass");
    assert_eq!(app.song.tracks[2].name, "Lead");
}

#[test]
fn brace_shortcuts_move_current_track_left_and_right() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
        .expect("set bass note");

    app.handle_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::SHIFT));

    assert_eq!(app.song.tracks[0].name, "Bass");
    assert_eq!(app.cursor.track, 0);
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 48 })
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('}'), KeyModifiers::SHIFT));

    assert_eq!(app.song.tracks[1].name, "Bass");
    assert_eq!(app.cursor.track, 1);
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 1)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 48 })
    );
    assert!(app.dirty);
}

#[test]
fn command_mode_deletes_numbered_track_after_confirmation() {
    let mut app = App::default();

    enter_command(&mut app, "track delete 2");

    assert_eq!(app.mode, AppMode::Dialog);
    assert!(matches!(
        app.dialog,
        Some(Dialog::DeleteTrack { track_index: 1, .. })
    ));

    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.song.tracks.len(), 3);
    assert_eq!(app.song.tracks[1].name, "Lead");
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

    assert_eq!(app.song.tracks.len(), 4);
    assert_eq!(app.song.tracks[1].name, "Bass");
}

#[test]
fn delete_in_normal_mode_removes_current_track_and_cells() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Dialog);
    assert!(matches!(
        app.dialog,
        Some(Dialog::DeleteTrack { track_index: 1, .. })
    ));

    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    assert_eq!(app.song.tracks.len(), 3);
    assert_eq!(app.song.tracks[1].name, "Lead");
    assert_eq!(app.cursor.track, 1);
    assert!(app
        .song
        .current_pattern()
        .expect("pattern")
        .rows
        .iter()
        .all(|row| row.cells.len() == 3));
}

#[test]
fn delete_track_dialog_can_be_cancelled() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.song.tracks.len(), 4);
    assert_eq!(app.song.tracks[1].name, "Bass");
}

#[test]
fn tracks_view_guides_track_management() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Tracks);
    assert_eq!(app.tui_active_view(), TuiView::Tracks);

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.cursor.track, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    assert_eq!(app.song.tracks.len(), 5);
    assert_eq!(app.cursor.track, 4);
    assert_eq!(app.song.tracks[4].name, "Bass Copy");

    app.handle_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::SHIFT));
    assert_eq!(app.cursor.track, 3);

    app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(app.song.tracks[3].muted);
    assert!(app.song.tracks[3].solo);

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "track channel 4 ");
    app.command_buffer.push('9');
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.song.tracks[3].midi_channel, 9);

    app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "track rename 4 ");
    app.command_buffer.push_str("Aux Bass");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.song.tracks[3].name, "Aux Bass");

    app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Dialog);
    assert!(matches!(
        app.dialog,
        Some(Dialog::DeleteTrack { track_index: 3, .. })
    ));

    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert_eq!(app.song.tracks.len(), 5);
    assert_eq!(app.mode, AppMode::Tracks);
}

#[test]
fn cannot_delete_last_track_from_app() {
    let mut app = App::default();

    while app.song.tracks.len() > 1 {
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

    assert_eq!(app.song.tracks.len(), 1);
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Cannot delete the last track")
    );
}

#[test]
fn mute_and_solo_commands_toggle_current_track() {
    let mut app = App {
        cursor: Cursor {
            track: 2,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

    assert!(app.song.tracks[2].muted);
    assert!(app.song.tracks[2].solo);
    assert!(app.dirty);
}

#[test]
fn command_mode_mutes_and_solos_numbered_track() {
    let mut app = App::default();

    type_command(&mut app, "track mute 2");
    type_command(&mut app, "track solo 2");

    assert!(app.song.tracks[1].muted);
    assert!(app.song.tracks[1].solo);
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(app.song.tracks[1].muted);
    assert!(!app.song.tracks[1].solo);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert!(!app.song.tracks[1].muted);
}

#[test]
fn command_mode_changes_current_or_named_track_midi_channel() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };

    type_command(&mut app, "track channel 12");
    assert_eq!(app.song.tracks[1].midi_channel, 12);
    assert!(app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Track channel set to 12")
    );

    type_command(&mut app, "track channel 3 15");
    assert_eq!(app.song.tracks[2].midi_channel, 15);

    type_command(&mut app, "track channel 3 0");
    assert_eq!(app.song.tracks[2].midi_channel, 15);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Track channel failed: invalid MIDI channel: 0")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.tracks[2].midi_channel, 2);
}

#[test]
fn command_mode_renames_current_or_named_track() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };

    type_command(&mut app, "track rename Acid Bass");
    assert_eq!(app.song.tracks[1].name, "Acid Bass");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Track renamed")
    );

    type_command(&mut app, "track rename 3 Main Lead");
    assert_eq!(app.song.tracks[2].name, "Main Lead");

    type_command(&mut app, "track rename 3    ");
    assert_eq!(app.song.tracks[2].name, "Main Lead");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Track rename failed: name cannot be empty")
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.tracks[2].name, "Lead");
}

#[test]
fn r_prefills_current_track_rename_command() {
    let mut app = App {
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "track rename 2 ");

    for value in "Sub Bass".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.song.tracks[1].name, "Sub Bass");
}

#[test]
fn c_prefills_current_track_channel_command() {
    let mut app = App {
        cursor: Cursor {
            track: 2,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Command);
    assert_eq!(app.command_buffer, "track channel 3 ");

    for value in "12".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.song.tracks[2].midi_channel, 12);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Track channel set to 12")
    );
}

#[test]
fn command_mode_panel_aliases_focus_views_and_restore_tracker_layout() {
    let mut app = App::default();

    enter_command(&mut app, "p");
    assert_eq!(app.mode, AppMode::Patterns);
    assert_eq!(app.tui_active_view(), TuiView::Patterns);

    enter_command(&mut app, "se");
    assert_eq!(app.mode, AppMode::Sequence);
    assert_eq!(app.tui_active_view(), TuiView::Sequence);

    enter_command(&mut app, "tr");
    assert_eq!(app.mode, AppMode::Tracks);
    assert_eq!(app.tui_active_view(), TuiView::Tracks);

    enter_command(&mut app, "sa");
    assert_eq!(app.mode, AppMode::Sampler);
    assert_eq!(app.tui_active_view(), TuiView::Sampler);

    enter_command(&mut app, "focus dsp");
    assert_eq!(app.mode, AppMode::DspRack);
    assert_eq!(app.tui_active_view(), TuiView::DspRack);

    enter_command(&mut app, "t");
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);

    enter_command(&mut app, "focus p");
    assert_eq!(app.mode, AppMode::Patterns);
    enter_command(&mut app, "layout");
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn command_mode_layout_commands_manage_tracker_panels() {
    let mut app = App::default();

    enter_command(&mut app, "layout studio");
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tracker_layout.preset, TrackerLayoutPreset::Studio);
    assert!(app.tracker_layout.inspector_visible);

    enter_command(&mut app, "layout hide inspector");
    assert!(!app.tracker_layout.inspector_visible);

    enter_command(&mut app, "layout toggle inspector");
    assert!(app.tracker_layout.inspector_visible);

    let previous = app.tracker_layout.inspector_width;
    enter_command(&mut app, "layout resize inspector 4");
    assert_eq!(app.tracker_layout.inspector_width, previous + 4);
}

#[test]
fn command_mode_sample_browser_alias_accepts_optional_directory() {
    let mut app = App::default();

    enter_command(&mut app, "sb fixtures");

    assert_eq!(app.mode, AppMode::SampleBrowser);
    assert_eq!(app.tui_active_view(), TuiView::SampleBrowser);
    assert_eq!(
        app.sample_browser_view
            .as_ref()
            .map(|browser| browser.current_dir.as_path()),
        Some(Path::new("fixtures"))
    );
}

#[test]
fn panel_aliases_do_not_fire_while_editing_text_or_cells() {
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Edit);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);
}

#[test]
fn f1_and_f2_change_octave_in_normal_mode() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(app.octave, 5);
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
    assert_eq!(app.octave, 4);
}
