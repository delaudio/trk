use super::*;

#[test]
fn project_and_critique_reports_format_deterministic_summaries() {
    let mut song = Song::empty();
    song.metadata.title = "Report Fixture".to_string();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("set note");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(4, 1, NoteEvent::Note { pitch: 48 }, 96)
        .expect("set bass");

    let project = format_project_report(&song);
    let critique = format_critique_report(&song);

    assert!(project.contains("# Project Report"));
    assert!(project.contains("- Title: Report Fixture"));
    assert!(project.contains("- Note cells: 2"));
    assert!(project.contains("- 01. Drums: 1 note cell(s)"));
    assert!(critique.contains("# Critique Report"));
    assert!(critique.contains("- Score:"));
    assert!(critique.contains("Follow-up commands"));
}

#[test]
fn cli_report_workflows_write_project_and_critique_artifacts() {
    let base = std::env::temp_dir().join(format!("trk-report-cli-{}", std::process::id()));
    let project_path = base.with_extension("trk");
    let project_report = base.with_extension("project.md");
    let critique_report = base.with_extension("critique.md");
    let mut song = Song::empty();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 64 }, 100)
        .expect("set note");
    save_project(&project_path, &song).expect("save project");

    assert_eq!(
        parse_report_command([
            "project".to_string(),
            project_path.to_string_lossy().to_string(),
            project_report.to_string_lossy().to_string(),
        ]),
        CliCommand::ReportProject(ReportArgs {
            input_path: Some(project_path.clone()),
            output_path: Some(project_report.clone()),
        })
    );
    run_report_project(&ReportArgs {
        input_path: Some(project_path.clone()),
        output_path: Some(project_report.clone()),
    })
    .expect("project report");
    run_report_critique(&ReportArgs {
        input_path: Some(project_path.clone()),
        output_path: Some(critique_report.clone()),
    })
    .expect("critique report");

    let project_output = std::fs::read_to_string(&project_report).expect("project output");
    let critique_output = std::fs::read_to_string(&critique_report).expect("critique output");
    let _ = std::fs::remove_file(&project_path);
    let _ = std::fs::remove_file(&project_report);
    let _ = std::fs::remove_file(&critique_report);

    assert!(project_output.contains("# Project Report"));
    assert!(project_output.contains("- Note cells: 1"));
    assert!(critique_output.contains("# Critique Report"));
    assert!(critique_output.contains(":revise"));
}

#[test]
fn tui_reports_save_workspace_artifacts_and_revision_preview_is_reviewable() {
    let workspace =
        std::env::temp_dir().join(format!("trk-report-workspace-{}", std::process::id()));
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "report project");
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("Project report"));
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.text.contains("# Project Report")));

    type_command(
        &mut app,
        format!("workspace init {}", workspace.display()).as_str(),
    );
    type_command(
        &mut app,
        format!("report critique workspace {}", workspace.display()).as_str(),
    );
    let critique_path = workspace.join("reports/critique-report.md");
    let critique = std::fs::read_to_string(&critique_path).expect("workspace critique");
    assert!(critique.contains("# Critique Report"));

    type_command(&mut app, "revise add a sparse counter melody");
    app.wait_for_tasks();
    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_some());

    type_command(&mut app, "ai accept");
    assert_ne!(app.song, before);
    app.undo();
    assert_eq!(app.song, before);

    let _ = std::fs::remove_dir_all(workspace);
}
