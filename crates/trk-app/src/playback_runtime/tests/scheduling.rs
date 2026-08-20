use super::*;

#[test]
fn runtime_starts_sequence_from_requested_position() {
    let runtime = PlaybackRuntime::spawn(None);
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    let second_pattern_id = song.create_pattern(64);
    song.push_sequence_pattern(second_pattern_id)
        .expect("add second pattern to sequence");

    runtime.start_sequence(song, None, 1);

    let deadline = Instant::now() + Duration::from_millis(250);
    let mut first_sequence_index = None;
    while Instant::now() < deadline {
        if let Some(PlaybackUpdate::Position(position)) = runtime.try_recv() {
            first_sequence_index = position.sequence_index;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    runtime.stop();

    assert_eq!(first_sequence_index, Some(1));
}

#[test]
fn pattern_playback_advances_to_next_pattern_before_stopping() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
        .expect("set first pattern note");
    let second_pattern_id = song.create_pattern(64);
    let second_pattern_index = song
        .patterns
        .iter()
        .position(|pattern| pattern.id == second_pattern_id)
        .expect("second pattern");
    song.pattern_mut(second_pattern_index)
        .expect("second pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 72 }, 0x50)
        .expect("set second pattern note");

    let (next_command, sent, updates) = run_pattern_chain_with_recording(&song, 0, false, None);

    assert!(next_command.is_none());
    assert!(sent.contains(&MidiMessage::note_on(10, 60, 0x64)));
    assert!(sent.contains(&MidiMessage::note_on(10, 72, 0x50)));
    assert!(updates.iter().any(|update| {
        matches!(
            update,
            PlaybackUpdate::Position(PlaybackCursor {
                pattern_index: 1,
                position: PlaybackPosition { row: 0, .. },
                ..
            })
        )
    }));
    assert!(updates
        .iter()
        .any(|update| matches!(update, PlaybackUpdate::Stopped)));
}

#[test]
fn runtime_position_intervals_track_row_duration_with_tolerance() {
    let runtime = PlaybackRuntime::spawn(None);
    let mut song = Song::empty();
    song.transport.bpm = 300;
    song.transport.lines_per_beat = 4;
    let expected = Duration::from_micros(row_duration_micros(&song.transport));

    runtime.start_pattern_from(song, None, 0, 0, true);

    let positions = collect_position_times(&runtime, 6, Duration::from_millis(500));
    runtime.stop();

    let intervals: Vec<_> = positions
        .windows(2)
        .filter_map(|pair| {
            let (previous_row, previous_time) = pair[0];
            let (next_row, next_time) = pair[1];
            (next_row == previous_row + 1).then_some(next_time.duration_since(previous_time))
        })
        .take(4)
        .collect();

    assert!(
        intervals.len() >= 4,
        "expected at least four sequential row intervals, got {positions:?}"
    );

    let tolerance = Duration::from_millis(35);
    for interval in intervals {
        let drift = interval.abs_diff(expected);
        assert!(
            drift <= tolerance,
            "row interval {interval:?} drifted more than {tolerance:?} from {expected:?}"
        );
    }
}

#[test]
fn playback_thread_advances_without_tui_polling() {
    let runtime = PlaybackRuntime::spawn(None);
    let mut song = Song::empty();
    song.transport.bpm = 300;
    song.transport.lines_per_beat = 4;
    let row_duration = Duration::from_micros(row_duration_micros(&song.transport));

    runtime.start_pattern_from(song, None, 0, 0, true);
    thread::sleep(row_duration.saturating_mul(5) + Duration::from_millis(30));

    let positions = collect_position_times(&runtime, 16, Duration::from_millis(100));
    runtime.stop();

    assert!(
        positions.iter().any(|(row, _)| *row >= 4),
        "playback did not advance while the test withheld TUI polling: {positions:?}"
    );
}

#[test]
fn runtime_writes_midi_log_when_enabled() {
    let path = std::env::temp_dir().join(format!("trk-midi-log-{}.log", std::process::id()));
    let runtime = PlaybackRuntime::spawn(Some(path.clone()));
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("set note");

    runtime.start_pattern_from(song, None, 0, 0, true);

    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        if std::fs::read_to_string(&path).is_ok_and(|contents| contents.contains("NOTE_ON")) {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    runtime.stop();
    drop(runtime);

    let contents = std::fs::read_to_string(&path).expect("midi log");
    let _ = std::fs::remove_file(&path);

    assert!(contents.contains("NOTE_ON ch=10 note=60 velocity=127"));
}

#[test]
fn runtime_disconnects_and_stops_when_midi_send_fails() {
    let path =
        std::env::temp_dir().join(format!("trk-midi-failure-log-{}.log", std::process::id()));
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("set note");

    let (_command_tx, command_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::channel();
    let mut output = PlaybackOutput::failing();
    let mut midi_logger = MidiLogger::new(Some(path.clone()), &update_tx);
    let mut audio_output = PlaybackAudioOutput::disabled(AudioConfig::default().sample_rate);
    let audio_sample_rate = audio_output.sample_rate();
    let mut context = PlaybackRunContext {
        command_rx: &command_rx,
        update_tx: &update_tx,
        output: &mut output,
        midi_logger: &mut midi_logger,
        audio_output: &mut audio_output,
        audio_sample_rate,
    };

    let result = run_pattern(&mut song, 0, 0, None, true, &mut context);

    assert!(matches!(result, PatternRunResult::Stopped));
    assert!(matches!(output, PlaybackOutput::Fake(_)));

    let updates: Vec<_> = update_rx.try_iter().collect();
    assert!(updates
        .iter()
        .any(|update| matches!(update, PlaybackUpdate::MidiDisconnected)));
    assert!(updates
        .iter()
        .any(|update| matches!(update, PlaybackUpdate::Stopped)));
    assert!(updates.iter().any(|update| matches!(
        update,
        PlaybackUpdate::MidiError(message)
            if message.contains("MIDI output disconnected during playback")
    )));

    let contents = std::fs::read_to_string(&path).expect("midi log");
    let _ = std::fs::remove_file(&path);

    assert!(contents.contains("NOTE_ON"));
    assert!(contents.contains("SEND_ERROR stopping playback"));
    assert!(contents.contains("CC ch=1 controller=123 value=0"));
    assert!(contents.contains("ALL_NOTES_OFF_ERROR during MIDI recovery"));
}

#[test]
fn fake_midi_pattern_playback_emits_note_on_and_note_off() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x64)
        .expect("set note");
    let (_command_tx, command_rx) = mpsc::channel();

    let (result, sent, _updates) = run_pattern_with_recording(&song, 0, 0, false, &command_rx);

    assert!(matches!(result, PatternRunResult::Finished));
    assert!(sent.contains(&MidiMessage::note_on(10, 60, 0x64)));
    assert!(sent.contains(&MidiMessage::note_off(10, 60, 0)));
}

#[test]
fn fake_midi_sequence_playback_emits_each_pattern_and_panic_cleanup() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("set first note");
    let second_pattern_id = song.create_pattern(4);
    let second_pattern_index = song
        .patterns
        .iter()
        .position(|pattern| pattern.id == second_pattern_id)
        .expect("second pattern");
    song.pattern_mut(second_pattern_index)
        .expect("second pattern")
        .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x50)
        .expect("set second note");
    song.push_sequence_pattern(second_pattern_id)
        .expect("push sequence");

    let (next_command, sent, updates) = run_sequence_with_recording(song, 0);

    assert!(next_command.is_none());
    assert!(sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
    assert!(sent.contains(&MidiMessage::note_on(1, 48, 0x50)));
    assert_eq!(
        sent.iter()
            .filter(|message| matches!(
                message,
                MidiMessage::ControlChange {
                    controller: 123,
                    ..
                }
            ))
            .count(),
        16
    );
    assert!(updates
        .iter()
        .any(|update| matches!(update, PlaybackUpdate::Stopped)));
}

#[test]
fn fake_midi_stop_command_sends_all_notes_off() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    let (command_tx, command_rx) = mpsc::channel();
    command_tx.send(PlaybackCommand::Stop).expect("queue stop");

    let (result, sent, updates) = run_pattern_with_recording(&song, 0, 0, true, &command_rx);

    assert!(matches!(
        result,
        PatternRunResult::Command(command) if matches!(*command, PlaybackCommand::Stop)
    ));
    assert_eq!(sent.len(), 16);
    assert_eq!(sent[0], MidiMessage::all_notes_off(1));
    assert_eq!(sent[15], MidiMessage::all_notes_off(16));
    assert!(updates
        .iter()
        .any(|update| matches!(update, PlaybackUpdate::Stopped)));
}

