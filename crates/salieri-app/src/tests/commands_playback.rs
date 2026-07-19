use super::*;

#[test]
fn command_mode_sets_bpm_and_lpb() {
    let mut app = App::default();

    type_command(&mut app, "bpm 140");
    type_command(&mut app, "lpb 8");

    assert_eq!(app.song.transport.bpm, 140);
    assert_eq!(app.song.transport.lines_per_beat, 8);
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.transport.lines_per_beat, 4);
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.song.transport.bpm, 120);
    assert!(!app.dirty);
}

#[test]
fn control_arrows_adjust_bpm_and_lpb() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
    assert_eq!(app.song.transport.bpm, 121);
    assert!(app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("BPM 121")
    );

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
    assert_eq!(app.song.transport.bpm, 120);
    assert!(!app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));
    assert_eq!(app.song.transport.lines_per_beat, 5);
    assert!(app.dirty);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("LPB 5")
    );

    app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL));
    assert_eq!(app.song.transport.lines_per_beat, 4);
    assert!(!app.dirty);

    app.song.transport.bpm = MIN_BPM;
    app.song.transport.lines_per_beat = MAX_LPB;
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));

    assert_eq!(app.song.transport.bpm, MIN_BPM);
    assert_eq!(app.song.transport.lines_per_beat, MAX_LPB);
}

#[test]
fn command_mode_sets_pattern_loop() {
    let mut app = App::default();

    assert!(app.loop_pattern);
    type_command(&mut app, "loop off");
    assert!(!app.loop_pattern);
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Pattern loop OFF")
    );
    type_command(&mut app, "loop on");
    assert!(app.loop_pattern);
    type_command(&mut app, "loop");
    assert!(!app.loop_pattern);
}

#[test]
fn command_mode_sets_and_clears_current_effect_command() {
    let mut app = App::default();

    type_command(&mut app, "fx D 20");

    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell")
            .command,
        Some(TrackerCommand::delay(0x20))
    );
    assert!(app.dirty);

    type_command(&mut app, "fx clear");

    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell")
            .command,
        None
    );
    assert!(!app.dirty);
}

#[test]
fn command_mode_sets_and_clears_tracker_cell_columns() {
    let mut app = App::default();

    type_command(&mut app, "cell instrument 01");
    type_command(&mut app, "cell volume 40");
    type_command(&mut app, "cell pan 7f");
    type_command(&mut app, "cell delay 20");
    type_command(&mut app, "cell effect R 04");

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(cell.instrument, Some(InstrumentId(1)));
    assert_eq!(cell.volume, Some(0x40));
    assert_eq!(cell.pan, Some(0x7f));
    assert_eq!(cell.delay, Some(0x20));
    assert_eq!(cell.command, Some(TrackerCommand::retrigger(0x04)));

    type_command(&mut app, "cell instrument clear");
    type_command(&mut app, "cell volume clear");
    type_command(&mut app, "cell pan clear");
    type_command(&mut app, "cell delay clear");
    type_command(&mut app, "cell effect clear");

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(cell.instrument, None);
    assert_eq!(cell.volume, None);
    assert_eq!(cell.pan, None);
    assert_eq!(cell.delay, None);
    assert_eq!(cell.command, None);
}

#[test]
fn command_mode_sets_and_clears_sample_gain_automation() {
    let mut app = App::default();
    let sample = app
        .song
        .upsert_sample_reference("samples/kick.wav", "kick.wav");
    let track = app.song.tracks[0].id;
    app.song
        .assign_sample_to_track(track, sample)
        .expect("assign sample");

    type_command(&mut app, "automation sample-gain 4 0.250");

    let pattern = app.song.current_pattern().expect("pattern");
    assert_eq!(
        pattern.automation_value_at(AutomationTarget::SampleGain { sample }, 3, 1.0),
        1.0
    );
    assert_eq!(
        pattern.automation_value_at(AutomationTarget::SampleGain { sample }, 4, 1.0),
        0.25
    );
    assert!(app.dirty);

    type_command(&mut app, "automation sample-gain clear 4");

    assert!(app
        .song
        .current_pattern()
        .expect("pattern")
        .automation
        .is_empty());
}

#[test]
fn command_mode_edits_mixer_state() {
    let mut app = App::default();

    type_command(&mut app, "mixer gain 2 0.500");
    type_command(&mut app, "mixer pan 2 -0.250");
    type_command(&mut app, "mixer mute 2");
    type_command(&mut app, "mixer solo 2");
    type_command(&mut app, "mixer master 0.800");

    let track_id = app.song.tracks[1].id;
    let mixer = app.song.track_mixer_for_track(track_id);
    assert_eq!(mixer.gain, 0.5);
    assert_eq!(mixer.pan, -0.25);
    assert!(mixer.muted);
    assert!(mixer.solo);
    assert_eq!(app.song.mixer.master_gain, 0.8);
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
    assert_eq!(app.tui_active_view(), TuiView::Tracks);
}

