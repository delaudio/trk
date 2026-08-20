use super::*;

#[test]
fn style_analysis_infers_roles_energy_and_json() {
    let mut song = Song::empty();
    song.metadata.title = "Style Fixture".to_string();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 1, NoteEvent::Note { pitch: 36 }, 100)
        .expect("bass");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(2, 2, NoteEvent::Note { pitch: 76 }, 110)
        .expect("lead");

    let analysis = analyze_style(&song);
    let text = format_style_analysis_text(&analysis);
    let json = format_analysis_output(&analysis, AnalysisOutputFormat::Json).expect("json");

    assert!(text.contains("# Style Analysis"));
    assert!(text.contains("Bass: bass"));
    assert!(text.contains("Lead: lead"));
    assert!(text.contains("- Note cells: 2"));
    assert!(json.contains("\"title\": \"Style Fixture\""));
    assert!(json.contains("\"energy\""));
}

#[test]
fn cli_analyze_and_compare_write_text_and_json_outputs() {
    let base = std::env::temp_dir().join(format!("trk-style-cli-{}", std::process::id()));
    let left_path = base.with_extension("left.trk");
    let right_path = base.with_extension("right.trk");
    let analysis_path = base.with_extension("analysis.json");
    let compare_path = base.with_extension("compare.txt");
    let mut left = Song::empty();
    left.metadata.title = "Left".to_string();
    left.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("left note");
    let mut right = left.clone();
    right.metadata.title = "Right".to_string();
    right.transport.bpm = 132;
    right
        .current_pattern_mut()
        .expect("pattern")
        .set_note(4, 1, NoteEvent::Note { pitch: 43 }, 100)
        .expect("right note");
    save_song_project(&left_path, &left).expect("left");
    save_song_project(&right_path, &right).expect("right");

    assert_eq!(
        parse_compare_args([
            left_path.to_string_lossy().to_string(),
            right_path.to_string_lossy().to_string(),
            compare_path.to_string_lossy().to_string(),
            "--format=json".to_string(),
        ])
        .format,
        AnalysisOutputFormat::Json
    );
    run_analyze(&AnalysisArgs {
        input_path: Some(left_path.clone()),
        output_path: Some(analysis_path.clone()),
        format: AnalysisOutputFormat::Json,
    })
    .expect("analyze");
    run_compare(&CompareArgs {
        left_path: Some(left_path.clone()),
        right_path: Some(right_path.clone()),
        output_path: Some(compare_path.clone()),
        format: AnalysisOutputFormat::Text,
    })
    .expect("compare");

    let analysis = std::fs::read_to_string(&analysis_path).expect("analysis");
    let comparison = std::fs::read_to_string(&compare_path).expect("comparison");
    let _ = std::fs::remove_file(&left_path);
    let _ = std::fs::remove_file(&right_path);
    let _ = std::fs::remove_file(&analysis_path);
    let _ = std::fs::remove_file(&compare_path);

    assert!(analysis.contains("\"noteCells\": 1"));
    assert!(comparison.contains("# Style Comparison"));
    assert!(comparison.contains("Tempo delta: 12 BPM"));
}

#[test]
fn tui_analyze_and_compare_append_reports_without_mutating() {
    let base = std::env::temp_dir().join(format!("trk-style-tui-{}", std::process::id()));
    let other_path = base.with_extension("trk");
    let mut app = App::default();
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("note");
    let before = app.song.clone();
    let mut other = app.song.clone();
    other.metadata.title = "Other".to_string();
    other.transport.bpm = 130;
    save_song_project(&other_path, &other).expect("other");

    type_command(&mut app, "analyze");
    assert_eq!(app.song, before);
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("Style analysis"));
    type_command(
        &mut app,
        format!("compare {}", other_path.display()).as_str(),
    );
    assert_eq!(app.song, before);
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.text.contains("# Style Comparison")));

    let _ = std::fs::remove_file(other_path);
}
