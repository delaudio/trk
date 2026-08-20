use super::*;

#[test]
fn calibration_modal_adjusts_and_resets_session_controls_without_dirtying_song() {
    let mut app = App::default();
    let target = app.song.tracks[0].id.0;

    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    assert!(app.calibration_open);
    assert_eq!(
        app.playback.calibration_settings().target_track_id,
        Some(target)
    );

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(app.playback.calibration_settings().master_gain, 1.1);
    assert!(!app.dirty);
    assert_eq!(app.history.undo_len(), 0);

    app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    assert_eq!(
        app.playback.calibration_settings(),
        CalibrationSettings {
            target_track_id: Some(target),
            ..CalibrationSettings::default()
        }
    );
    assert!(!app.dirty);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.calibration_open);
}

#[test]
fn calibration_captures_keys_while_ctrl_t_keeps_create_track_shortcut() {
    let mut app = App::default();
    let initial_tracks = app.song.tracks.len();

    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    assert_eq!(app.song.tracks.len(), initial_tracks + 1);
    assert!(!app.calibration_open);

    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    let row = app.cursor.row;
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.calibration_cursor, 1);
    assert_eq!(app.cursor.row, row);
    app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    assert!(!app.calibration_open);
}
