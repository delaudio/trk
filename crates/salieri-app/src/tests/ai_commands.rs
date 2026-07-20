use super::*;

#[test]
fn command_mode_ai_proposal_preview_apply_and_undo() {
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "ai propose sparse bass sketch");
    app.wait_for_tasks();

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_some());
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("touches"));

    type_command(&mut app, "ai accept");

    assert_ne!(app.song, before);
    assert!(app.pending_ai_proposal.is_none());
    assert!(app.dirty);

    app.undo();

    assert_eq!(app.song, before);
}

#[test]
fn command_mode_ai_provider_status_is_visible_before_prompt_submission() {
    let mut app = App::new(AppConfig {
        ai: config::AiConfig {
            provider: config::AiProviderKind::Mock,
            model: "fixture-mock".to_string(),
            ..config::AiConfig::default()
        },
        ..AppConfig::default()
    });

    type_command(&mut app, "ai provider");

    let notification = app.notification.as_ref().expect("notification");
    assert_eq!(notification.kind, NotificationKind::Info);
    assert!(notification
        .message
        .contains("AI provider mock model=fixture-mock available"));
}

#[test]
fn command_mode_ai_proposal_uses_configured_mock_provider() {
    let mut app = App::new(AppConfig {
        ai: config::AiConfig {
            provider: config::AiProviderKind::Mock,
            model: "fixture-mock".to_string(),
            ..config::AiConfig::default()
        },
        ..AppConfig::default()
    });

    type_command(&mut app, "ai propose mocked idea");
    app.wait_for_tasks();

    let pending = app.pending_ai_proposal.as_ref().expect("proposal");
    assert_eq!(
        pending.proposal.source,
        salieri_ai::AiSource::Mock {
            model: "fixture-mock".to_string()
        }
    );
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("Mock AI provider fixture-mock preview"));
}

#[test]
fn command_mode_ai_proposal_reports_missing_command_provider_requirements() {
    let missing_env = format!("SALIERI_TEST_MISSING_AI_TOKEN_{}", std::process::id());
    let mut app = App::new(AppConfig {
        ai: config::AiConfig {
            provider: config::AiProviderKind::Command,
            model: "codex-cli".to_string(),
            command_path: Some("definitely-missing-salieri-ai-command".to_string()),
            required_env: vec![missing_env.clone()],
            ..config::AiConfig::default()
        },
        ..AppConfig::default()
    });

    type_command(&mut app, "ai propose should fail");

    let notification = app.notification.as_ref().expect("notification");
    assert_eq!(notification.kind, NotificationKind::Error);
    assert!(notification.message.contains("AI provider command"));
    assert!(notification
        .message
        .contains("command binary not found: definitely-missing-salieri-ai-command"));
    assert!(notification.message.contains(&missing_env));
    assert!(app.task_runtime.is_idle());
    assert!(app.pending_ai_proposal.is_none());
}

#[test]
fn command_mode_ai_proposal_can_be_rejected_without_mutating_song() {
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "ai propose lead idea");
    app.wait_for_tasks();
    type_command(&mut app, "ai reject");

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_none());
}

#[test]
fn ai_chat_command_opens_native_view() {
    let mut app = App::default();

    enter_command(&mut app, "ai chat");

    assert_eq!(app.mode, AppMode::Ai);
    assert_eq!(app.tui_active_view(), TuiView::AiChat);
    assert!(app
        .notification
        .as_ref()
        .expect("provider status")
        .message
        .contains("AI provider local_deterministic"));
}

#[test]
fn ai_chat_composer_submits_prompt_without_mutating_until_accept() {
    let mut app = App::default();
    let before = app.song.clone();

    enter_command(&mut app, "ai chat");
    for ch in "chat bass sketch".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.wait_for_tasks();

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_some());
    assert!(app.ai_thread.composer.is_empty());
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.role == AiMessageRole::User && message.text == "chat bass sketch"));
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.role == AiMessageRole::Assistant));
}

