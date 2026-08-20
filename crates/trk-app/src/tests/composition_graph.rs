use super::*;

fn fixture_graph() -> CompositionGraph {
    CompositionGraph {
        schema: COMPOSITION_GRAPH_SCHEMA.to_string(),
        title: "Narrative arc".to_string(),
        sections: vec![
            CompositionGraphSection {
                id: "intro".to_string(),
                name: "Intro".to_string(),
                pattern: 1,
                repeats: 2,
                motifs: vec!["pulse".to_string()],
                evidence: vec!["Pattern 01 establishes pulse".to_string()],
                transition: Some("build".to_string()),
            },
            CompositionGraphSection {
                id: "answer".to_string(),
                name: "Answer".to_string(),
                pattern: 2,
                repeats: 1,
                motifs: Vec::new(),
                evidence: Vec::new(),
                transition: None,
            },
        ],
    }
}

#[test]
fn composition_graph_validates_and_compiles_sequence() {
    let mut song = Song::empty();
    let second = song.create_pattern(64);
    let graph = fixture_graph();

    validate_composition_graph(&graph).expect("valid graph");
    let compiled = compile_composition_graph(&song, &graph).expect("compile");
    let preview = format_composition_graph_preview(&graph);

    assert_eq!(
        compiled.sequence,
        vec![song.patterns[0].id, song.patterns[0].id, second]
    );
    assert!(preview.contains("# Composition Graph Preview"));
    assert!(preview.contains("Intro: pattern 1"));
}

#[test]
fn graph_cli_validates_and_compiles_project_files() {
    let base = std::env::temp_dir().join(format!("trk-graph-cli-{}", std::process::id()));
    let graph_path = base.with_extension("graph.json");
    let input_path = base.with_extension("input.trk");
    let output_path = base.with_extension("output.trk");
    let mut song = Song::empty();
    song.create_pattern(64);
    let mut variation_history = PatternVariationHistory::default();
    variation_history
        .record_at(
            123,
            "generated intro",
            PatternVariationSource::AiProposal,
            0,
            Some(0),
            song.patterns[0].clone(),
        )
        .expect("record variation");
    let graph = fixture_graph();
    std::fs::write(
        &graph_path,
        serde_json::to_string_pretty(&graph).expect("graph json"),
    )
    .expect("write graph");
    save_project_file(
        &input_path,
        &crate::persistence::ProjectFile::with_history(song.clone(), variation_history.clone()),
    )
    .expect("save project");

    assert_eq!(
        parse_graph_command([
            "validate".to_string(),
            graph_path.to_string_lossy().to_string()
        ]),
        CliCommand::GraphValidate(GraphValidateArgs {
            graph_path: Some(graph_path.clone()),
        })
    );
    run_graph_validate(&GraphValidateArgs {
        graph_path: Some(graph_path.clone()),
    })
    .expect("validate");
    run_graph_compile(&GraphCompileArgs {
        graph_path: Some(graph_path.clone()),
        input_path: Some(input_path.clone()),
        output_path: Some(output_path.clone()),
    })
    .expect("compile");

    let compiled = load_project_file(&output_path).expect("compiled");
    let _ = std::fs::remove_file(graph_path);
    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);

    assert_eq!(compiled.song.sequence.len(), 3);
    assert_eq!(compiled.variation_history, variation_history);
}

#[test]
fn graph_tui_draft_can_be_previewed_rejected_applied_and_undone() {
    let mut app = App::default();
    app.song.create_pattern(64);
    let before = app.song.clone();

    type_command(&mut app, "graph draft verse then answer");
    assert_eq!(app.song, before);
    assert!(app.pending_composition_graph.is_some());
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.text.contains("# Composition Graph Preview")));

    type_command(&mut app, "graph show");
    type_command(&mut app, "graph reject");
    assert!(app.pending_composition_graph.is_none());
    assert_eq!(app.song, before);

    type_command(&mut app, "graph draft verse then answer");
    type_command(&mut app, "graph apply");
    assert!(app.pending_composition_graph.is_none());
    assert_ne!(app.song.sequence, before.sequence);
    app.undo();
    assert_eq!(app.song, before);
}
