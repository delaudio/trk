use super::*;

#[test]
fn app_uses_keyboard_config_defaults() {
    let app = App::new(AppConfig {
        keyboard: config::KeyboardConfig {
            default_octave: 5,
            edit_step: 4,
            vim_navigation: false,
        },
        ui: config::UiConfig {
            show_line_numbers_hex: true,
            ..config::UiConfig::default()
        },
        ..AppConfig::default()
    });

    assert_eq!(app.octave, 5);
    assert_eq!(app.edit_step, 4);
    assert!(!app.vim_navigation);
    assert!(app.show_line_numbers_hex);
}

#[test]
fn configured_keymap_overrides_defaults_per_mode() {
    let mut config = AppConfig::default();
    config
        .keymap
        .normal
        .insert("q".to_string(), "bpm 150".to_string());
    config
        .keymap
        .edit
        .insert("q".to_string(), "bpm 90".to_string());
    let mut app = App::new(config);
    let song_before_edit = app.song.clone();

    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

    assert_eq!(app.song.transport.bpm, 150);
    assert!(!app.should_quit);

    app.mode = AppMode::Edit;
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

    assert_eq!(app.song.transport.bpm, 90);
    assert_eq!(app.mode, AppMode::Edit);
    assert_eq!(app.song.patterns, song_before_edit.patterns);

    app.open_help();
    assert!(app
        .notification
        .as_ref()
        .expect("custom key help")
        .message
        .contains("normal.q -> :bpm 150"));
}

#[test]
fn vim_navigation_can_be_disabled_by_config() {
    let mut app = App::new(AppConfig {
        keyboard: config::KeyboardConfig {
            vim_navigation: false,
            ..config::KeyboardConfig::default()
        },
        ..AppConfig::default()
    });
    app.cursor.row = 4;

    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    assert_eq!(app.cursor.row, 3);
}

#[test]
fn vim_navigation_jumps_to_pattern_bounds() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
    assert_eq!(app.cursor.row, 63);

    app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert_eq!(app.cursor.row, 63);

    app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    assert_eq!(app.cursor.row, 0);
}

#[test]
fn playhead_follow_can_be_disabled_by_config() {
    let mut app = App::new(AppConfig {
        ui: config::UiConfig {
            follow_playhead: false,
            ..config::UiConfig::default()
        },
        ..AppConfig::default()
    });
    app.cursor.row = 0;
    app.is_playing = true;
    app.playhead_row = Some(20);

    app.keep_active_row_visible(10);

    assert_eq!(app.row_offset, 0);
}

#[test]
fn finds_midi_output_by_exact_or_partial_name() {
    let ports = vec![
        MidiOutputPort {
            index: 0,
            name: "External Synth".to_string(),
        },
        MidiOutputPort {
            index: 1,
            name: "IAC Driver Bus 1".to_string(),
        },
    ];

    assert_eq!(
        find_midi_output_port(&ports, "IAC Driver").map(|(position, port)| (position, port.index)),
        Some((1, 1))
    );
    assert_eq!(
        find_midi_output_port(&ports, "iac driver bus 1")
            .map(|(position, port)| (position, port.index)),
        Some((1, 1))
    );
    assert_eq!(
        find_midi_output_port(&ports, "IAC Driver (Bus 1)")
            .map(|(position, port)| (position, port.index)),
        Some((1, 1))
    );
    assert_eq!(
        resolve_midi_output_port(&ports, "1")
            .map(|(position, port)| (position, port.name.as_str())),
        Some((1, "IAC Driver Bus 1"))
    );
    assert!(find_midi_output_port(&ports, "Missing").is_none());
}

#[test]
fn finds_midi_input_by_exact_or_partial_name() {
    let ports = vec![
        MidiInputPort {
            index: 0,
            name: "USB Keyboard".to_string(),
        },
        MidiInputPort {
            index: 2,
            name: "IAC Driver Bus 1".to_string(),
        },
    ];

    assert_eq!(
        resolve_midi_input_port(&ports, "2").map(|(position, port)| (position, port.name.as_str())),
        Some((1, "IAC Driver Bus 1"))
    );
    assert_eq!(
        find_midi_input_port(&ports, "IAC Driver (Bus 1)")
            .map(|(position, port)| (position, port.index)),
        Some((1, 2))
    );
    assert_eq!(
        find_midi_input_port(&ports, "keyboard").map(|(_, port)| port.index),
        Some(0)
    );
    assert!(find_midi_input_port(&ports, "Missing").is_none());
}

