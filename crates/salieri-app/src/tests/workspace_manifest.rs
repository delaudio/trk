use super::*;

#[test]
fn workspace_init_writes_portable_manifest_and_default_roots() {
    let root = workspace_test_dir("init");
    let mut app = App::default();

    type_command(&mut app, &format!("workspace init {}", root.display()));

    let manifest_path = root.join(".salieri-workspace.json");
    let json = std::fs::read_to_string(&manifest_path).expect("manifest");
    let value = serde_json::from_str::<serde_json::Value>(&json).expect("valid json");
    assert_eq!(value["schema"], "salieri.workspace.v1");
    assert_eq!(value["roots"]["projects"], "projects");
    assert_eq!(value["roots"]["samples"], "samples");
    assert_eq!(value["roots"]["presets"], "presets");
    assert_eq!(value["roots"]["reports"], "reports");
    assert_eq!(value["roots"]["guidance"], "guidance");
    assert!(root.join("projects").is_dir());
    assert!(root.join("samples").is_dir());
    assert!(root.join("presets").is_dir());
    assert!(root.join("reports").is_dir());
    assert!(root.join("guidance").is_dir());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_index_counts_artifact_roots_from_one_model() {
    let root = workspace_test_dir("index");
    let mut app = App::default();
    type_command(&mut app, &format!("workspace init {}", root.display()));
    write_file(root.join("projects/song.salieri"), "{}");
    write_file(root.join("samples/kick.wav"), "wav");
    write_file(root.join("presets/profile.json"), "{}");
    write_file(root.join("reports/session.md"), "# report");
    write_file(root.join("guidance/dub.txt"), "dub");

    type_command(&mut app, &format!("workspace index {}", root.display()));

    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("1 project(s)")
            && message.text.contains("1 sample(s)")
            && message.text.contains("1 preset profile(s)")
            && message.text.contains("1 report(s)")
            && message.text.contains("1 guidance file(s)")
    }));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_trash_and_restore_are_non_destructive_moves() {
    let root = workspace_test_dir("trash");
    let mut app = App::default();
    type_command(&mut app, &format!("workspace init {}", root.display()));
    let project = root.join("projects/song.salieri");
    write_file(&project, "{}");

    type_command(
        &mut app,
        &format!("workspace trash {} projects/song.salieri", root.display()),
    );

    assert!(!project.exists());
    assert!(root.join(".salieri-trash/song.salieri").exists());
    let json = std::fs::read_to_string(root.join(".salieri-workspace.json")).expect("manifest");
    let value = serde_json::from_str::<serde_json::Value>(&json).expect("valid json");
    assert_eq!(
        value["trashRecords"][0]["original"],
        "projects/song.salieri"
    );
    assert_eq!(
        value["trashRecords"][0]["trashed"],
        ".salieri-trash/song.salieri"
    );

    type_command(
        &mut app,
        &format!("workspace restore {} projects/song.salieri", root.display()),
    );

    assert!(project.exists());
    assert!(!root.join(".salieri-trash/song.salieri").exists());
    let json = std::fs::read_to_string(root.join(".salieri-workspace.json")).expect("manifest");
    let value = serde_json::from_str::<serde_json::Value>(&json).expect("valid json");
    assert_eq!(value["trashRecords"].as_array().expect("records").len(), 0);

    let _ = std::fs::remove_dir_all(root);
}

fn workspace_test_dir(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("salieri-workspace-{label}-{}", std::process::id()))
}

fn write_file(path: impl AsRef<std::path::Path>, contents: &str) {
    let path = path.as_ref();
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    std::fs::write(path, contents).expect("write file");
}
