use super::*;

#[test]
fn sampler_view_opens_without_sample_and_loads_wav_from_command() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    assert_eq!(app.mode, AppMode::Sampler);
    assert_eq!(app.tui_active_view(), TuiView::Sampler);
    assert!(app.tui_sampler_view().is_none());

    let path =
        std::env::temp_dir().join(format!("salieri-sampler-view-{}.wav", std::process::id()));
    std::fs::write(
        &path,
        wav_pcm16_bytes(44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]),
    )
    .expect("write wav");

    enter_command(&mut app, &format!("sample view {}", path.display()));
    let _ = std::fs::remove_file(&path);

    assert_eq!(app.mode, AppMode::Sampler);
    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.name, path.file_name().unwrap().to_string_lossy());
    assert_eq!(sampler.overview.sample_rate, 44_100);
    assert_eq!(sampler.overview.channels, 1);
    assert_eq!(sampler.overview.frames, 4);
    assert_eq!(sampler.waveform_zoom, 1);
    assert_eq!(sampler.waveform_start_bucket, 0);
    assert_eq!(sampler.waveform_end_bucket, sampler.overview.buckets.len());

    app.handle_key(KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE));
    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.waveform_zoom, 2);
    assert_eq!(sampler.waveform_start_bucket, 0);
    assert_eq!(
        sampler.waveform_end_bucket,
        sample_waveform_visible_buckets(sampler.overview.buckets.len(), 2)
    );

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    let sampler = app.tui_sampler_view().expect("sampler view");
    assert!(sampler.waveform_start_bucket > 0);

    app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.waveform_end_bucket, sampler.overview.buckets.len());

    app.handle_key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE));
    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.waveform_zoom, 1);
    assert_eq!(sampler.waveform_start_bucket, 0);
    assert_eq!(sampler.waveform_end_bucket, sampler.overview.buckets.len());
}

#[test]
fn sampler_commands_assign_list_and_unassign_loaded_sample() {
    let mut app = App::default();
    let path =
        std::env::temp_dir().join(format!("salieri-sampler-assign-{}.wav", std::process::id()));
    std::fs::write(&path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX])).expect("write wav");

    enter_command(&mut app, &format!("sample view {}", path.display()));
    enter_command(&mut app, "sample assign 2");

    let track_id = app.song.tracks[1].id;
    let assignment = app
        .song
        .sample_assignment_for_track(track_id)
        .expect("assignment");
    let sample = app.song.sample_for_id(assignment.sample).expect("sample");
    assert_eq!(sample.name, path.file_name().unwrap().to_string_lossy());
    assert_eq!(sample.path, path.to_string_lossy());
    let instrument = app.song.instrument_for_track(track_id).expect("instrument");
    assert_eq!(instrument.sample, Some(assignment.sample));
    assert!(app.dirty);

    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.instrument, Some(instrument.name.as_str()));
    assert_eq!(
        sampler.assigned_track,
        Some(app.song.tracks[1].name.as_str())
    );
    assert_eq!(sampler.assigned_track_count, 1);

    enter_command(&mut app, "sample assignments");
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("Bass"));

    enter_command(&mut app, "sample unassign 2");
    let _ = std::fs::remove_file(&path);

    assert!(app.song.sample_assignment_for_track(track_id).is_none());
    assert!(app.song.instrument_assignment_for_track(track_id).is_none());
}