#[test]
fn midi_settings_keys_select_connect_and_close() {
    let mut app = App {
        midi_ports: vec![
            MidiOutputPort {
                index: 0,
                name: "First".to_string(),
            },
            MidiOutputPort {
                index: 2,
                name: "Second".to_string(),
            },
        ],
        mode: AppMode::MidiSettings,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.midi_port_cursor, 1);

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.midi_status, "MIDI Connecting 2");

    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn midi_settings_connect_without_ports_reports_warning() {
    let mut app = App {
        midi_ports: Vec::new(),
        mode: AppMode::MidiSettings,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::MidiSettings);
    assert_eq!(app.midi_status, "MIDI No Outputs");
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("No MIDI output selected")
    );
    assert!(!app.dirty);
}

#[test]
fn f4_opens_midi_settings_without_mutating_song() {
    let mut app = App::default();
    let song = app.song.clone();

    app.handle_key(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::MidiSettings);
    assert_eq!(app.song, song);
    assert!(!app.dirty);
}

#[test]
fn f5_refreshes_midi_settings_without_mutating_song() {
    let mut app = App {
        mode: AppMode::MidiSettings,
        ..App::default()
    };
    let song = app.song.clone();

    app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::MidiSettings);
    assert_eq!(app.song, song);
    assert!(!app.dirty);
}

#[test]
fn scrolls_down_to_keep_cursor_visible() {
    let mut app = App {
        cursor: Cursor {
            row: 20,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.keep_cursor_visible(10);

    assert_eq!(app.row_offset, 11);
}

#[test]
fn scrolls_up_to_keep_cursor_visible() {
    let mut app = App {
        cursor: Cursor {
            row: 5,
            ..Cursor::new()
        },
        row_offset: 20,
        ..App::default()
    };

    app.keep_cursor_visible(10);

    assert_eq!(app.row_offset, 5);
}

#[test]
fn scroll_offset_is_clamped_near_pattern_end() {
    let mut app = App {
        cursor: Cursor {
            row: 63,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.keep_cursor_visible(20);

    assert_eq!(app.row_offset, 44);
}

#[test]
fn scrolls_to_keep_playhead_visible_while_playing() {
    let mut app = App {
        cursor: Cursor {
            row: 0,
            ..Cursor::new()
        },
        is_playing: true,
        playhead_row: Some(20),
        ..App::default()
    };

    app.keep_active_row_visible(10);

    assert_eq!(app.row_offset, 11);
}

#[test]
fn scrolls_right_to_keep_active_track_visible() {
    let mut app = App {
        cursor: Cursor {
            track: 3,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.keep_track_visible(app.cursor.track, 2);

    assert_eq!(app.track_offset, 2);
}

#[test]
fn scrolls_left_to_keep_active_track_visible() {
    let mut app = App {
        cursor: Cursor {
            track: 0,
            ..Cursor::new()
        },
        track_offset: 2,
        ..App::default()
    };

    app.keep_track_visible(app.cursor.track, 2);

    assert_eq!(app.track_offset, 0);
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
fn tab_and_backtab_move_between_tracks() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(app.cursor.track, 1);

    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.cursor.track, 0);

    app.mode = AppMode::Edit;
    for _ in 0..10 {
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    }
    assert_eq!(app.cursor.track, 3);

    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert_eq!(app.cursor.track, 2);
}

#[test]
fn edit_mode_inserts_note_and_advances_cursor() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(app.mode, AppMode::Edit);
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(cell.velocity, Some(DEFAULT_NOTE_VELOCITY));
    assert_eq!(app.cursor.row, 1);
    assert!(app.dirty);
}

#[test]
fn edit_mode_inserts_note_off_and_note_cut() {
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));

    let pattern = app.song.current_pattern().expect("pattern");
    let off = pattern.cell(0, 0).expect("note off cell");
    let cut = pattern.cell(1, 0).expect("note cut cell");
    assert_eq!(off.note, Some(NoteEvent::NoteOff));
    assert_eq!(off.velocity, None);
    assert_eq!(cut.note, Some(NoteEvent::NoteCut));
    assert_eq!(cut.velocity, None);
    assert_eq!(app.cursor.row, 2);
}

#[test]
fn velocity_entry_uses_two_hex_digits() {
    let mut app = App {
        mode: AppMode::Edit,
        cursor: Cursor {
            field: CellField::Velocity,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));
    assert_eq!(app.cursor.row, 0);
    assert_eq!(app.cursor.digit, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(cell.velocity, Some(0x4f));
    assert_eq!(app.cursor.row, 1);
    assert_eq!(app.cursor.digit, 0);
}

#[test]
fn edit_mode_hex_entry_updates_tracker_subcolumns() {
    let mut app = App {
        mode: AppMode::Edit,
        cursor: Cursor {
            field: CellField::Delay,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE));

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(cell.delay, Some(0x20));
    assert_eq!(app.cursor.row, 1);
}

#[test]
fn clipboard_copies_cuts_and_pastes_current_cell() {
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.cursor.row = 0;

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    app.cursor.row = 4;
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(4, 0)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 60 })
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(4, 0)
            .expect("cell"),
        &PatternCell::default()
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(4, 0)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 60 })
    );
}

