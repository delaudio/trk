use super::*;

#[test]
fn preset_save_writes_local_inventory_metadata() {
    let path = preset_profile_path("save");
    let mut app = App::default();
    app.song.metadata.title = "Dub Kit".to_string();
    let sample = app.song.upsert_sample_reference("samples/kick.wav", "Kick");
    app.song
        .assign_sample_to_track(app.song.tracks[0].id, sample)
        .expect("assign sample");
    app.song.mixer.tracks[0]
        .effects
        .push(EffectDevice::gain(1, 0.75));
    app.midi_ports = vec![MidiOutputPort {
        index: 0,
        name: "IAC Driver Bus 1".to_string(),
    }];
    app.midi_input_ports = vec![MidiInputPort {
        index: 0,
        name: "Controller In".to_string(),
    }];

    type_command(&mut app, &format!("preset save {}", path.display()));

    let json = std::fs::read_to_string(&path).expect("profile saved");
    let value = serde_json::from_str::<serde_json::Value>(&json).expect("valid json");
    assert_eq!(value["schema"], "salieri.preset-profile.v1");
    assert_eq!(value["title"], "Dub Kit");
    assert_eq!(value["tracks"][0]["assignedInstrument"], "Kick");
    assert_eq!(value["tracks"][0]["assignedSample"], "samples/kick.wav");
    assert_eq!(value["instruments"][0]["primarySample"], "samples/kick.wav");
    assert_eq!(value["nativeDevices"][0]["kind"], "gain");
    assert_eq!(value["midi"]["outputPorts"][0], "IAC Driver Bus 1");
    assert_eq!(value["midi"]["inputPorts"][0], "Controller In");
    assert_eq!(value["abletonBridge"]["state"], "optional_not_configured");

    let _ = std::fs::remove_file(path);
}

#[test]
fn preset_list_show_and_load_profile_as_ai_guidance() {
    let dir = preset_profile_dir("list-load");
    let path = dir.join("bass.json");
    let mut app = App::default();
    app.song.metadata.title = "Bass Profile".to_string();
    let sample = app.song.upsert_sample_reference("samples/bass.wav", "Bass");
    app.song
        .assign_sample_to_track(app.song.tracks[1].id, sample)
        .expect("assign sample");

    type_command(&mut app, &format!("preset save {}", path.display()));
    type_command(&mut app, &format!("preset list {}", dir.display()));
    type_command(&mut app, &format!("preset show {}", path.display()));
    type_command(&mut app, &format!("preset load {}", path.display()));
    type_command(&mut app, "ai propose use loaded profile");
    app.wait_for_tasks();

    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("Preset profiles:")
            && message.text.contains("Bass Profile")
    }));
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("Preset profile Bass Profile")
    }));
    assert!(app
        .pending_ai_proposal
        .as_ref()
        .expect("proposal")
        .proposal
        .prompt
        .contains("Preset profile: Bass Profile"));
    assert!(app
        .pending_ai_proposal
        .as_ref()
        .expect("proposal")
        .proposal
        .prompt
        .contains("Instrument Bass: sample=samples/bass.wav"));

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn preset_ableton_command_reports_optional_bridge_boundary() {
    let mut app = App::default();

    type_command(&mut app, "preset ableton capture");

    let notification = app.notification.as_ref().expect("notification");
    assert_eq!(notification.kind, NotificationKind::Info);
    assert!(notification.message.contains("optional Ableton bridge"));
    assert!(notification.message.contains("local preset metadata only"));
}

fn preset_profile_path(label: &str) -> std::path::PathBuf {
    preset_profile_dir(label).join("profile.json")
}

fn preset_profile_dir(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "salieri-preset-profile-{label}-{}",
        std::process::id()
    ))
}