#[test]
fn live_pattern_replacement_is_applied_without_stopping_transport() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(1, 0, NoteEvent::Note { pitch: 60 }, 0x60)
        .expect("set original note");
    let mut replacement = song.current_pattern().expect("pattern").clone();
    replacement
        .set_note(1, 0, NoteEvent::Note { pitch: 64 }, 0x60)
        .expect("set replacement note");
    let (command_tx, command_rx) = mpsc::channel();
    command_tx
        .send(PlaybackCommand::ReplacePattern {
            pattern_index: 0,
            pattern: replacement,
        })
        .expect("queue live replacement");

    let (result, sent, updates) = run_pattern_with_recording(&song, 0, 0, false, &command_rx);

    assert!(matches!(result, PatternRunResult::Finished));
    assert!(!sent.contains(&MidiMessage::note_on(10, 60, 0x60)));
    assert!(sent.contains(&MidiMessage::note_on(10, 64, 0x60)));
    assert!(!updates
        .iter()
        .any(|update| matches!(update, PlaybackUpdate::Stopped)));
}

#[test]
fn live_replacement_for_a_later_pattern_is_stored_without_stopping_current_playback() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    let second_id = song.create_pattern(64);
    let second_index = song
        .patterns
        .iter()
        .position(|pattern| pattern.id == second_id)
        .expect("second pattern");
    song.pattern_mut(second_index)
        .expect("second pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 62 }, 0x60)
        .expect("set original second note");
    let mut replacement = song.pattern(second_index).expect("second pattern").clone();
    replacement
        .set_note(0, 0, NoteEvent::Note { pitch: 67 }, 0x60)
        .expect("set replacement second note");

    let (next_command, sent, updates) = run_pattern_chain_with_recording(
        &song,
        0,
        false,
        Some(PlaybackCommand::ReplacePattern {
            pattern_index: second_index,
            pattern: replacement,
        }),
    );

    assert!(next_command.is_none());
    assert!(!sent.contains(&MidiMessage::note_on(10, 62, 0x60)));
    assert!(sent.contains(&MidiMessage::note_on(10, 67, 0x60)));
    assert_eq!(
        updates
            .iter()
            .filter(|update| matches!(update, PlaybackUpdate::Stopped))
            .count(),
        1
    );
}

