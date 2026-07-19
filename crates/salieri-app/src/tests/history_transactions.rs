use super::*;

#[test]
fn repeated_tracker_typing_is_one_transaction() {
    let mut app = App {
        mode: AppMode::Edit,
        ..App::default()
    };

    app.insert_note(60);
    app.insert_note(62);

    assert_eq!(app.history.undo_len(), 1);
    assert!(app.dirty);
    app.undo();
    let pattern = app.song.current_pattern().expect("pattern");
    assert_eq!(pattern.cell(0, 0), Some(&PatternCell::default()));
    assert_eq!(pattern.cell(1, 0), Some(&PatternCell::default()));
    assert!(!app.dirty);
}

#[test]
fn paste_is_atomic_across_all_cells_and_supports_redo() {
    let mut app = App::default();
    let first = PatternCell {
        note: Some(NoteEvent::Note { pitch: 60 }),
        ..PatternCell::default()
    };
    let second = PatternCell {
        note: Some(NoteEvent::Note { pitch: 64 }),
        ..PatternCell::default()
    };
    app.clipboard = Some(Clipboard::Region(ClipboardRegion {
        cells: vec![vec![first, second]],
    }));

    app.paste_clipboard();
    assert_eq!(app.history.undo_len(), 1);
    app.undo();
    let pattern = app.song.current_pattern().expect("pattern");
    assert_eq!(pattern.cell(0, 0), Some(&PatternCell::default()));
    assert_eq!(pattern.cell(0, 1), Some(&PatternCell::default()));

    app.redo();
    let pattern = app.song.current_pattern().expect("pattern");
    assert_eq!(
        pattern.cell(0, 0).and_then(|cell| cell.note),
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert_eq!(
        pattern.cell(0, 1).and_then(|cell| cell.note),
        Some(NoteEvent::Note { pitch: 64 })
    );
}

#[test]
fn structural_edit_supports_undo_and_redo() {
    let mut app = App::default();
    let initial_tracks = app.song.tracks.len();

    app.create_track();
    assert_eq!(app.song.tracks.len(), initial_tracks + 1);
    app.undo();
    assert_eq!(app.song.tracks.len(), initial_tracks);
    app.redo();
    assert_eq!(app.song.tracks.len(), initial_tracks + 1);
}

#[test]
fn continuous_parameter_changes_merge_and_restore_both_directions() {
    let mut app = App::default();
    let original_bpm = app.song.transport.bpm;

    app.set_bpm(121);
    app.set_bpm(122);
    app.set_bpm(123);

    assert_eq!(app.history.undo_len(), 1);
    app.undo();
    assert_eq!(app.song.transport.bpm, original_bpm);
    app.redo();
    assert_eq!(app.song.transport.bpm, 123);
}

#[test]
fn failed_outer_transaction_does_not_mutate_song_or_history() {
    let mut app = App::default();
    let before = app.song.clone();

    let result = app.transact_song(TransactionSpec::new("Invalid edit"), |transaction, _| {
        transaction.song_mut().transport.bpm = 150;
        transaction.nested(|nested| {
            nested.song_mut().transport.lines_per_beat = 8;
            Err::<(), _>("reject nested edit")
        })?;
        Ok(())
    });

    assert_eq!(result, Err("reject nested edit"));
    assert_eq!(app.song, before);
    assert_eq!(app.history.undo_len(), 0);
    assert!(!app.dirty);
}

#[test]
fn handled_nested_failure_rolls_back_only_nested_work() {
    let mut app = App::default();
    let original_lpb = app.song.transport.lines_per_beat;

    let result = app.transact_song(
        TransactionSpec::new("Grouped transport edit"),
        |transaction, _| {
            transaction.song_mut().transport.bpm = 135;
            let nested_result = transaction.nested(|nested| {
                nested.song_mut().transport.lines_per_beat = 8;
                Err::<(), _>("invalid LPB")
            });
            assert_eq!(nested_result, Err("invalid LPB"));
            Ok::<(), std::convert::Infallible>(())
        },
    );

    assert_eq!(result, Ok(true));
    assert_eq!(app.song.transport.bpm, 135);
    assert_eq!(app.song.transport.lines_per_beat, original_lpb);
    assert_eq!(app.history.undo_len(), 1);
}

#[test]
fn configured_limit_bounds_app_history() {
    let mut app = App::new(AppConfig {
        history: config::HistoryConfig { undo_limit: 2 },
        ..AppConfig::default()
    });

    for bpm in [121, 122, 123] {
        app.mutate_song(|song, _| song.transport.bpm = bpm);
    }

    assert_eq!(app.history.undo_len(), 2);
    app.undo();
    app.undo();
    assert_eq!(app.song.transport.bpm, 121);
    app.undo();
    assert_eq!(app.song.transport.bpm, 121);
}

#[test]
fn new_edit_after_undo_clears_redo_and_dirty_tracks_song_content() {
    let mut app = App::default();
    app.mutate_song(|song, _| song.transport.bpm = 130);
    app.undo();

    assert!(!app.dirty);
    assert_eq!(app.history.redo_len(), 1);
    app.mutate_song(|song, _| song.transport.lines_per_beat = 8);

    assert!(app.dirty);
    assert_eq!(app.history.redo_len(), 0);
    app.redo();
    assert_eq!(app.song.transport.lines_per_beat, 8);
}

#[test]
fn history_and_runtime_state_are_outside_project_serialization() {
    let mut app = App::default();
    app.mutate_song(|song, _| song.transport.bpm = 144);
    app.is_playing = true;
    app.playhead_row = Some(12);
    let history_len = app.history.undo_len();

    let serialized = serde_json::to_string(&app.song).expect("serialize song");
    let restored: Song = serde_json::from_str(&serialized).expect("deserialize song");

    assert_eq!(restored, app.song);
    assert!(!serialized.contains("history"));
    assert!(!serialized.contains("playhead"));
    assert_eq!(app.history.undo_len(), history_len);
}
