use super::*;

#[test]
fn cli_parses_strudel_export_options() {
    assert_eq!(
        parse_export_command([
            "strudel".to_string(),
            "song.trk".to_string(),
            "song.js".to_string(),
            "--patterns=1,2".to_string(),
        ]),
        CliCommand::ExportStrudel(StrudelExportArgs {
            input_path: Some("song.trk".into()),
            output_path: Some("song.js".into()),
            pattern: 1,
            patterns: vec![1, 2],
            sequence: false,
        })
    );
    assert_eq!(
        parse_export_command([
            "strudel".to_string(),
            "song.trk".to_string(),
            "--sequence".to_string(),
        ]),
        CliCommand::ExportStrudel(StrudelExportArgs {
            input_path: Some("song.trk".into()),
            output_path: None,
            pattern: 1,
            patterns: Vec::new(),
            sequence: true,
        })
    );
}

#[test]
fn strudel_export_is_deterministic_and_reports_diagnostics() {
    let mut song = Song::empty();
    song.transport.bpm = 96;
    song.tracks[0].name = "Lead One".to_string();
    song.tracks[1].name = "Bass/Two".to_string();
    song.mixer.master_gain = 0.8;
    let sample = song.upsert_sample_reference("samples/lead.wav", "Lead");
    song.assign_sample_to_track(song.tracks[0].id, sample)
        .expect("assign sample");
    let pattern = song.current_pattern_mut().expect("pattern");
    pattern
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 127)
        .expect("c4");
    pattern
        .set_note(1, 1, NoteEvent::Note { pitch: 43 }, 80)
        .expect("g2");
    pattern.cell_mut(0, 0).expect("cell").volume = Some(64);
    pattern.cell_mut(0, 0).expect("cell").pan = Some(0);
    pattern.cell_mut(1, 1).expect("cell").command = Some(TrackerCommand::delay(8));

    let args = StrudelExportArgs {
        input_path: None,
        output_path: None,
        pattern: 1,
        patterns: Vec::new(),
        sequence: false,
    };
    let first = format_strudel_export(&song, &args).expect("first export");
    let second = format_strudel_export(&song, &args).expect("second export");

    assert_eq!(first, second);
    assert!(first.contains("// scope: pattern 1"));
    assert!(first.contains("// tempo: bpm=96 linesPerBeat=4"));
    assert!(first.contains("setcps(96/60/4)"));
    assert!(first.contains("note(\"c4"));
    assert!(first.contains("note(\"_ g2"));
    assert!(first.contains(".velocity(\"1.00"));
    assert!(first.contains(".gain(\"0.50"));
    assert!(first.contains(".pan(\"0.00"));
    assert!(first.contains(".s(\"track_01_lead_one\")"));
    assert!(first.contains("// Track 02: Bass/Two"));
    assert!(first.contains("Sampler, instrument, and sample assignment data"));
    assert!(first.contains("Mixer gain, routing, sends, and native effects"));
    assert!(first.contains("tracker commands"));
}

#[test]
fn strudel_export_writes_project_scope_to_file() {
    let base = std::env::temp_dir().join(format!("trk-strudel-{}", std::process::id()));
    let project_path = base.with_extension("trk");
    let output_path = base.with_extension("js");
    let mut song = Song::empty();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 64 }, 100)
        .expect("set note");
    save_song_project(&project_path, &song).expect("save project");

    run_export_strudel(&StrudelExportArgs {
        input_path: Some(project_path.clone()),
        output_path: Some(output_path.clone()),
        pattern: 1,
        patterns: Vec::new(),
        sequence: false,
    })
    .expect("write Strudel");

    let output = std::fs::read_to_string(&output_path).expect("read output");
    let _ = std::fs::remove_file(&project_path);
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(base.with_extension("js.tmp"));

    assert!(output.contains("note(\"e4"));
    assert!(output.contains("// diagnostics:"));
}
