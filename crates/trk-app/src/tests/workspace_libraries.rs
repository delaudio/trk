use super::*;

#[test]
fn sample_browser_uses_workspace_sample_library_by_default() {
    let dir = std::env::temp_dir().join(format!("trk-workspace-samples-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create sample dir");
    let sample_path = dir.join("library.wav");
    std::fs::write(&sample_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX])).expect("write wav");
    let mut app = App::new(AppConfig {
        workspace: config::WorkspaceConfig {
            sample_library: Some(dir.clone()),
            ..config::WorkspaceConfig::default()
        },
        ..AppConfig::default()
    });

    enter_command(&mut app, "sample browse");

    assert_eq!(app.mode, AppMode::SampleBrowser);
    let entries = app.tui_sample_browser_entries();
    let browser = app
        .tui_sample_browser_view(&entries)
        .expect("sample browser view");
    assert_eq!(browser.current_dir, dir.to_string_lossy());
    assert!(browser
        .entries
        .iter()
        .any(|entry| entry.name == "library.wav"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_browser_uses_workspace_project_library_by_default() {
    let dir = std::env::temp_dir().join(format!("trk-workspace-projects-{}", std::process::id()));
    let project_path = dir.join("library.trk");
    std::fs::create_dir_all(&dir).expect("create project dir");
    save_song_project(&project_path, &Song::empty()).expect("save project");
    let mut app = App::new(AppConfig {
        workspace: config::WorkspaceConfig {
            project_library: Some(dir.clone()),
            ..config::WorkspaceConfig::default()
        },
        ..AppConfig::default()
    });

    enter_command(&mut app, "projects");

    assert_eq!(app.mode, AppMode::ProjectBrowser);
    let entries = app.tui_project_browser_entries();
    let browser = app
        .tui_project_browser_view(&entries)
        .expect("project browser view");
    assert_eq!(browser.current_dir, dir.to_string_lossy());
    assert!(browser
        .entries
        .iter()
        .any(|entry| entry.name == "library.trk"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn project_browser_prefers_workspace_library_over_current_project_parent() {
    let library_dir =
        std::env::temp_dir().join(format!("trk-workspace-priority-{}", std::process::id()));
    let current_dir =
        std::env::temp_dir().join(format!("trk-current-project-{}", std::process::id()));
    std::fs::create_dir_all(&library_dir).expect("create library dir");
    std::fs::create_dir_all(&current_dir).expect("create current dir");
    let library_project = library_dir.join("library.trk");
    let current_project = current_dir.join("current.trk");
    save_song_project(&library_project, &Song::empty()).expect("save library project");
    save_song_project(&current_project, &Song::empty()).expect("save current project");
    let mut app = App {
        project_path: Some(current_project),
        ..App::new(AppConfig {
            workspace: config::WorkspaceConfig {
                project_library: Some(library_dir.clone()),
                ..config::WorkspaceConfig::default()
            },
            ..AppConfig::default()
        })
    };

    enter_command(&mut app, "projects");

    let entries = app.tui_project_browser_entries();
    let browser = app
        .tui_project_browser_view(&entries)
        .expect("project browser view");
    assert_eq!(browser.current_dir, library_dir.to_string_lossy());

    let _ = std::fs::remove_dir_all(&library_dir);
    let _ = std::fs::remove_dir_all(&current_dir);
}