#[test]
fn fake_midi_playback_honors_mute_and_solo() {
    let mut muted_song = Song::empty();
    speed_up_transport(&mut muted_song);
    {
        let pattern = muted_song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set drums note");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x70)
            .expect("set bass note");
    }
    muted_song.toggle_mute(0).expect("mute drums");
    let (_command_tx, command_rx) = mpsc::channel();

    let (_result, muted_sent, _updates) =
        run_pattern_with_recording(&muted_song, 0, 0, false, &command_rx);

    assert!(!muted_sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
    assert!(muted_sent.contains(&MidiMessage::note_on(1, 48, 0x70)));

    let mut solo_song = muted_song;
    solo_song.toggle_mute(0).expect("unmute drums");
    solo_song.toggle_solo(0).expect("solo drums");
    let (_command_tx, command_rx) = mpsc::channel();

    let (_result, solo_sent, _updates) =
        run_pattern_with_recording(&solo_song, 0, 0, false, &command_rx);

    assert!(solo_sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
    assert!(!solo_sent.contains(&MidiMessage::note_on(1, 48, 0x70)));
}

#[test]
fn fake_midi_playback_honors_output_routing_settings() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    let cc_track = song.tracks[1].id;
    {
        let pattern = song.current_pattern_mut().expect("pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("set drums note");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x70)
            .expect("set bass note");
        pattern
            .set_automation_point(
                AutomationTarget::MidiCc {
                    track: cc_track,
                    controller: 74,
                },
                0,
                0.5,
            )
            .expect("set CC point");
    }
    let (_command_tx, command_rx) = mpsc::channel();

    song.midi.cc_out = true;
    song.midi.output_channels = vec![1];
    let (_result, filtered_sent, _updates) =
        run_pattern_with_recording(&song, 0, 0, false, &command_rx);
    assert!(!filtered_sent.contains(&MidiMessage::note_on(10, 60, 0x7f)));
    assert!(filtered_sent.contains(&MidiMessage::note_on(1, 48, 0x70)));
    assert!(filtered_sent.contains(&MidiMessage::control_change(1, 74, 64)));

    let (_command_tx, command_rx) = mpsc::channel();
    song.midi.notes_out = false;
    let (_result, disabled_sent, _updates) =
        run_pattern_with_recording(&song, 0, 0, false, &command_rx);
    assert!(!disabled_sent.contains(&MidiMessage::note_on(1, 48, 0x70)));
    assert!(disabled_sent.contains(&MidiMessage::control_change(1, 74, 64)));

    let (_command_tx, command_rx) = mpsc::channel();
    song.midi.cc_out = false;
    let (_result, cc_disabled, _updates) =
        run_pattern_with_recording(&song, 0, 0, false, &command_rx);
    assert!(!cc_disabled.contains(&MidiMessage::control_change(1, 74, 64)));
}