#[test]
fn sampler_commands_edit_playback_settings() {
    let mut app = App::default();
    let path = std::env::temp_dir().join(format!(
        "salieri-sampler-settings-{}.wav",
        std::process::id()
    ));
    std::fs::write(
        &path,
        wav_pcm16_bytes(44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]),
    )
    .expect("write wav");

    enter_command(&mut app, &format!("sample view {}", path.display()));
    enter_command(&mut app, "sample start 1");
    enter_command(&mut app, "sample end 4");
    enter_command(&mut app, "sample loop 1 3");
    enter_command(&mut app, "sample envelope 0.010 0.020 0.500 0.030");

    let sample = app
        .song
        .samples
        .iter()
        .find(|sample| sample.path == path.to_string_lossy())
        .expect("sample reference");
    assert_eq!(sample.playback.start_frame, Some(1));
    assert_eq!(sample.playback.end_frame, Some(4));
    assert_eq!(sample.playback.mode, SamplePlaybackMode::Loop);
    assert_eq!(sample.playback.loop_start_frame, Some(1));
    assert_eq!(sample.playback.loop_end_frame, Some(3));
    assert_eq!(sample.playback.envelope.sustain, 0.5);

    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.playback_mode, "loop");
    assert_eq!(sampler.start_frame, Some(1));
    assert_eq!(sampler.end_frame, Some(4));
    assert_eq!(sampler.loop_start_frame, Some(1));
    assert_eq!(sampler.loop_end_frame, Some(3));
    assert_eq!(sampler.envelope, (0.010, 0.020, 0.500, 0.030));

    enter_command(&mut app, "sample start 9");
    let sample = app.song.samples.first().expect("sample reference");
    assert_eq!(sample.playback.start_frame, Some(1));
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("before end"));

    enter_command(&mut app, "sample loop off");
    let sample = app.song.samples.first().expect("sample reference");
    assert_eq!(sample.playback.mode, SamplePlaybackMode::OneShot);
    assert_eq!(sample.playback.loop_start_frame, None);
    assert_eq!(sample.playback.loop_end_frame, None);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn sampler_view_keyboard_controls_edit_envelope() {
    let mut app = App::default();
    let path = std::env::temp_dir().join(format!(
        "salieri-sampler-envelope-{}.wav",
        std::process::id()
    ));
    std::fs::write(
        &path,
        wav_pcm16_bytes(44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]),
    )
    .expect("write wav");

    enter_command(&mut app, &format!("sample view {}", path.display()));

    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('}'), KeyModifiers::SHIFT));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));

    let sample = app
        .song
        .samples
        .iter()
        .find(|sample| sample.path == path.to_string_lossy())
        .expect("sample reference");
    assert_eq!(sample.playback.envelope.attack_seconds, 0.005);
    assert_eq!(sample.playback.envelope.decay_seconds, 0.050);
    assert_eq!(sample.playback.envelope.sustain, 0.950);
    assert_eq!(sample.playback.envelope.release_seconds, 0.005);
    assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Sustain);
    assert!(app.dirty);

    let sampler = app.tui_sampler_view().expect("sampler view");
    assert_eq!(sampler.selected_envelope, SamplerEnvelopeField::Sustain);
    assert_eq!(sampler.envelope, (0.005, 0.050, 0.950, 0.005));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn sampler_commands_replace_unload_and_cleanup_references() {
    let mut app = App::default();
    let first_path =
        std::env::temp_dir().join(format!("salieri-sampler-first-{}.wav", std::process::id()));
    let second_path =
        std::env::temp_dir().join(format!("salieri-sampler-second-{}.wav", std::process::id()));
    std::fs::write(&first_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX]))
        .expect("write first wav");
    std::fs::write(&second_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MIN]))
        .expect("write second wav");

    enter_command(&mut app, &format!("sample view {}", first_path.display()));
    enter_command(&mut app, "sample assign 2");
    let track_id = app.song.tracks[1].id;
    let first_sample = app
        .song
        .sample_assignment_for_track(track_id)
        .expect("first assignment")
        .sample;

    enter_command(&mut app, &format!("sample view {}", second_path.display()));
    enter_command(&mut app, "sample replace 2");
    let second_sample = app
        .song
        .sample_assignment_for_track(track_id)
        .expect("second assignment")
        .sample;

    assert_ne!(first_sample, second_sample);
    assert!(app.song.sample_for_id(first_sample).is_none());
    assert_eq!(
        app.song
            .sample_for_id(second_sample)
            .expect("second sample")
            .path,
        second_path.to_string_lossy()
    );

    enter_command(&mut app, "sample unload");
    assert!(app.song.sample_for_id(second_sample).is_some());
    assert!(app.tui_sampler_view().is_some());
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("Unassign or replace"));

    enter_command(&mut app, "sample unassign 2");
    enter_command(&mut app, "sample cleanup");

    assert!(app.song.sample_for_id(second_sample).is_none());
    let _ = std::fs::remove_file(&first_path);
    let _ = std::fs::remove_file(&second_path);
}

