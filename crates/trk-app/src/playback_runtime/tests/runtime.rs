use super::*;

#[test]
fn runtime_emits_positions_and_stops() {
    let runtime = PlaybackRuntime::spawn(None);
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("set note");

    runtime.start_pattern_from(song, None, 0, 0, true);

    let deadline = Instant::now() + Duration::from_millis(250);
    let mut saw_position = false;
    while Instant::now() < deadline {
        if matches!(runtime.try_recv(), Some(PlaybackUpdate::Position(_))) {
            saw_position = true;
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    runtime.stop();

    let deadline = Instant::now() + Duration::from_millis(250);
    let mut saw_stop = false;
    while Instant::now() < deadline {
        while let Some(update) = runtime.try_recv() {
            if matches!(update, PlaybackUpdate::Stopped) {
                saw_stop = true;
                break;
            }
        }
        if saw_stop {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert!(saw_position);
    assert!(saw_stop);
}

#[test]
fn runtime_starts_pattern_from_requested_row() {
    let runtime = PlaybackRuntime::spawn(None);
    let mut song = Song::empty();
    speed_up_transport(&mut song);

    runtime.start_pattern_from(song, None, 0, 4, true);

    let deadline = Instant::now() + Duration::from_millis(250);
    let mut first_position = None;
    while Instant::now() < deadline {
        if let Some(PlaybackUpdate::Position(position)) = runtime.try_recv() {
            first_position = Some(position.position.row);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    runtime.stop();

    assert_eq!(first_position, Some(4));
}

#[test]
fn runtime_stops_when_pattern_loop_is_disabled() {
    let runtime = PlaybackRuntime::spawn(None);
    let mut song = Song::empty();
    speed_up_transport(&mut song);

    runtime.start_pattern_from(song, None, 0, 0, false);

    let deadline = Instant::now() + Duration::from_millis(250);
    let mut saw_stop = false;
    while Instant::now() < deadline {
        while let Some(update) = runtime.try_recv() {
            if matches!(update, PlaybackUpdate::Stopped) {
                saw_stop = true;
                break;
            }
        }
        if saw_stop {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert!(saw_stop);
}

#[test]
fn pattern_playback_routes_assigned_samples_to_audio_commands() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    let track = song.tracks[0].id;
    let sample = song.upsert_sample_reference("samples/kick.wav", "kick.wav");
    song.samples[0].gain = 0.5;
    song.assign_sample_to_track(track, sample)
        .expect("assign sample");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(1, 0, NoteEvent::Note { pitch: 72 }, 64)
        .expect("set note");

    let (_result, commands) = run_pattern_with_audio_recording(&song, 0, 1_000_000);
    let trigger = commands
        .iter()
        .find_map(|command| match command {
            RealtimeAudioCommand::TriggerSample {
                sample_id,
                frame,
                gain,
                pan,
                pitch_ratio,
                ..
            } => Some((*sample_id, *frame, *gain, *pan, *pitch_ratio)),
            RealtimeAudioCommand::StopVoice { .. }
            | RealtimeAudioCommand::StopTrack { .. }
            | RealtimeAudioCommand::AllNotesOff { .. } => None,
        })
        .expect("trigger sample command");

    assert_eq!(trigger.0, sample.0);
    assert_eq!(
        trigger.1,
        micros_to_frames(row_duration_micros(&song.transport), 1_000_000)
    );
    assert_approx_eq(trigger.2, 0.5 * (64.0 / 127.0));
    assert_approx_eq(trigger.3, 0.0);
    assert_approx_eq(trigger.4, 2.0);
}

#[test]
fn realtime_sample_loader_prepares_assigned_wavs() {
    let path = std::env::temp_dir().join(format!("trk-realtime-sample-{}.wav", std::process::id()));
    write_test_wav(&path, 44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]);
    let mut song = Song::empty();
    let track = song.tracks[0].id;
    let sample = song.upsert_sample_reference(path.to_string_lossy(), "kick.wav");
    song.assign_sample_to_track(track, sample)
        .expect("assign sample");
    let (update_tx, update_rx) = mpsc::channel();

    let samples = load_realtime_samples(
        &song,
        AudioConfig {
            sample_rate: 48_000,
            channels: 2,
            buffer_frames: 256,
        },
        &update_tx,
        None,
    );
    let _ = std::fs::remove_file(&path);

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].0, sample.0);
    assert_eq!(samples[0].1.sample_rate, 48_000);
    assert_eq!(samples[0].1.channels, 2);
    assert!(samples[0].1.frames >= 4);
    assert!((samples[0].2 - (48_000.0 / 44_100.0)).abs() < f64::EPSILON);
    assert!(samples.complete);
    assert!(update_rx.try_iter().collect::<Vec<_>>().is_empty());
}

#[test]
fn realtime_sample_loader_marks_partial_sets_incomplete() {
    let mut song = Song::empty();
    let track = song.tracks[0].id;
    let sample = song.upsert_sample_reference("missing-sample.wav", "missing.wav");
    song.assign_sample_to_track(track, sample)
        .expect("assign sample");
    let (update_tx, update_rx) = mpsc::channel();

    let samples = load_realtime_samples(&song, AudioConfig::default(), &update_tx, None);

    assert!(samples.is_empty());
    assert!(!samples.complete);
    assert!(update_rx
        .try_iter()
        .any(|update| matches!(update, PlaybackUpdate::AudioError(_))));
}

#[test]
fn realtime_sample_loader_resolves_paths_from_project_directory() {
    let dir = std::env::temp_dir().join(format!("trk-realtime-relative-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let sample_path = dir.join("hit.wav");
    write_test_wav(&sample_path, 44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]);
    let mut song = Song::empty();
    let track = song.tracks[0].id;
    let sample = song.upsert_sample_reference("hit.wav", "hit.wav");
    song.assign_sample_to_track(track, sample)
        .expect("assign sample");
    let (update_tx, update_rx) = mpsc::channel();

    let samples = load_realtime_samples(
        &song,
        AudioConfig {
            sample_rate: 48_000,
            channels: 2,
            buffer_frames: 256,
        },
        &update_tx,
        Some(&dir),
    );
    let _ = std::fs::remove_file(&sample_path);
    let _ = std::fs::remove_dir(&dir);

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].0, sample.0);
    assert!(update_rx.try_iter().collect::<Vec<_>>().is_empty());
}

#[test]
fn realtime_sample_loader_prepares_cell_instrument_samples() {
    let path = std::env::temp_dir().join(format!(
        "trk-realtime-cell-instrument-{}.wav",
        std::process::id()
    ));
    write_test_wav(&path, 44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]);
    let mut song = Song::empty();
    let sample = song.upsert_sample_reference(path.to_string_lossy(), "hit.wav");
    let instrument = song.upsert_sample_instrument(sample).expect("instrument");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
        .expect("set note");
    song.current_pattern_mut().expect("pattern").rows[0].cells[0].instrument = Some(instrument);
    let (update_tx, update_rx) = mpsc::channel();

    let samples = load_realtime_samples(
        &song,
        AudioConfig {
            sample_rate: 48_000,
            channels: 2,
            buffer_frames: 256,
        },
        &update_tx,
        None,
    );
    let _ = std::fs::remove_file(&path);

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].0, sample.0);
    assert!(update_rx.try_iter().collect::<Vec<_>>().is_empty());
}

