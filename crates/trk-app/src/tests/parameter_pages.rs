use super::*;

fn assigned_sample_app() -> App {
    let mut app = App::default();
    let sample = app.song.upsert_sample_reference("samples/kick.wav", "Kick");
    let track = app.song.tracks[0].id;
    app.song
        .assign_sample_to_track(track, sample)
        .expect("assign sample");
    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("note");
    app
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn function_keys_open_switch_and_repeat_into_deep_editors() {
    let mut app = App::default();

    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::ParameterPage);
    assert_eq!(app.parameter_surface.page, ParameterPage::Source);
    assert_eq!(app.tui_active_view(), TuiView::ParameterPage);

    app.handle_key(key(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(app.parameter_surface.page, ParameterPage::Filter);
    app.handle_key(key(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::DspRack);

    app.open_tracker_view();
    app.handle_key(key(KeyCode::Char('3'), KeyModifiers::NONE));
    assert_eq!(app.parameter_surface.page, ParameterPage::Amp);
    app.handle_key(key(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_key(key(KeyCode::F(6), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::F(6), KeyModifiers::NONE));
    assert_eq!(app.mode, AppMode::Strudel);
}

#[test]
fn parameter_page_keys_take_precedence_over_normal_keymap_bindings() {
    let mut app = assigned_sample_app();
    let mut config = crate::keymap::KeymapConfig::default();
    config.normal.insert("q".to_string(), "help".to_string());
    app.keymap = crate::keymap::Keymap::from_config(&config).expect("valid keymap");
    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));

    app.handle_key(key(KeyCode::Char('q'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::ParameterPage);
    assert_eq!(app.parameter_surface.selected, 0);
}

#[test]
fn modified_function_keys_remain_available_to_the_normal_keymap() {
    let mut app = App::default();
    let mut config = crate::keymap::KeymapConfig::default();
    config
        .normal
        .insert("control+f1".to_string(), "help".to_string());
    app.keymap = crate::keymap::Keymap::from_config(&config).expect("valid keymap");

    app.handle_key(key(KeyCode::F(1), KeyModifiers::CONTROL));

    assert_eq!(app.mode, AppMode::Help);
}

#[test]
fn encoder_adjustment_sets_only_the_current_row_lock_and_undoes() {
    let mut app = assigned_sample_app();
    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char('+'), KeyModifiers::NONE));

    let cell = app
        .song
        .pattern(0)
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
    assert!(cell.parameter_locks.iter().any(|lock| {
        lock.parameter == ParameterId::from(SAMPLE_GAIN_PARAMETER_ID)
            && matches!(lock.action, ParameterLockAction::Set { .. })
    }));
    assert!(app.tui_parameter_page_slots()[0].locked);

    app.undo();
    let cell = app
        .song
        .pattern(0)
        .expect("pattern")
        .cell(0, 0)
        .expect("cell");
    assert!(cell.parameter_locks.is_empty());
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
}

#[test]
fn bounded_encoder_adjustment_does_not_create_a_no_op_lock() {
    let mut app = assigned_sample_app();
    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char('a'), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char('-'), KeyModifiers::NONE));

    assert!(app.song.pattern(0).expect("pattern").rows[0].cells[0]
        .parameter_locks
        .is_empty());
}

#[test]
fn backspace_then_encoder_clears_only_that_lock_on_selected_pattern() {
    let mut app = assigned_sample_app();
    let second = app.song.create_pattern(16);
    app.pattern_index = app
        .song
        .patterns
        .iter()
        .position(|pattern| pattern.id == second)
        .expect("second pattern");
    app.song
        .pattern_mut(app.pattern_index)
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 64 }, 90)
        .expect("note");
    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char('w'), KeyModifiers::SHIFT));
    assert!(app.song.pattern(0).expect("first").rows[0].cells[0]
        .parameter_locks
        .is_empty());
    assert_eq!(
        app.song.pattern(app.pattern_index).expect("second").rows[0].cells[0]
            .parameter_locks
            .len(),
        1
    );

    app.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char('w'), KeyModifiers::NONE));

    let cell = &app.song.pattern(app.pattern_index).expect("second").rows[0].cells[0];
    assert!(cell.parameter_locks.is_empty());
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 64 }));
    assert_eq!(cell.velocity, Some(90));
}

