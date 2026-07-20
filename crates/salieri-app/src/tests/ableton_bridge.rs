use super::*;

#[test]
fn ableton_push_pull_and_clear_dry_runs_do_not_mutate_song() {
    let mut app = App::default();
    app.song.patterns[0]
        .set_note(0, 0, NoteEvent::Note { pitch: 64 }, 100)
        .expect("note");
    type_command(&mut app, "clip add");
    let before = app.song.clone();

    type_command(&mut app, "ableton push --dry-run scene 0 track 1");
    assert_eq!(app.song, before);
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("Ableton Live bridge dry-run: push")
            && message.text.contains("session clip")
    }));

    type_command(&mut app, "live pull --dry-run scene 0 track 1");
    assert_eq!(app.song, before);
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("Ableton Live bridge dry-run: pull")
    }));

    type_command(&mut app, "bridge clear --dry-run scene 0 track 1");
    assert_eq!(app.song, before);
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("Ableton Live bridge dry-run: clear")
    }));
}

#[test]
fn ableton_non_dry_run_reports_unavailable_without_mutating_song() {
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "ableton clear");

    assert_eq!(app.song, before);
    assert!(app
        .notification
        .as_ref()
        .expect("notification")
        .message
        .contains("unavailable unless configured"));
}

#[test]
fn ableton_clear_dry_run_can_target_live_scene_without_local_clips() {
    let mut app = App::default();
    let before = app.song.clone();

    type_command(&mut app, "ableton clear --dry-run scene 7 track 2");

    assert_eq!(app.song, before);
    assert!(app.ai_thread.messages.iter().any(|message| {
        message.role == AiMessageRole::Assistant
            && message.text.contains("Ableton Live bridge dry-run: clear")
            && message.text.contains("scene 07, track 02")
    }));
}
