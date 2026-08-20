use super::*;

#[test]
fn history_modal_restores_selected_take_and_undo_reconciles_active_version() {
    let mut app = App::default();
    let mut first = app.song.patterns[0].clone();
    first
        .set_note(0, 0, NoteEvent::Note { pitch: 48 }, 100)
        .expect("first take");
    let mut second = first.clone();
    second
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 110)
        .expect("second take");
    let first_id = app
        .variation_history
        .record_at(
            100,
            "first bass take",
            PatternVariationSource::AiProposal,
            0,
            Some(0),
            first.clone(),
        )
        .expect("record first");
    let second_id = app
        .variation_history
        .record_at(
            200,
            "second bass take",
            PatternVariationSource::EuclideanTransform,
            0,
            Some(0),
            second.clone(),
        )
        .expect("record second");
    app.song.patterns[0] = second.clone();
    app.clean_song = app.song.clone();
    app.clean_variation_history = app.variation_history.clone();
    app.refresh_dirty();

    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    assert!(app.variation_history_open);
    assert_eq!(app.variation_history_cursor, 1);
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert!(!app.variation_history_open);
    assert_eq!(app.song.patterns[0], first);
    assert_eq!(app.variation_history.active_id(), Some(first_id));
    assert!(app.dirty);
    assert_eq!(app.history.undo_len(), 1);

    app.undo();

    assert_eq!(app.song.patterns[0], second);
    assert_eq!(app.variation_history.active_id(), Some(second_id));
    assert!(!app.dirty);
}

#[test]
fn history_overlay_captures_keys_while_uppercase_v_retains_visual_selection() {
    let mut app = App::default();

    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    assert!(app.variation_history_open);
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
    assert!(!app.variation_history_open);
    assert!(app.selection.is_none());

    app.handle_key(KeyEvent::new(KeyCode::Char('V'), KeyModifiers::SHIFT));
    assert!(app.selection.is_some());
    assert!(!app.variation_history_open);

    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
    assert!(!app.variation_history_open);
}

#[test]
fn ordinary_edit_invalidates_an_active_take_that_no_longer_matches() {
    let mut app = App::default();
    let id = app
        .variation_history
        .record_at(
            100,
            "baseline take",
            PatternVariationSource::AiProposal,
            0,
            Some(0),
            app.song.patterns[0].clone(),
        )
        .expect("record baseline");
    assert_eq!(app.variation_history.active_id(), Some(id));

    app.mutate_song(|song, cursor| {
        song.patterns[0]
            .set_note(cursor.row, cursor.track, NoteEvent::Note { pitch: 72 }, 100)
            .expect("edit pattern");
    });

    assert_eq!(app.variation_history.active_id(), None);
}

#[test]
fn restoring_identical_snapshot_is_a_clean_no_op() {
    let mut app = App::default();
    let snapshot = app.song.patterns[0].clone();
    let first_id = app
        .variation_history
        .record_at(
            100,
            "first identical take",
            PatternVariationSource::AiProposal,
            0,
            Some(0),
            snapshot.clone(),
        )
        .expect("record first");
    let second_id = app
        .variation_history
        .record_at(
            200,
            "second identical take",
            PatternVariationSource::AiProposal,
            0,
            Some(0),
            snapshot,
        )
        .expect("record second");
    app.clean_variation_history = app.variation_history.clone();
    app.variation_history_open = true;
    app.variation_history_cursor = 0;

    app.restore_selected_variation();

    assert!(!app.variation_history_open);
    assert_eq!(app.variation_history.entries()[0].id, first_id);
    assert_eq!(app.variation_history.active_id(), Some(second_id));
    assert_eq!(app.history.undo_len(), 0);
    assert!(!app.dirty);
}