#[test]
fn command_mode_edits_dsp_chains() {
    let mut app = App::default();

    type_command(&mut app, "dsp track 2 gain 0.500");
    type_command(&mut app, "dsp track 2 pan -0.250");
    type_command(&mut app, "dsp master gain 0.800");

    let track_id = app.song.tracks[1].id;
    let mixer = app.song.track_mixer_for_track(track_id);
    assert_eq!(mixer.effects.len(), 2);
    assert_eq!(mixer.effects[0].kind, EffectDeviceKind::Gain { gain: 0.5 });
    assert_eq!(mixer.effects[1].kind, EffectDeviceKind::Pan { pan: -0.25 });
    assert_eq!(app.song.mixer.master_effects.len(), 1);
    assert_eq!(
        app.song.mixer.master_effects[0].kind,
        EffectDeviceKind::Gain { gain: 0.8 }
    );
    assert!(app.dirty);

    type_command(&mut app, "dsp track 2 clear");
    type_command(&mut app, "dsp master clear");

    let mixer = app.song.track_mixer_for_track(track_id);
    assert!(mixer.effects.is_empty());
    assert!(app.song.mixer.master_effects.is_empty());
}

#[test]
fn command_mode_ai_proposal_preview_apply_and_undo() {
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "ai propose sparse bass sketch");
    app.wait_for_tasks();

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_some());
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("touches"));

    type_command(&mut app, "ai accept");

    assert_ne!(app.song, before);
    assert!(app.pending_ai_proposal.is_none());
    assert!(app.dirty);

    app.undo();

    assert_eq!(app.song, before);
}

#[test]
fn command_mode_ai_proposal_can_be_rejected_without_mutating_song() {
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "ai propose lead idea");
    app.wait_for_tasks();
    type_command(&mut app, "ai reject");

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_none());
}

#[test]
fn command_mode_reports_unknown_commands() {
    let mut app = App::default();

    type_command(&mut app, "doesnotexist");

    let notification = app.notification.as_ref().expect("notification");
    assert_eq!(notification.kind, NotificationKind::Warning);
    assert_eq!(notification.message, "Unknown command: doesnotexist");
}

#[test]
fn command_mode_write_saves_project() {
    let path = std::env::temp_dir().join(format!(
        "salieri-command-write-{}.salieri",
        std::process::id()
    ));
    let mut app = App {
        mode: AppMode::Edit,
        project_path: Some(path.clone()),
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    type_command(&mut app, "write");

    let saved = load_project(&path).expect("saved project loads");
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved, app.song);
    assert!(!app.dirty);
    assert!(!app.should_quit);
}

#[test]
fn command_mode_write_accepts_project_path() {
    let path = std::env::temp_dir().join(format!(
        "salieri-command-write-as-{}.salieri",
        std::process::id()
    ));
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    type_command(&mut app, &format!("write {}", path.display()));

    let saved = load_project(&path).expect("saved project loads");
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved, app.song);
    assert_eq!(app.project_path, Some(path));
    assert!(!app.dirty);
}

#[test]
fn command_mode_quit_marks_app_for_exit() {
    let mut app = App::default();

    type_command(&mut app, "quit");

    assert!(app.should_quit);
}

#[test]
fn dirty_quit_opens_confirmation_dialog() {
    let mut app = App::default();

    app.set_bpm(140);
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Dialog);
    assert!(!app.should_quit);

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert!(!app.should_quit);
}

#[test]
fn dirty_quit_can_discard_changes() {
    let mut app = App::default();

    app.set_bpm(140);
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

    assert!(app.should_quit);
}

#[test]
fn dirty_quit_can_save_before_exit() {
    let path =
        std::env::temp_dir().join(format!("salieri-quit-save-{}.salieri", std::process::id()));
    let mut app = App {
        project_path: Some(path.clone()),
        ..App::default()
    };

    app.set_bpm(140);
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let saved = load_project(&path).expect("saved project loads");
    let _ = std::fs::remove_file(&path);
    assert_eq!(saved.transport.bpm, 140);
    assert!(!app.dirty);
    assert!(app.should_quit);
}

#[test]
fn force_quit_command_bypasses_dirty_confirmation() {
    let mut app = App::default();

    app.set_bpm(140);
    type_command(&mut app, "q!");

    assert_ne!(app.mode, AppMode::Dialog);
    assert!(app.should_quit);
}

#[test]
fn space_toggles_playback_and_f8_stops() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));

    app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));

    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);
}

