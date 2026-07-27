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
    type_command(&mut app, "fx2 R 04");

    let cell = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert_eq!(cell.command, Some(TrackerCommand::delay(0x20)));
    assert_eq!(cell.command2, Some(TrackerCommand::retrigger(0x04)));
    assert!(app.dirty);

    type_command(&mut app, "fx clear");
    type_command(&mut app, "fx2 clear");

    let cell = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert_eq!(cell.command, None);
    assert_eq!(cell.command2, None);
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
    type_command(&mut app, "cell effect2 D 10");

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(cell.instrument, Some(InstrumentId(1)));
    assert_eq!(cell.volume, Some(0x40));
    assert_eq!(cell.pan, Some(0x7f));
    assert_eq!(cell.delay, Some(0x20));
    assert_eq!(cell.command, Some(TrackerCommand::retrigger(0x04)));
    assert_eq!(cell.command2, Some(TrackerCommand::delay(0x10)));

    type_command(&mut app, "cell instrument clear");
    type_command(&mut app, "cell volume clear");
    type_command(&mut app, "cell pan clear");
    type_command(&mut app, "cell delay clear");
    type_command(&mut app, "cell effect clear");
    type_command(&mut app, "cell effect2 clear");

    let pattern = app.song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 0).expect("cell");
    assert_eq!(cell.instrument, None);
    assert_eq!(cell.volume, None);
    assert_eq!(cell.pan, None);
    assert_eq!(cell.delay, None);
    assert_eq!(cell.command, None);
    assert_eq!(cell.command2, None);
}

