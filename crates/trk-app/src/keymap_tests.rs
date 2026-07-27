use std::collections::BTreeMap;

use super::*;

#[test]
fn resolves_normalized_keys_within_the_active_mode() {
    let mut config = KeymapConfig::default();
    config
        .normal
        .insert("control+p".to_string(), "play pattern".to_string());
    config.edit.insert("S".to_string(), ":stop".to_string());
    let keymap = Keymap::from_config(&config).expect("valid keymap");

    assert_eq!(
        keymap.command_for(
            KeymapMode::Normal,
            &KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)
        ),
        Some(TrkCommand::Play(crate::command::PlayCommand::Pattern))
    );
    assert_eq!(
        keymap.command_for(
            KeymapMode::Edit,
            &KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT)
        ),
        Some(TrkCommand::Stop)
    );
    assert!(keymap
        .command_for(
            KeymapMode::Normal,
            &KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT)
        )
        .is_none());
    assert_eq!(
        keymap.help_summary(),
        Some("Custom keys: normal.control+p -> :play pattern | edit.S -> :stop".to_string())
    );
}

#[test]
fn accepts_current_and_future_mode_sections() {
    let mut config = KeymapConfig::default();
    config.ai.insert("a".to_string(), "help".to_string());
    config.clip.insert("c".to_string(), "help".to_string());

    let keymap = Keymap::from_config(&config).expect("future layers compile");

    assert!(keymap
        .command_for(
            KeymapMode::Ai,
            &KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)
        )
        .is_some());
    assert!(keymap
        .command_for(
            KeymapMode::Clip,
            &KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)
        )
        .is_some());
}

#[test]
fn reports_invalid_keys_commands_and_normalized_conflicts() {
    let config = KeymapConfig {
        normal: BTreeMap::from([
            ("ctrl+p".to_string(), "play pattern".to_string()),
            ("control+p".to_string(), "stop".to_string()),
            ("ctrl++".to_string(), "stop".to_string()),
            ("x".to_string(), "does-not-exist".to_string()),
        ]),
        ..KeymapConfig::default()
    };

    let diagnostics = Keymap::from_config(&config).expect_err("invalid keymap");

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("conflicts")));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("plus")));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Unknown command")));
}

#[test]
fn canonicalizes_shift_tab_and_function_keys() {
    assert_eq!(KeyChord::parse("shift+tab"), KeyChord::parse("backtab"));
    assert_eq!(
        KeyChord::parse("f12"),
        Ok(KeyChord {
            code: KeyCode::F(12),
            modifiers: KeyModifiers::NONE,
        })
    );
    assert!(KeyChord::parse("f25").is_err());
}

#[test]
fn rejects_semantically_invalid_commands() {
    let config = KeymapConfig {
        normal: BTreeMap::from([("b".to_string(), "bpm 0".to_string())]),
        ..KeymapConfig::default()
    };

    let diagnostics = Keymap::from_config(&config).expect_err("invalid BPM");

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0]
        .message
        .contains("BPM must be between 1 and 999"));
}
