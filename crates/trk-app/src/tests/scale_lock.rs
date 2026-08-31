use super::*;
use trk_core::ScaleMode;

#[test]
fn scale_command_configures_session_state_and_rejects_invalid_changes_atomically() {
    let mut app = App::default();
    let original_song = app.song.clone();

    type_command(&mut app, "scale D minor");

    assert!(app.scale_lock.enabled);
    assert_eq!(
        app.scale_lock.scale,
        HarmonicScale::new(2, ScaleMode::Minor).expect("scale")
    );
    assert_eq!(app.song, original_song);
    assert!(!app.dirty);
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notification| notification.message == "Scale Lock ON (D minor)"));

    type_command(&mut app, "scale H unknown");

    assert!(app.scale_lock.enabled);
    assert_eq!(
        app.scale_lock.scale,
        HarmonicScale::new(2, ScaleMode::Minor).expect("scale")
    );
    assert!(app.notification.as_ref().is_some_and(|notification| {
        notification.message.contains("Usage: :scale")
            && notification.kind == NotificationKind::Warning
    }));

    type_command(&mut app, "scale off");
    assert!(!app.scale_lock.enabled);
    assert_eq!(app.song, original_song);
}

#[test]
fn exact_uppercase_k_toggles_in_normal_and_edit_without_stealing_modified_keys() {
    let mut app = App::default();
    let original_cursor = app.cursor;

    app.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
    assert!(app.scale_lock.enabled);
    assert_eq!(app.cursor, original_cursor);
    assert!(!app.dirty);

    app.handle_key(KeyEvent::new(
        KeyCode::Char('K'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    assert!(app.scale_lock.enabled);

    app.mode = AppMode::Edit;
    app.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
    assert!(!app.scale_lock.enabled);
    assert_eq!(app.cursor, original_cursor);
}

#[test]
fn scale_locked_keyboard_maps_both_physical_rows_to_degrees_and_restores_chromatic_entry() {
    let mut app = App {
        octave: 3,
        ..App::default()
    };
    app.apply_scale_command(command::ScaleCommand::Select(
        HarmonicScale::new(2, ScaleMode::Minor).expect("D minor"),
    ));

    let lower = "zsxdcvgbhnjm"
        .chars()
        .map(|key| app.keyboard_note_for_entry(key).expect("lower degree"))
        .collect::<Vec<_>>();
    assert_eq!(lower, vec![50, 52, 53, 55, 57, 58, 60, 62, 64, 65, 67, 69]);
    let upper = "q2w3er5t6y7u"
        .chars()
        .map(|key| app.keyboard_note_for_entry(key).expect("upper degree"))
        .collect::<Vec<_>>();
    assert_eq!(upper, vec![62, 64, 65, 67, 69, 70, 72, 74, 76, 77, 79, 81]);

    app.mode = AppMode::Edit;
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert_eq!(
        app.song
            .pattern(0)
            .and_then(|pattern| pattern.cell(0, 0))
            .and_then(|cell| cell.note),
        Some(NoteEvent::Note { pitch: 50 })
    );
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(
        app.song
            .pattern(0)
            .and_then(|pattern| pattern.cell(0, 0))
            .and_then(|cell| cell.note),
        None
    );

    app.apply_scale_command(command::ScaleCommand::Off);
    assert_eq!(
        app.keyboard_note_for_entry('s'),
        keyboard_note('s', app.octave)
    );
}

#[test]
fn live_harmonic_label_follows_playhead_audibility_and_transport_state() {
    let mut app = App::default();
    app.apply_scale_command(command::ScaleCommand::Select(
        HarmonicScale::new(2, ScaleMode::Minor).expect("D minor"),
    ));
    let pattern = app.song.pattern_mut(0).expect("pattern");
    for (track, pitch) in [50, 53, 57, 60].into_iter().enumerate() {
        pattern
            .set_note(0, track, NoteEvent::Note { pitch }, 100)
            .expect("chord note");
        pattern.set_gate(0, track, Some(2)).expect("gate");
    }
    app.is_playing = true;
    app.playhead_row = Some(1);

    assert_eq!(app.current_chord_name().as_deref(), Some("Dm7"));
    assert_eq!(
        app.tui_harmonic_mode_label(TuiView::Pattern),
        "NORMAL K:D:min Dm7"
    );

    app.song.tracks[3].muted = true;
    assert_eq!(app.current_chord_name().as_deref(), Some("Dm"));
    app.is_playing = false;
    app.playhead_row = None;
    assert_eq!(app.current_chord_name(), None);
    assert_eq!(
        app.tui_harmonic_mode_label(TuiView::Pattern),
        "NORMAL K:D:min"
    );
}

#[test]
fn live_harmonic_label_uses_the_active_sequence_pattern() {
    let mut app = App::default();
    let active_pattern_id = app.song.create_pattern(8);
    let pattern = app.song.pattern_mut(1).expect("sequence pattern");
    for (track, pitch) in [53, 57, 60].into_iter().enumerate() {
        pattern
            .set_note(0, track, NoteEvent::Note { pitch }, 100)
            .expect("chord note");
    }
    app.song.sequence = vec![active_pattern_id];
    app.pattern_index = 0;
    app.sequence_position = Some(0);
    app.is_playing = true;
    app.playhead_row = Some(1);

    assert_eq!(app.current_chord_name().as_deref(), Some("F"));
}