#[test]
fn shift_space_starts_playback_from_pattern_start() {
    let mut app = App {
        cursor: Cursor {
            row: 12,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));
    assert_eq!(app.sequence_position, None);
}

#[test]
fn uppercase_l_toggles_pattern_loop_without_breaking_vim_right() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));
    assert!(!app.loop_pattern);

    app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(app.cursor.field, CellField::Velocity);
    assert!(!app.loop_pattern);
}

#[test]
fn enter_starts_playback_from_cursor_row() {
    let mut app = App {
        cursor: Cursor {
            row: 12,
            ..Cursor::new()
        },
        ..App::default()
    };

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(12));
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn command_mode_requests_midi_connection_and_panic_stops_playback() {
    let mut app = App::default();

    type_command(&mut app, "midi connect 3");
    assert_eq!(app.midi_status, "MIDI Connecting 3");

    app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
    assert!(app.is_playing);

    type_command(&mut app, "midi panic");
    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);
}

#[test]
fn command_mode_edits_midi_input_record_and_clock_state() {
    let mut app = App::default();

    type_command(&mut app, "midi-input record on");
    assert!(app.midi_record_armed);
    assert_eq!(app.tui_midi_status(), "MIDI Disconnected | MIDI In Rec");

    type_command(&mut app, "midi-in clock on");
    assert!(app.midi_clock_follow);
    assert_eq!(
        app.tui_midi_status(),
        "MIDI Disconnected | MIDI In Rec+Clock"
    );

    type_command(&mut app, "midi-input disconnect");
    assert!(!app.midi_record_armed);
    assert!(!app.midi_clock_follow);
    assert_eq!(app.midi_input_status, "MIDI In Disconnected");
}

#[test]
fn midi_input_recording_drains_fake_input_and_is_undoable() {
    let packet = MidiInputPacket {
        timestamp_micros: 0,
        event: MidiInputEvent::NoteOn {
            channel: 1,
            note: 60,
            velocity: 100,
        },
    };
    let mut app = App {
        midi_input: Some(AppMidiInput::new(FakeMidiInput::new([packet]))),
        midi_record_armed: true,
        ..App::default()
    };

    app.drain_midi_input();

    let cell = app
        .song
        .pattern(0)
        .and_then(|pattern| pattern.cell(0, 0))
        .expect("recorded cell");
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(cell.velocity, Some(100));
    assert_eq!(app.cursor.row, 1);
    assert!(app.dirty);

    app.undo();
    let cell = app
        .song
        .pattern(0)
        .and_then(|pattern| pattern.cell(0, 0))
        .expect("undo cell");
    assert_eq!(cell, &PatternCell::default());
}

#[test]
fn midi_clock_follow_controls_transport() {
    let mut app = App {
        midi_clock_follow: true,
        ..App::default()
    };

    app.handle_midi_input_packet(MidiInputPacket {
        timestamp_micros: 0,
        event: MidiInputEvent::Clock(MidiClockMessage::Start),
    });
    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));

    app.handle_midi_input_packet(MidiInputPacket {
        timestamp_micros: 1,
        event: MidiInputEvent::Clock(MidiClockMessage::TimingClock),
    });
    assert_eq!(app.midi_clock_ticks, 1);
    assert_eq!(app.midi_input_status, "MIDI In Clock 1");

    app.handle_midi_input_packet(MidiInputPacket {
        timestamp_micros: 2,
        event: MidiInputEvent::Clock(MidiClockMessage::Stop),
    });
    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);
}

#[test]
fn command_mode_can_start_sequence_playback() {
    let mut app = App::default();

    type_command(&mut app, "play sequence");

    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));
    assert_eq!(app.sequence_position, Some(0));

    type_command(&mut app, "stop");

    assert!(!app.is_playing);
    assert_eq!(app.sequence_position, None);
}

#[test]
fn command_mode_can_start_sequence_from_position() {
    let mut app = App::default();
    type_command(&mut app, "pattern new");
    type_command(&mut app, "sequence add 2");

    type_command(&mut app, "play sequence 1");

    assert!(app.is_playing);
    assert_eq!(app.pattern_index, 1);
    assert_eq!(app.playhead_row, Some(0));
    assert_eq!(app.sequence_position, Some(1));
}

#[test]
fn shift_enter_starts_sequence_from_selected_position() {
    let mut app = App::default();
    type_command(&mut app, "pattern new");
    type_command(&mut app, "sequence add 2");
    app.sequence_cursor = 1;

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

    assert!(app.is_playing);
    assert_eq!(app.pattern_index, 1);
    assert_eq!(app.playhead_row, Some(0));
    assert_eq!(app.sequence_position, Some(1));
    assert_eq!(
        app.notification.as_ref().map(|n| n.message.as_str()),
        Some("Playing sequence from 1")
    );
}