#[test]
fn realtime_sample_loader_prepares_zoned_instrument_samples() {
    let low_path =
        std::env::temp_dir().join(format!("trk-realtime-zone-low-{}.wav", std::process::id()));
    let high_path =
        std::env::temp_dir().join(format!("trk-realtime-zone-high-{}.wav", std::process::id()));
    write_test_wav(&low_path, 44_100, 1, &[0, i16::MAX, i16::MIN]);
    write_test_wav(&high_path, 44_100, 1, &[0, i16::MIN, i16::MAX]);
    let mut song = Song::empty();
    let low = song.upsert_sample_reference(low_path.to_string_lossy(), "low.wav");
    let high = song.upsert_sample_reference(high_path.to_string_lossy(), "high.wav");
    let instrument = song.upsert_sample_instrument(low).expect("instrument");
    song.instruments
        .iter_mut()
        .find(|candidate| candidate.id == instrument)
        .expect("instrument")
        .zones = vec![InstrumentSampleZone {
        sample: high,
        key_start: 60,
        key_end: 127,
        velocity_start: 0,
        velocity_end: 127,
    }];
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 72 }, 0x7f)
        .expect("set note");
    song.current_pattern_mut().expect("pattern").rows[0].cells[0].instrument = Some(instrument);
    let (update_tx, update_rx) = mpsc::channel();

    let samples = load_realtime_samples(&song, AudioConfig::default(), &update_tx, None);
    let _ = std::fs::remove_file(&low_path);
    let _ = std::fs::remove_file(&high_path);

    let sample_ids = samples.iter().map(|(id, _, _)| *id).collect::<Vec<_>>();
    assert!(sample_ids.contains(&low.0));
    assert!(sample_ids.contains(&high.0));
    assert!(update_rx.try_iter().collect::<Vec<_>>().is_empty());
}

#[test]
fn interrupted_pattern_playback_sends_audio_all_notes_off() {
    let mut song = Song::empty();
    speed_up_transport(&mut song);
    let (command_tx, command_rx) = mpsc::channel();
    let (update_tx, _update_rx) = mpsc::channel();
    let (audio_tx, audio_rx) = mpsc::channel();
    let messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut output = PlaybackOutput::recording(messages);
    let mut midi_logger = MidiLogger::new(None, &update_tx);
    let mut audio_output =
        PlaybackAudioOutput::recording(audio_tx, AudioConfig::default().sample_rate);
    let audio_sample_rate = audio_output.sample_rate();
    let mut context = PlaybackRunContext {
        command_rx: &command_rx,
        update_tx: &update_tx,
        output: &mut output,
        midi_logger: &mut midi_logger,
        audio_output: &mut audio_output,
        audio_sample_rate,
        pending_reload: None,
    };
    command_tx.send(PlaybackCommand::Stop).expect("send stop");

    let result = run_pattern(&mut song, 0, 0, None, true, &mut context);
    let commands = audio_rx.try_iter().collect::<Vec<_>>();

    assert!(matches!(result, PatternRunResult::Command(_)));
    assert!(commands
        .iter()
        .any(|command| matches!(command, RealtimeAudioCommand::AllNotesOff { frame: 0 })));
}