#[test]
fn sample_browser_command_queues_external_request_when_configured() {
    let mut app = App::new(AppConfig {
        sample_browser: SampleBrowserConfig {
            chooser_command: Some("true".to_string()),
            start_dir: Some(PathBuf::from("Samples")),
        },
        ..AppConfig::default()
    });

    enter_command(&mut app, "sample choose Drums");

    assert_eq!(app.mode, AppMode::Sampler);
    let (config, request) = app.take_sample_browser_request().expect("browser request");
    assert_eq!(config.chooser_command, Some("true".to_string()));
    assert_eq!(request.start_dir, Some(PathBuf::from("Drums")));
}

#[test]
fn sample_browser_command_warns_without_configuration() {
    let mut app = App::default();

    enter_command(&mut app, "sample choose");

    assert_eq!(app.mode, AppMode::Sampler);
    assert!(app.take_sample_browser_request().is_none());
    assert_eq!(
        app.notification
            .as_ref()
            .map(|value| value.message.as_str()),
        Some("Sample browser not configured")
    );
}

#[test]
fn in_app_sample_browser_previews_and_loads_wav_files() {
    let mut app = App::default();
    let dir = std::env::temp_dir().join(format!(
        "salieri-in-app-sample-browser-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create sample dir");
    let sample_path = dir.join("kick.wav");
    std::fs::write(&sample_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX])).expect("write wav");

    enter_command(&mut app, &format!("sample browse {}", dir.display()));

    assert_eq!(app.mode, AppMode::SampleBrowser);
    assert_eq!(app.tui_active_view(), TuiView::SampleBrowser);
    let entries = app.tui_sample_browser_entries();
    let browser = app
        .tui_sample_browser_view(&entries)
        .expect("sample browser view");
    assert_eq!(browser.entries.len(), 1);
    assert_eq!(
        browser.entries[0].kind,
        SampleBrowserEntryKind::SupportedSample
    );
    assert!(browser.preview.is_some());

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Sampler);
    assert_eq!(
        app.tui_sampler_view().expect("loaded sample").source_path,
        sample_path.to_str().expect("utf8 path")
    );

    let _ = std::fs::remove_file(&sample_path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn in_app_sample_browser_reports_unsupported_files() {
    let mut app = App::default();
    let dir = std::env::temp_dir().join(format!(
        "salieri-unsupported-sample-browser-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create sample dir");
    let text_path = dir.join("notes.txt");
    std::fs::write(&text_path, "not a wav").expect("write text");

    enter_command(&mut app, &format!("sample browse {}", dir.display()));

    assert_eq!(app.mode, AppMode::SampleBrowser);
    let entries = app.tui_sample_browser_entries();
    let browser = app
        .tui_sample_browser_view(&entries)
        .expect("sample browser view");
    assert_eq!(
        browser.entries[0].kind,
        SampleBrowserEntryKind::UnsupportedFile
    );
    assert_eq!(browser.message, Some("Unsupported file type"));

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        app.notification
            .as_ref()
            .map(|value| value.message.as_str()),
        Some("Unsupported sample file")
    );

    let _ = std::fs::remove_file(&text_path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn project_browser_discovers_projects_and_persists_recent_projects() {
    let dir = std::env::temp_dir().join(format!(
        "salieri-project-browser-discovery-{}",
        std::process::id()
    ));
    let recent_file = dir.join("recent-projects.json");
    let project_path = dir.join("valid.salieri");
    let missing_path = dir.join("missing.salieri");
    let invalid_path = dir.join("invalid.salieri");
    std::fs::create_dir_all(&dir).expect("create project dir");
    let mut song = Song::empty();
    song.metadata.title = "Discovery Test".to_string();
    save_project(&project_path, &song).expect("save project");
    std::fs::write(&invalid_path, "not json").expect("write invalid project");

    save_recent_projects(
        Some(&recent_file),
        &[project_path.clone(), missing_path.clone()],
    )
    .expect("save recents");
    let recent_projects = load_recent_projects(Some(&recent_file));

    let entries = read_project_browser_entries(&dir, &recent_projects).expect("discover projects");

    assert_eq!(
        recent_projects,
        vec![project_path.clone(), missing_path.clone()]
    );
    assert_eq!(entries[0].kind, ProjectBrowserEntryKind::RecentProject);
    assert_eq!(entries[0].name, "valid.salieri");
    assert!(entries[0].detail.contains("Discovery Test"));
    assert_eq!(entries[1].kind, ProjectBrowserEntryKind::MissingProject);
    assert_eq!(entries[1].name, "missing.salieri");
    assert!(entries.iter().any(|entry| entry.name == "invalid.salieri"
        && entry.kind == ProjectBrowserEntryKind::InvalidProject));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_browser_opens_valid_project_and_records_recent() {
    let dir = std::env::temp_dir().join(format!(
        "salieri-project-browser-open-{}",
        std::process::id()
    ));
    let recent_file = dir.join("recent-projects.json");
    let project_path = dir.join("opened.salieri");
    std::fs::create_dir_all(&dir).expect("create project dir");
    let mut song = Song::empty();
    song.metadata.title = "Opened Project".to_string();
    song.transport.bpm = 137;
    save_project(&project_path, &song).expect("save project");
    let mut app = App::new(AppConfig {
        project_browser: ProjectBrowserConfig {
            start_dir: Some(dir.clone()),
            recent_file: Some(recent_file.clone()),
        },
        ..AppConfig::default()
    });

    enter_command(&mut app, "open");
    assert_eq!(app.mode, AppMode::ProjectBrowser);
    assert_eq!(app.tui_active_view(), TuiView::ProjectBrowser);

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.song.metadata.title, "Opened Project");
    assert_eq!(app.song.transport.bpm, 137);
    assert_eq!(app.project_path, Some(project_path.clone()));
    assert!(!app.dirty);
    assert_eq!(app.history.undo_len(), 0);
    assert_eq!(
        load_recent_projects(Some(&recent_file)).first(),
        Some(&project_path.canonicalize().expect("canonical project"))
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_browser_requires_confirmation_before_discarding_dirty_song() {
    let dir = std::env::temp_dir().join(format!(
        "salieri-project-browser-dirty-{}",
        std::process::id()
    ));
    let recent_file = dir.join("recent-projects.json");
    let project_path = dir.join("target.salieri");
    std::fs::create_dir_all(&dir).expect("create project dir");
    let mut target = Song::empty();
    target.transport.bpm = 155;
    save_project(&project_path, &target).expect("save project");
    let mut app = App::new(AppConfig {
        project_browser: ProjectBrowserConfig {
            start_dir: Some(dir.clone()),
            recent_file: Some(recent_file),
        },
        ..AppConfig::default()
    });
    app.set_bpm(130);

    enter_command(&mut app, "projects");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Dialog);
    assert_eq!(app.song.transport.bpm, 130);
    assert!(app.delete_confirmation_message().is_some());

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::ProjectBrowser);
    assert_eq!(app.song.transport.bpm, 130);

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.song.transport.bpm, 155);
    assert!(!app.dirty);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_browser_reports_invalid_projects_without_mutating_active_song() {
    let dir = std::env::temp_dir().join(format!(
        "salieri-project-browser-invalid-{}",
        std::process::id()
    ));
    let recent_file = dir.join("recent-projects.json");
    let invalid_path = dir.join("invalid.salieri");
    std::fs::create_dir_all(&dir).expect("create project dir");
    std::fs::write(&invalid_path, "not json").expect("write invalid project");
    let mut app = App::new(AppConfig {
        project_browser: ProjectBrowserConfig {
            start_dir: Some(dir.clone()),
            recent_file: Some(recent_file),
        },
        ..AppConfig::default()
    });
    app.set_bpm(131);
    app.clean_song = app.song.clone();
    app.refresh_dirty();

    enter_command(&mut app, "open");
    let entries = app.tui_project_browser_entries();
    assert_eq!(entries[0].kind, ProjectBrowserEntryKind::InvalidProject);

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::ProjectBrowser);
    assert_eq!(app.song.transport.bpm, 131);
    assert!(!app.dirty);
    assert_eq!(
        app.notification
            .as_ref()
            .map(|value| value.message.as_str()),
        Some("Project cannot be opened")
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn external_sample_browser_reads_selected_path_from_chooser_file() {
    let selected = run_external_sample_browser(
        &SampleBrowserConfig {
            chooser_command: Some(
                "printf '%s\n' \"$SALIERI_SAMPLE_START_DIR/pick.wav\" > \"$SALIERI_CHOOSER_FILE\""
                    .to_string(),
            ),
            start_dir: Some(PathBuf::from("Samples")),
        },
        &SampleBrowserRequest { start_dir: None },
    )
    .expect("run browser");

    assert_eq!(selected, Some(PathBuf::from("Samples/pick.wav")));
}
