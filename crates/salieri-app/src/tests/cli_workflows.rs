use super::*;

#[test]
fn cli_parses_help_version_and_midi_listing() {
    assert_eq!(
        CliArgs::parse(["--help".to_string()]),
        CliArgs {
            command: CliCommand::Help,
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
    assert_eq!(
        CliArgs::parse(["--version".to_string()]).command,
        CliCommand::Version
    );
    assert_eq!(
        CliArgs::parse(["--list-midi-outputs".to_string()]).command,
        CliCommand::ListMidiOutputs
    );
    assert_eq!(
        CliArgs::parse(["--list-midi-inputs".to_string()]).command,
        CliCommand::ListMidiInputs
    );
}

#[test]
fn cli_parses_optional_project_path() {
    assert_eq!(
        CliArgs::parse(["song.salieri".to_string()]),
        CliArgs {
            command: CliCommand::Run,
            project_path: Some(PathBuf::from("song.salieri")),
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn cli_parses_config_and_log_level_options() {
    assert_eq!(
        CliArgs::parse([
            "--config".to_string(),
            "custom.toml".to_string(),
            "--log-level=debug".to_string(),
            "--midi-log".to_string(),
            "midi.log".to_string(),
            "song.salieri".to_string()
        ]),
        CliArgs {
            command: CliCommand::Run,
            project_path: Some(PathBuf::from("song.salieri")),
            config_path: Some(PathBuf::from("custom.toml")),
            log_level: Some("debug".to_string()),
            midi_log_path: Some(PathBuf::from("midi.log")),
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn cli_parses_midi_test_options() {
    assert_eq!(
        CliArgs::parse([
            "--midi-test-output=0".to_string(),
            "--midi-test-channel".to_string(),
            "2".to_string(),
            "--midi-test-note".to_string(),
            "64".to_string(),
            "--midi-test-duration-ms".to_string(),
            "1500".to_string(),
        ]),
        CliArgs {
            command: CliCommand::MidiTest,
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs {
                output: Some("0".to_string()),
                channel: 2,
                note: 64,
                duration_ms: 1500,
            },
        }
    );
}

#[test]
fn cli_parses_euclidean_transform_options() {
    assert_eq!(
        CliArgs::parse([
            "transform".to_string(),
            "euclidean".to_string(),
            "input.salieri".to_string(),
            "output.salieri".to_string(),
            "--pattern=2".to_string(),
            "--track".to_string(),
            "3".to_string(),
            "--steps".to_string(),
            "12".to_string(),
            "--pulses=5".to_string(),
            "--rotation=1".to_string(),
            "--pitch".to_string(),
            "40".to_string(),
            "--velocity=96".to_string(),
        ]),
        CliArgs {
            command: CliCommand::TransformEuclidean(TransformEuclideanArgs {
                input_path: Some(PathBuf::from("input.salieri")),
                output_path: Some(PathBuf::from("output.salieri")),
                pattern: 2,
                track: 3,
                steps: 12,
                pulses: 5,
                rotation: 1,
                pitch: 40,
                velocity: 96,
            }),
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn cli_parses_sample_inspect_options() {
    assert_eq!(
        CliArgs::parse([
            "sample".to_string(),
            "inspect".to_string(),
            "kick.wav".to_string(),
            "--format=json".to_string(),
            "--width".to_string(),
            "8".to_string(),
        ]),
        CliArgs {
            command: CliCommand::SampleInspect(SampleInspectArgs {
                path: Some(PathBuf::from("kick.wav")),
                format: SampleInspectFormat::Json,
                buckets: 8,
            }),
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn cli_parses_xrns_import_options() {
    assert_eq!(
        CliArgs::parse([
            "import".to_string(),
            "xrns".to_string(),
            "input.xrns".to_string(),
            "output.salieri".to_string(),
        ]),
        CliArgs {
            command: CliCommand::ImportXrns(ImportXrnsArgs {
                input_path: Some(PathBuf::from("input.xrns")),
                output_path: Some(PathBuf::from("output.salieri")),
                sample_dir: None,
                sample_path_prefix: None,
                convert_samples_to_wav: false,
            }),
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn cli_parses_xrns_sample_extraction_options() {
    assert_eq!(
        CliArgs::parse([
            "import".to_string(),
            "xrns".to_string(),
            "input.xrns".to_string(),
            "output.salieri".to_string(),
            "--sample-dir".to_string(),
            "fixtures/local/samples/demo".to_string(),
            "--sample-path-prefix=samples/demo".to_string(),
        ]),
        CliArgs {
            command: CliCommand::ImportXrns(ImportXrnsArgs {
                input_path: Some(PathBuf::from("input.xrns")),
                output_path: Some(PathBuf::from("output.salieri")),
                sample_dir: Some(PathBuf::from("fixtures/local/samples/demo")),
                sample_path_prefix: Some("samples/demo".to_string()),
                convert_samples_to_wav: false,
            }),
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn cli_parses_xrns_sample_conversion_option() {
    assert_eq!(
        parse_import_xrns_args([
            "input.xrns".to_string(),
            "output.salieri".to_string(),
            "--sample-dir=samples".to_string(),
            "--convert-samples-to-wav".to_string(),
        ]),
        ImportXrnsArgs {
            input_path: Some(PathBuf::from("input.xrns")),
            output_path: Some(PathBuf::from("output.salieri")),
            sample_dir: Some(PathBuf::from("samples")),
            sample_path_prefix: None,
            convert_samples_to_wav: true,
        }
    );
}

#[test]
fn xrns_sample_export_names_are_stable_and_unique() {
    let mut used = HashSet::new();

    assert_eq!(
        unique_sample_file_name("SampleData/Instrument02/Sample00.wav", &mut used),
        "instrument02-sample00.wav"
    );
    assert_eq!(
        unique_sample_file_name("SampleData/Instrument02/Sample00.wav", &mut used),
        "instrument02-sample00-2.wav"
    );
    assert_eq!(
        unique_sample_file_name("SampleData/Foley Hit!.wav", &mut used),
        "foley-hit.wav"
    );
}

#[test]
fn cli_parses_audio_export_options() {
    assert_eq!(
        CliArgs::parse([
            "export".to_string(),
            "audio".to_string(),
            "song.salieri".to_string(),
            "song.wav".to_string(),
            "--sequence".to_string(),
            "--sample-rate".to_string(),
            "44100".to_string(),
            "--channels=1".to_string(),
        ]),
        CliArgs {
            command: CliCommand::ExportAudio(AudioExportArgs {
                input_path: Some(PathBuf::from("song.salieri")),
                output_path: Some(PathBuf::from("song.wav")),
                pattern: 1,
                sequence: true,
                sample_rate: 44_100,
                channels: 1,
            }),
            project_path: None,
            config_path: None,
            log_level: None,
            midi_log_path: None,
            midi_test: MidiTestArgs::default(),
        }
    );
}

#[test]
fn sample_inspect_loads_tiny_wav_and_formats_outputs() {
    let path =
        std::env::temp_dir().join(format!("salieri-sample-inspect-{}.wav", std::process::id()));
    std::fs::write(
        &path,
        wav_pcm16_bytes(44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]),
    )
    .expect("write wav");

    let inspection = inspect_sample(&SampleInspectArgs {
        path: Some(path.clone()),
        format: SampleInspectFormat::Text,
        buckets: 2,
    })
    .expect("inspect sample");
    let _ = std::fs::remove_file(&path);

    assert_eq!(inspection.sample.sample_rate, 44_100);
    assert_eq!(inspection.sample.channels, 1);
    assert_eq!(inspection.sample.frames, 4);
    assert_eq!(inspection.overview.buckets.len(), 2);

    let text = format_sample_inspection_text(&inspection);
    assert!(text.contains("sample_rate: 44100"));
    assert!(text.contains("channels: 1"));
    assert!(text.contains("waveform_buckets: 2"));

    let json = format_sample_inspection_json(&inspection).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["sample"]["sample_rate"], 44_100);
    assert_eq!(value["waveform"]["bucket_count"], 2);
}

#[test]
fn audio_export_writes_sampler_events_to_wav() {
    let base = std::env::temp_dir().join(format!("salieri-audio-export-{}", std::process::id()));
    let project_path = base.with_extension("salieri");
    let sample_path = base.with_extension("wav");
    let output_path = base.with_extension("export.wav");
    std::fs::write(
        &sample_path,
        wav_pcm16_bytes(44_100, 1, &[i16::MAX, 16_384]),
    )
    .expect("write sample");

    let mut song = Song::empty();
    let sample = song.upsert_sample_reference(sample_path.to_string_lossy(), "hit.wav");
    let track = song.tracks[0].id;
    song.assign_sample_to_track(track, sample)
        .expect("assign sample");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 127)
        .expect("set note");
    save_project(&project_path, &song).expect("save project");

    run_export_audio(&AudioExportArgs {
        input_path: Some(project_path.clone()),
        output_path: Some(output_path.clone()),
        pattern: 1,
        sequence: false,
        sample_rate: 44_100,
        channels: 1,
    })
    .expect("export audio");

    let bytes = std::fs::read(&output_path).expect("read wav");
    let _ = std::fs::remove_file(&project_path);
    let _ = std::fs::remove_file(&sample_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(base.with_extension("export.wav.tmp"));

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(
        u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
        4
    );
    assert!(i16::from_le_bytes([bytes[44], bytes[45]]) > 32_760);
}

#[test]
fn sample_inspect_reports_invalid_wav_with_context() {
    let path = std::env::temp_dir().join(format!(
        "salieri-sample-inspect-invalid-{}.wav",
        std::process::id()
    ));
    std::fs::write(&path, b"not a wave").expect("write invalid wav");

    let error = inspect_sample(&SampleInspectArgs {
        path: Some(path.clone()),
        format: SampleInspectFormat::Text,
        buckets: 4,
    })
    .expect_err("invalid wav");
    let _ = std::fs::remove_file(&path);

    assert!(format!("{error:#}").contains("failed to load sample"));
}

#[test]
fn euclidean_transform_command_round_trips_project_files() {
    let base = std::env::temp_dir().join(format!("salieri-transform-cli-{}", std::process::id()));
    let input_path = base.with_extension("input.salieri");
    let output_path = base.with_extension("output.salieri");
    let song = Song::empty();
    save_project(&input_path, &song).expect("save input");

    run_transform_euclidean(&TransformEuclideanArgs {
        input_path: Some(input_path.clone()),
        output_path: Some(output_path.clone()),
        pattern: 1,
        track: 1,
        steps: 4,
        pulses: 2,
        rotation: 0,
        pitch: 36,
        velocity: 100,
    })
    .expect("transform");

    let transformed = load_project(&output_path).expect("load output");
    let pattern = transformed.current_pattern().expect("pattern");
    let active_rows = (0..8)
        .filter(|row| pattern.cell(*row, 0).expect("cell").note.is_some())
        .collect::<Vec<_>>();

    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    assert_eq!(active_rows, vec![1, 3, 5, 7]);
}
