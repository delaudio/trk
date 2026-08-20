use super::*;
use trk_core::pattern_events;

#[test]
fn command_mode_adds_lists_reports_and_clears_text_annotations() {
    let mut app = App::default();
    app.cursor.row = 6;

    enter_command(&mut app, "note project project sketch");
    enter_command(&mut app, "note pattern verse starts here");
    enter_command(&mut app, "note lyric pattern 8 words aligned to row");
    enter_command(&mut app, "note cue sequence 0 intro cue");

    assert_eq!(app.song.annotations.len(), 4);
    assert!(matches!(
        app.song.annotations[0].scope,
        TextAnnotationScope::Project
    ));
    assert!(matches!(
        app.song.annotations[1].scope,
        TextAnnotationScope::Pattern { row: Some(6), .. }
    ));
    assert!(matches!(
        app.song.annotations[2].kind,
        TextAnnotationKind::Lyric
    ));
    assert!(matches!(
        app.song.annotations[3].scope,
        TextAnnotationScope::Sequence { position: 0 }
    ));

    enter_command(&mut app, "note list");
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notification| notification.message.contains("words aligned")));
    enter_command(&mut app, "note report");
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notification| notification.message.contains("Text annotations report")));

    enter_command(&mut app, "note clear 2");
    assert_eq!(app.song.annotations.len(), 3);
    assert!(!app
        .song
        .annotations
        .iter()
        .any(|annotation| annotation.id == 2));
    assert!(app.dirty);
}

#[test]
fn text_annotations_persist_without_affecting_playback_state() {
    let path =
        std::env::temp_dir().join(format!("trk-text-annotations-{}.trk", std::process::id()));
    let mut song = Song::empty();
    let pattern_id = song.patterns[0].id;
    let before_events = pattern_events(&song, song.current_pattern().expect("pattern"));
    song.add_text_annotation(
        TextAnnotationKind::Lyric,
        TextAnnotationScope::Pattern {
            pattern: pattern_id,
            row: Some(4),
        },
        "hello persistence",
    );

    save_song_project(&path, &song).expect("save");
    let loaded = load_project(&path).expect("load");
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.annotations, song.annotations);
    assert_eq!(
        pattern_events(&loaded, loaded.current_pattern().expect("pattern")),
        before_events
    );
}
