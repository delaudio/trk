use super::*;

#[test]
fn clip_commands_create_queue_activate_and_stop_without_destroying_patterns() {
    let mut app = App::default();
    app.song.patterns[0]
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("note");
    type_command(&mut app, "pattern new");
    app.pattern_index = 1;

    type_command(&mut app, "clip add");
    assert_eq!(app.song.clip_scenes.len(), 1);
    assert_eq!(app.clip_scene_cursor, 0);

    enter_command(&mut app, "clips");
    assert_eq!(app.mode, AppMode::Clips);
    assert_eq!(app.tui_active_view(), TuiView::Clips);

    enter_command(&mut app, "clip set 0 2 1");
    assert_eq!(app.song.clip_scenes[0].clips.len(), 1);
    assert_eq!(app.song.patterns.len(), 2);

    enter_command(&mut app, "clip launch scene 0");
    assert_eq!(app.queued_clip_scene, Some(0));
    assert_eq!(app.active_clip_scene, None);

    enter_command(&mut app, "clip commit");
    assert_eq!(app.queued_clip_scene, None);
    assert_eq!(app.active_clip_scene, Some(0));

    enter_command(&mut app, "clip stop");
    assert_eq!(app.active_clip_scene, None);
}
