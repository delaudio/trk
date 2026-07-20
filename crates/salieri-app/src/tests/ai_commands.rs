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
