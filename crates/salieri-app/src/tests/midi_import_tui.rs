use super::*;

#[test]
fn command_mode_imports_midi_into_dirty_tui_project() {
    let base = std::env::temp_dir().join(format!("salieri-tui-midi-import-{}", std::process::id()));
    let midi_path = base.with_extension("mid");
    std::fs::write(
        &midi_path,
        hex_bytes(include_str!("../../../../fixtures/midi/simple-format0.hex")),
    )
    .expect("write midi");
    let mut app = App::default();

    type_command(&mut app, &format!("midi import {}", midi_path.display()));

    assert!(app.dirty);
    assert_eq!(app.project_path, None);
    assert_eq!(app.pattern_index, 0);
    assert_eq!(app.song.patterns.len(), 1);
    assert_eq!(
        app.song.patterns[0].row_count(),
        salieri_core::model::DEFAULT_PATTERN_LEN
    );
    assert_eq!(
        app.song
            .pattern(0)
            .expect("pattern")
            .cell(0, 1)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert!(app.notification.as_ref().is_some_and(|notification| {
        notification.message.contains("MIDI imported")
            && notification.message.contains("4 tracks")
            && notification.message.contains("1 patterns")
    }));

    let _ = std::fs::remove_file(&midi_path);
}

#[test]
fn command_mode_imports_midi_with_shell_escaped_path() {
    let base = std::env::temp_dir()
        .join(format!("salieri tui midi import {}", std::process::id()))
        .join("Top MIDI Tracks Pack (Free)");
    std::fs::create_dir_all(&base).expect("create midi dir");
    let midi_path = base.join("A Thousand Miles.mid");
    std::fs::write(
        &midi_path,
        hex_bytes(include_str!("../../../../fixtures/midi/simple-format0.hex")),
    )
    .expect("write midi");
    let escaped_path = shell_escape_path_for_command(&midi_path);
    let mut app = App::default();

    type_command(&mut app, &format!("midi import {escaped_path}"));

    assert!(app.dirty);
    assert_eq!(
        app.song
            .pattern(0)
            .expect("pattern")
            .cell(0, 1)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert!(app.notification.as_ref().is_some_and(|notification| {
        notification.message.contains("MIDI imported")
            && notification.message.contains("A Thousand Miles.mid")
    }));

    let _ = std::fs::remove_file(&midi_path);
    let _ = std::fs::remove_dir_all(base.parent().expect("base parent"));
}

fn hex_bytes(contents: &str) -> Vec<u8> {
    contents
        .split_whitespace()
        .filter_map(|byte| u8::from_str_radix(byte, 16).ok())
        .collect()
}

fn shell_escape_path_for_command(path: &Path) -> String {
    path.display()
        .to_string()
        .chars()
        .flat_map(|character| match character {
            ' ' | '(' | ')' => vec!['\\', character],
            _ => vec![character],
        })
        .collect()
}