#[test]
fn pointer_wheel_adjusts_exact_slot_and_disabled_slots_are_inert() {
    let mut app = assigned_sample_app();
    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));
    app.interaction_map.register_with_payload(
        interaction_region::PARAMETER_PAGE_SLOT,
        ratatui::layout::Rect::new(10, 8, 12, 4),
        InteractionPayload::ParameterPageSlot { index: 1 },
    );
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 12,
            row: 9,
            modifiers: KeyModifiers::NONE,
        },
        MouseViewport {
            terminal_width: 100,
            terminal_height: 30,
        },
    );
    let cell = &app.song.pattern(0).expect("pattern").rows[0].cells[0];
    assert_eq!(cell.parameter_locks.len(), 1);
    assert_eq!(
        cell.parameter_locks[0].parameter,
        ParameterId::from(trk_core::SAMPLE_ROOT_NOTE_PARAMETER_ID)
    );

    app.interaction_map.register_with_payload(
        interaction_region::PARAMETER_PAGE_SLOT,
        ratatui::layout::Rect::new(30, 8, 12, 4),
        InteractionPayload::ParameterPageSlot { index: 0 },
    );
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 32,
            row: 9,
            modifiers: KeyModifiers::NONE,
        },
        MouseViewport {
            terminal_width: 100,
            terminal_height: 30,
        },
    );
    let cell = &app.song.pattern(0).expect("pattern").rows[0].cells[0];
    assert_eq!(cell.parameter_locks.len(), 1);
    assert_eq!(app.parameter_surface.selected, 0);

    let mut disabled = App::default();
    disabled.handle_key(key(KeyCode::F(6), KeyModifiers::NONE));
    disabled.interaction_map.register_with_payload(
        interaction_region::PARAMETER_PAGE_SLOT,
        ratatui::layout::Rect::new(10, 8, 12, 4),
        InteractionPayload::ParameterPageSlot { index: 0 },
    );
    let before = disabled.song.clone();
    disabled.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 12,
            row: 9,
            modifiers: KeyModifiers::NONE,
        },
        MouseViewport {
            terminal_width: 100,
            terminal_height: 30,
        },
    );
    assert_eq!(disabled.song, before);
}

#[test]
fn unset_sample_end_is_visible_but_inert_until_defined() {
    let mut app = assigned_sample_app();
    app.handle_key(key(KeyCode::F(1), KeyModifiers::NONE));

    let slots = app.tui_parameter_page_slots();
    assert_eq!(slots[3].label, "End");
    assert!(!slots[3].enabled);
    assert_eq!(
        slots[3].disabled_reason.as_deref(),
        Some("Set sample end in Sampler")
    );

    app.parameter_surface.selected = 3;
    app.handle_key(key(KeyCode::Char('+'), KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Backspace, KeyModifiers::NONE));
    app.handle_key(key(KeyCode::Char('r'), KeyModifiers::NONE));

    assert!(app.song.pattern(0).expect("pattern").rows[0].cells[0]
        .parameter_locks
        .is_empty());
}

#[test]
fn temporary_save_reload_and_quick_mute_preserve_workflow_context() {
    let mut app = App::default();
    app.cursor.row = 7;
    app.handle_key(key(KeyCode::Char('S'), KeyModifiers::SHIFT));
    assert!(app.parameter_surface.snapshot.is_some());

    app.handle_key(key(KeyCode::Char('1'), KeyModifiers::SHIFT));
    assert!(app.song.tracks[0].muted);
    assert_eq!(app.cursor.row, 7);
    assert_eq!(app.mode, AppMode::Normal);

    app.handle_key(key(KeyCode::Char('R'), KeyModifiers::SHIFT));
    assert!(!app.song.tracks[0].muted);
    assert_eq!(app.cursor.row, 7);

    app.undo();
    assert!(app.song.tracks[0].muted);
}

#[test]
fn queued_performance_reloads_keep_latest_token_and_clear_on_stop() {
    let mut app = App::default();
    app.temp_save_performance_state();
    app.is_playing = true;

    app.reload_temp_performance_state();
    let first_token = app
        .parameter_surface
        .pending_reload
        .as_ref()
        .expect("first pending reload")
        .0;
    app.reload_temp_performance_state();
    let latest_token = app
        .parameter_surface
        .pending_reload
        .as_ref()
        .expect("latest pending reload")
        .0;
    assert!(latest_token > first_token);

    app.apply_pending_performance_reload(first_token);
    assert_eq!(
        app.parameter_surface
            .pending_reload
            .as_ref()
            .map(|(token, _)| *token),
        Some(latest_token)
    );
    app.apply_pending_performance_reload(latest_token);
    assert!(app.parameter_surface.pending_reload.is_none());

    app.reload_temp_performance_state();
    assert!(app.parameter_surface.pending_reload.is_some());
    app.dispatch_event(AppEvent::Runtime(RuntimeEvent::PlaybackUpdate(
        crate::playback_runtime::PlaybackUpdate::Stopped,
    )));
    assert!(app.parameter_surface.pending_reload.is_none());
}

#[test]
fn shifted_encoder_keys_remain_coarse_controls_inside_parameter_pages() {
    let mut app = assigned_sample_app();
    app.handle_key(key(KeyCode::F(3), KeyModifiers::NONE));

    app.handle_key(key(KeyCode::Char('S'), KeyModifiers::SHIFT));
    app.handle_key(key(KeyCode::Char('R'), KeyModifiers::SHIFT));

    assert!(app.parameter_surface.snapshot.is_none());
    let locks = &app.song.pattern(0).expect("pattern").rows[0].cells[0].parameter_locks;
    assert!(locks.iter().any(|lock| {
        lock.parameter == ParameterId::from(trk_core::MIXER_TRACK_PAN_PARAMETER_ID)
    }));
    assert!(locks.iter().any(|lock| {
        lock.parameter == ParameterId::from(trk_core::SAMPLE_ENVELOPE_RELEASE_PARAMETER_ID)
    }));
}

#[test]
fn absent_quick_mute_track_is_inert() {
    let mut app = App::default();
    let before = app.song.clone();
    app.handle_key(key(KeyCode::Char('8'), KeyModifiers::SHIFT));
    assert_eq!(app.song, before);
}