#[test]
fn selection_region_can_be_copied_cut_pasted_and_deleted() {
    let mut app = App::default();
    {
        let pattern = app.song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set note");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 62 }, 0x7f)
            .expect("set note");
        pattern
            .set_note(1, 0, NoteEvent::Note { pitch: 64 }, 0x7f)
            .expect("set note");
        pattern
            .set_note(1, 1, NoteEvent::Note { pitch: 65 }, 0x7f)
            .expect("set note");
    }

    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(
        app.selection_rect(),
        Some(SelectionRect {
            row_start: 0,
            row_end: 1,
            track_start: 0,
            track_end: 1,
        })
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    app.cursor.row = 4;
    app.cursor.track = 2;
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(5, 3)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 65 })
    );

    app.cursor.row = 0;
    app.cursor.track = 0;
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_eq!(app.selection_rect(), None);
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(1, 1)
            .expect("cell"),
        &PatternCell::default()
    );

    app.cursor.row = 8;
    app.cursor.track = 0;
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(9, 1)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 65 })
    );

    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(8, 0)
            .expect("cell"),
        &PatternCell::default()
    );
}

#[test]
fn parameter_locks_follow_cell_copy_paste_and_clear() {
    let mut app = App::default();
    let sample = app
        .song
        .upsert_sample_reference("samples/kick.wav", "kick.wav");
    let track = app.song.tracks[0].id;
    app.song
        .assign_sample_to_track(track, sample)
        .expect("assign sample");
    let destination_track = app.song.tracks[1].id;
    app.song
        .assign_sample_to_track(destination_track, sample)
        .expect("assign destination sample");
    type_command(&mut app, "plock sample-gain 0.500");

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    app.cursor.row = 2;
    app.cursor.track = 1;
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

    let pasted = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(2, 1)
        .expect("cell");
    assert_eq!(pasted.parameter_locks.len(), 1);
    assert_eq!(
        pasted.parameter_locks[0].parameter,
        ParameterId::from(SAMPLE_GAIN_PARAMETER_ID)
    );

    type_command(&mut app, "plock sample-gain clear");
    assert!(app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(2, 1)
        .expect("cell")
        .parameter_locks
        .is_empty());
}

#[test]
fn insert_and_ctrl_delete_edit_pattern_rows() {
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    app.cursor.row = 0;

    app.handle_key(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));

    let pattern = app.song.current_pattern().expect("pattern");
    assert_eq!(pattern.row_count(), 65);
    assert_eq!(pattern.cell(0, 0), Some(&PatternCell::default()));
    assert_eq!(
        pattern.cell(1, 0).expect("cell").note,
        Some(NoteEvent::Note { pitch: 60 })
    );

    app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL));
    assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 65);
}

#[test]
fn undo_and_redo_restore_song_snapshots() {
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell"),
        &salieri_core::PatternCell::default()
    );
    assert!(!app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert!(app.dirty);
}