#[test]
fn ai_chat_renders_selected_proposal_preview_with_all_touched_cells() {
    let mut app = App::default();

    enter_command(&mut app, "ai chat");
    for ch in "preview all cells".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.wait_for_tasks();

    let preview_lines = app.tui_ai_proposal_preview_lines();
    let pending = app.pending_ai_proposal.as_ref().expect("proposal");
    for cell in &pending.touched_cells {
        let cell_label = format!(
            "p{:02}/r{:02}/t{:02}",
            cell.pattern + 1,
            cell.row,
            cell.track + 1
        );
        assert!(
            preview_lines.iter().any(|line| line.contains(&cell_label)),
            "missing touched cell {cell_label} from {preview_lines:?}"
        );
    }
    assert!(preview_lines
        .iter()
        .any(|line| line.contains("a apply | r reject | p preview")));
    assert!(preview_lines
        .iter()
        .any(|line| line.contains("no instrument, automation, or mixer changes")));
}

#[test]
fn ai_chat_apply_shortcut_routes_through_undo_stack() {
    let mut app = App::default();
    let before = app.song.clone();

    enter_command(&mut app, "ai chat");
    for ch in "apply from chat".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.wait_for_tasks();
    assert!(app.pending_ai_proposal.is_some());

    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

    assert!(app.pending_ai_proposal.is_none());
    assert_ne!(app.song, before);
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant && message.text.contains("AI proposal applied")
    }));

    app.undo();
    assert_eq!(app.song, before);
}

#[test]
fn ai_chat_reject_shortcut_does_not_mutate_song() {
    let mut app = App::default();
    let before = app.song.clone();

    enter_command(&mut app, "ai chat");
    for ch in "reject from chat".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.wait_for_tasks();
    assert!(app.pending_ai_proposal.is_some());

    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_none());
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Progress && message.text == "AI proposal rejected"
    }));
}

#[test]
fn ai_chat_session_survives_restart_without_restoring_pending_proposal() {
    let session_file = ai_session_test_path("roundtrip");
    let config = AppConfig {
        ai: config::AiConfig {
            session_file: Some(session_file.clone()),
            ..config::AiConfig::default()
        },
        ..AppConfig::default()
    };
    let mut app = App::new(config.clone());

    enter_command(&mut app, "ai chat");
    for ch in "persistent chat".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.wait_for_tasks();
    assert!(app.pending_ai_proposal.is_some());
    assert!(session_file.exists());

    let mut restored = App::new(config);
    restored.load_ai_session_command();

    assert!(restored.pending_ai_proposal.is_none());
    assert!(restored.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::User && message.text == "persistent chat"
    }));
    assert!(restored.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant && message.text.contains("touches")
    }));

    let _ = std::fs::remove_file(session_file);
}

#[test]
fn ai_chat_delete_session_does_not_modify_current_project() {
    let session_file = ai_session_test_path("delete");
    let config = AppConfig {
        ai: config::AiConfig {
            session_file: Some(session_file.clone()),
            ..config::AiConfig::default()
        },
        ..AppConfig::default()
    };
    let mut app = App::new(config);
    let before = app.song.clone();

    type_command(&mut app, "ai propose delete session only");
    app.wait_for_tasks();
    assert!(session_file.exists());

    type_command(&mut app, "ai delete");

    assert_eq!(app.song, before);
    assert!(app.pending_ai_proposal.is_none());
    assert!(!session_file.exists());
    assert_eq!(app.ai_thread.messages.len(), 1);
    assert_eq!(app.ai_thread.messages[0].role, AiMessageRole::System);
}

#[test]
fn ai_chat_retention_keeps_system_message_and_recent_history() {
    let session_file = ai_session_test_path("retention");
    let mut app = App::new(AppConfig {
        ai: config::AiConfig {
            session_file: Some(session_file.clone()),
            retention_messages: 3,
            ..config::AiConfig::default()
        },
        ..AppConfig::default()
    });

    app.push_ai_message(AiMessageRole::User, "first");
    app.push_ai_message(AiMessageRole::Assistant, "second");
    app.push_ai_message(AiMessageRole::User, "third");
    app.push_ai_message(AiMessageRole::Assistant, "fourth");

    assert_eq!(app.ai_thread.messages.len(), 3);
    assert_eq!(app.ai_thread.messages[0].role, AiMessageRole::System);
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.text == "third"));
    assert!(app
        .ai_thread
        .messages
        .iter()
        .any(|message| message.text == "fourth"));

    let _ = std::fs::remove_file(session_file);
}

fn ai_session_test_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "salieri-ai-command-session-{label}-{}.json",
        std::process::id()
    ))
}