#[test]
fn command_mode_rejects_deferred_or_invalid_effect_commands() {
    let mut app = App::default();

    type_command(&mut app, "fx V 40");
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("deferred"));
    assert_eq!(
        app.song
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell")
            .command,
        None
    );

    type_command(&mut app, "fx R 00");
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("outside"));
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
fn command_mode_sets_resets_and_clears_parameter_locks() {
    let mut app = App::default();
    let sample = app
        .song
        .upsert_sample_reference("samples/kick.wav", "kick.wav");
    let track = app.song.tracks[0].id;
    app.song
        .assign_sample_to_track(track, sample)
        .expect("assign sample");
    type_command(&mut app, "dsp track gain 1.000");

    type_command(&mut app, "plock sample-gain 0.250");
    type_command(&mut app, "plock mixer pan -0.500");
    type_command(&mut app, "plock send 1 0.500");
    type_command(&mut app, "plock dsp track gain 0.750");
    type_command(&mut app, "plock dsp track width 1.500");
    type_command(&mut app, "plock dsp track filter-cutoff 2000");
    type_command(&mut app, "plock dsp track filter-mode High-pass");
    type_command(&mut app, "plock dsp track delay-left 250");
    type_command(&mut app, "plock dsp track delay-mix 0.500");
    type_command(&mut app, "plock dsp track reverb-decay 3.500");
    type_command(&mut app, "plock dsp track drive-tone 0.250");
    type_command(&mut app, "plock dsp track bit-depth 8");
    type_command(&mut app, "plock dsp track chorus-rate 0.500");
    type_command(&mut app, "plock dsp track flanger-feedback 0.250");
    type_command(&mut app, "plock dsp track phaser-stages 6");

    let cell = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert_eq!(cell.parameter_locks.len(), 15);
    assert_eq!(
        cell.parameter_locks[0].parameter,
        ParameterId::from(SAMPLE_GAIN_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[1].parameter,
        ParameterId::from(MIXER_TRACK_PAN_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[2].target,
        ParameterLockTarget::TrackSend { track, send: 1 }
    );
    assert_eq!(
        cell.parameter_locks[3].target,
        ParameterLockTarget::TrackEffect { track, device: 1 }
    );
    assert_eq!(
        cell.parameter_locks[4].parameter,
        ParameterId::from(NATIVE_WIDTH_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[4].target,
        ParameterLockTarget::TrackEffect { track, device: 4 }
    );
    assert_eq!(
        cell.parameter_locks[5].parameter,
        ParameterId::from(NATIVE_FILTER_CUTOFF_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[6].parameter,
        ParameterId::from(NATIVE_FILTER_MODE_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[7].parameter,
        ParameterId::from(NATIVE_DELAY_TIME_LEFT_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[7].target,
        ParameterLockTarget::TrackEffect { track, device: 7 }
    );
    assert_eq!(
        cell.parameter_locks[8].parameter,
        ParameterId::from(NATIVE_DELAY_MIX_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[9].parameter,
        ParameterId::from(NATIVE_REVERB_DECAY_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[9].target,
        ParameterLockTarget::TrackEffect { track, device: 8 }
    );
    assert_eq!(
        cell.parameter_locks[10].parameter,
        ParameterId::from(NATIVE_DRIVE_TONE_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[10].target,
        ParameterLockTarget::TrackEffect { track, device: 9 }
    );
    assert_eq!(
        cell.parameter_locks[11].parameter,
        ParameterId::from(NATIVE_BITCRUSHER_BIT_DEPTH_PARAMETER_ID)
    );
    assert_eq!(
        cell.parameter_locks[11].target,
        ParameterLockTarget::TrackEffect { track, device: 10 }
    );
    assert_eq!(
        cell.parameter_locks[12].target,
        ParameterLockTarget::TrackEffect { track, device: 11 }
    );
    assert_eq!(
        cell.parameter_locks[13].target,
        ParameterLockTarget::TrackEffect { track, device: 12 }
    );
    assert_eq!(
        cell.parameter_locks[14].target,
        ParameterLockTarget::TrackEffect { track, device: 13 }
    );

    type_command(&mut app, "plock sample-gain reset");
    let cell = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert!(matches!(
        cell.parameter_locks[0].action,
        ParameterLockAction::Reset
    ));

    type_command(&mut app, "plock sample-gain clear");
    let cell = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert_eq!(cell.parameter_locks.len(), 14);
    assert!(app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    let cell = app
        .song
        .current_pattern()
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert_eq!(cell.parameter_locks.len(), 15);
}

#[test]
fn command_mode_edits_mixer_state() {
    let mut app = App::default();

    type_command(&mut app, "mixer gain 2 0.500");
    type_command(&mut app, "mixer pan 2 -0.250");
    type_command(&mut app, "mixer mute 2");
    type_command(&mut app, "mixer solo 2");
    type_command(&mut app, "mixer master 0.800");
    type_command(&mut app, "mixer send delay");
    type_command(&mut app, "mixer send delay pre");
    type_command(&mut app, "mixer send delay gain 2 0.375");
    type_command(&mut app, "mixer send reverb");
    type_command(&mut app, "mixer send reverb gain 0.250");
    type_command(&mut app, "mixer send list");

    let track_id = app.song.tracks[1].id;
    let mixer = app.song.track_mixer_for_track(track_id);
    assert_eq!(mixer.gain, 0.5);
    assert_eq!(mixer.pan, -0.25);
    assert!(mixer.muted);
    assert!(mixer.solo);
    assert_eq!(app.song.mixer.sends.len(), 2);
    assert!(app.song.mixer.sends[0].pre_fader);
    assert_eq!(app.song.mixer.sends[0].name, "Delay");
    assert!(matches!(
        app.song.mixer.sends[0].effects[0].kind,
        EffectDeviceKind::Delay { .. }
    ));
    assert!(matches!(
        app.song.mixer.sends[1].effects[0].kind,
        EffectDeviceKind::Reverb { .. }
    ));
    assert_eq!(mixer.sends[0].send, 1);
    assert_eq!(mixer.sends[0].gain, 0.375);
    let current_track_mixer = app.song.track_mixer_for_track(app.song.tracks[0].id);
    assert_eq!(current_track_mixer.sends[0].send, 2);
    assert_eq!(current_track_mixer.sends[0].gain, 0.25);
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
    type_command(&mut app, "dsp track 2 balance 0.250");
    type_command(&mut app, "dsp track 2 width 1.500");
    type_command(&mut app, "dsp track 2 phase on off");
    type_command(
        &mut app,
        "dsp track 2 filter lowpass 2000 0.500 3.000 0.750",
    );
    type_command(&mut app, "dsp track 2 delay free 250 500 0.350 0.250 ping");
    type_command(&mut app, "dsp track 2 reverb 0.600 10 3.000 0.400");
    type_command(&mut app, "dsp track 2 drive hardclip 18.000 0.250 0.500");
    type_command(&mut app, "dsp track 2 bitcrusher 8 4 0.750 on");
    type_command(&mut app, "dsp track 2 chorus 0.500 0.750 12 2 1.000 0.500");
    type_command(
        &mut app,
        "dsp track 2 flanger 0.500 0.750 0.500 0.250 1.000 0.500",
    );
    type_command(
        &mut app,
        "dsp track 2 phaser 0.500 0.750 1000 4 0.250 1.000 0.500",
    );
    type_command(&mut app, "dsp master gain 0.800");
    type_command(&mut app, "dsp master width 0.750");
    type_command(&mut app, "dsp master phase false true");
    type_command(&mut app, "dsp master filter notch 4000 0.250 0.000 0.500");
    type_command(&mut app, "dsp master delay sync 500 500 0.250 0.500");
    type_command(&mut app, "dsp master reverb 0.500 20 2.500 0.250");
    type_command(&mut app, "dsp master drive saturation 12.000 0.500 0.600");
    type_command(&mut app, "dsp master crusher 6 8 0.400");

    let track_id = app.song.tracks[1].id;
    let mixer = app.song.track_mixer_for_track(track_id);
    assert_eq!(mixer.effects.len(), 13);
    assert_eq!(mixer.effects[0].kind, EffectDeviceKind::Gain { gain: 0.5 });
    assert_eq!(mixer.effects[1].kind, EffectDeviceKind::Pan { pan: -0.25 });
    assert_eq!(
        mixer.effects[2].kind,
        EffectDeviceKind::Balance { balance: 0.25 }
    );
    assert_eq!(
        mixer.effects[3].kind,
        EffectDeviceKind::StereoWidth { width: 1.5 }
    );
    assert_eq!(
        mixer.effects[4].kind,
        EffectDeviceKind::PhaseInvert {
            invert_left: true,
            invert_right: false
        }
    );
    assert_eq!(
        mixer.effects[5].kind,
        EffectDeviceKind::Filter {
            mode: FilterMode::LowPass,
            cutoff_hz: 2000.0,
            resonance: 0.5,
            drive_db: 3.0,
            key_track: 0.0,
            env_amount: 0.0,
            mix: 0.75
        }
    );
    assert_eq!(
        mixer.effects[6].kind,
        EffectDeviceKind::Delay {
            sync: false,
            time_left_ms: 250.0,
            time_right_ms: 500.0,
            link_times: false,
            feedback: 0.35,
            ping_pong: true,
            filter_low_cut_hz: 20.0,
            filter_high_cut_hz: 20_000.0,
            mod_rate_hz: 0.0,
            mod_depth: 0.0,
            mix: 0.25,
            output_db: 0.0
        }
    );
    assert_eq!(
        mixer.effects[7].kind,
        EffectDeviceKind::Reverb {
            size: 0.6,
            predelay_ms: 10.0,
            decay_s: 3.0,
            damping: 0.5,
            low_cut_hz: 100.0,
            high_cut_hz: 16_000.0,
            diffusion: 0.75,
            width: 1.0,
            early_reflections: 0.5,
            mix: 0.4,
            output_db: 0.0
        }
    );
    assert_eq!(
        mixer.effects[8].kind,
        EffectDeviceKind::Drive {
            mode: DriveMode::HardClip,
            drive_db: 18.0,
            tone: 0.25,
            bias: 0.0,
            mix: 0.5,
            output_db: 0.0
        }
    );
    assert_eq!(
        mixer.effects[9].kind,
        EffectDeviceKind::Bitcrusher {
            bit_depth: 8,
            reduction_ratio: 4.0,
            dither: true,
            mix: 0.75,
            output_db: 0.0
        }
    );
    assert!(matches!(
        mixer.effects[10].kind,
        EffectDeviceKind::Chorus { voices: 2, .. }
    ));
    assert!(matches!(
        mixer.effects[11].kind,
        EffectDeviceKind::Flanger { feedback: 0.25, .. }
    ));
    assert!(matches!(
        mixer.effects[12].kind,
        EffectDeviceKind::Phaser { stages: 4, .. }
    ));
    assert_eq!(app.song.mixer.master_effects.len(), 8);
    assert_eq!(
        app.song.mixer.master_effects[0].kind,
        EffectDeviceKind::Gain { gain: 0.8 }
    );
    assert_eq!(
        app.song.mixer.master_effects[1].kind,
        EffectDeviceKind::StereoWidth { width: 0.75 }
    );
    assert_eq!(
        app.song.mixer.master_effects[2].kind,
        EffectDeviceKind::PhaseInvert {
            invert_left: false,
            invert_right: true
        }
    );
    assert_eq!(
        app.song.mixer.master_effects[3].kind,
        EffectDeviceKind::Filter {
            mode: FilterMode::Notch,
            cutoff_hz: 4000.0,
            resonance: 0.25,
            drive_db: 0.0,
            key_track: 0.0,
            env_amount: 0.0,
            mix: 0.5
        }
    );
    assert_eq!(
        app.song.mixer.master_effects[4].kind,
        EffectDeviceKind::Delay {
            sync: true,
            time_left_ms: 500.0,
            time_right_ms: 500.0,
            link_times: true,
            feedback: 0.25,
            ping_pong: false,
            filter_low_cut_hz: 20.0,
            filter_high_cut_hz: 20_000.0,
            mod_rate_hz: 0.0,
            mod_depth: 0.0,
            mix: 0.5,
            output_db: 0.0
        }
    );
    assert_eq!(
        app.song.mixer.master_effects[5].kind,
        EffectDeviceKind::Reverb {
            size: 0.5,
            predelay_ms: 20.0,
            decay_s: 2.5,
            damping: 0.5,
            low_cut_hz: 100.0,
            high_cut_hz: 16_000.0,
            diffusion: 0.75,
            width: 1.0,
            early_reflections: 0.5,
            mix: 0.25,
            output_db: 0.0
        }
    );
    assert_eq!(
        app.song.mixer.master_effects[6].kind,
        EffectDeviceKind::Drive {
            mode: DriveMode::Saturation,
            drive_db: 12.0,
            tone: 0.5,
            bias: 0.0,
            mix: 0.6,
            output_db: 0.0
        }
    );
    assert_eq!(
        app.song.mixer.master_effects[7].kind,
        EffectDeviceKind::Bitcrusher {
            bit_depth: 6,
            reduction_ratio: 8.0,
            dither: false,
            mix: 0.4,
            output_db: 0.0
        }
    );
    assert!(app.dirty);

    type_command(&mut app, "dsp track 2 clear");
    type_command(&mut app, "dsp master clear");

    let mixer = app.song.track_mixer_for_track(track_id);
    assert!(mixer.effects.is_empty());
    assert!(app.song.mixer.master_effects.is_empty());
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
    let path = std::env::temp_dir().join(format!("trk-command-write-{}.trk", std::process::id()));
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
    let path =
        std::env::temp_dir().join(format!("trk-command-write-as-{}.trk", std::process::id()));
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
    let path = std::env::temp_dir().join(format!("trk-quit-save-{}.trk", std::process::id()));
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
    let mut song = Song::empty();
    song.midi.clock_in = true;
    song.midi.transport_in = true;
    let mut app = App {
        song,
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
