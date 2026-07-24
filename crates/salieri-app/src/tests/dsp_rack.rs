use super::*;

#[test]
fn dsp_rack_view_tracks_current_chain_and_keyboard_selection() {
    let mut app = App::default();

    enter_command(&mut app, "dsp track gain 0.500");
    enter_command(&mut app, "dsp track delay free 250 500 0.350 0.250 ping");
    enter_command(&mut app, "dsp master reverb 0.500 20 2.500 0.250");
    enter_command(&mut app, "focus dsp");

    let rack = app.tui_dsp_rack_view();
    assert_eq!(rack.track_effects.len(), 2);
    assert_eq!(rack.master_effects.len(), 1);
    assert_eq!(rack.selected_target, DspRackTargetView::Track);
    assert_eq!(rack.selected_index, 0);

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let rack = app.tui_dsp_rack_view();
    assert_eq!(rack.selected_target, DspRackTargetView::Track);
    assert_eq!(rack.selected_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let rack = app.tui_dsp_rack_view();
    assert_eq!(rack.selected_target, DspRackTargetView::Master);
    assert_eq!(rack.selected_index, 0);

    enter_command(&mut app, "dsp master clear");
    app.open_dsp_rack_view();
    let rack = app.tui_dsp_rack_view();
    assert!(rack.master_effects.is_empty());
    assert_eq!(rack.selected_index, 0);
}

#[test]
fn dsp_rack_palette_assigns_device_with_keyboard() {
    let mut app = App::default();

    enter_command(&mut app, "focus dsp");
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(app.tui_dsp_rack_view().device_palette.is_some());

    for _ in 0..5 {
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let rack = app.tui_dsp_rack_view();
    assert!(rack.device_palette.is_none());
    assert_eq!(rack.track_effects.len(), 1);
    assert!(matches!(
        rack.track_effects[0].kind,
        EffectDeviceKind::Filter { .. }
    ));

    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let rack = app.tui_dsp_rack_view();
    assert_eq!(rack.track_effects.len(), 1);
    assert!(matches!(
        rack.track_effects[0].kind,
        EffectDeviceKind::Filter { .. }
    ));
}

#[test]
fn dsp_rack_parameter_editor_adjusts_core_device_families() {
    let mut app = App::default();

    enter_command(&mut app, "dsp track gain 1.000");
    enter_command(&mut app, "dsp track filter lowpass 2000 0.250 0.000 0.500");
    enter_command(&mut app, "dsp track delay sync 250 500 0.350 0.250");
    enter_command(&mut app, "dsp track reverb 0.500 20 2.500 0.250");
    app.upsert_track_dsp_device(
        app.cursor.track,
        EffectDevice::compressor(14, CompressorSpec::default()),
    );
    enter_command(&mut app, "focus dsp");

    app.dsp_rack_cursor = 0;
    app.dsp_parameter_cursor = 0;
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 0),
        EffectDeviceKind::Gain { gain } if *gain > 1.0
    ));

    app.dsp_rack_cursor = 1;
    app.dsp_parameter_cursor = 0;
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 1),
        EffectDeviceKind::Filter { mode, .. } if *mode == FilterMode::HighPass
    ));
    app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 1),
        EffectDeviceKind::Filter { cutoff_hz, .. } if *cutoff_hz > 2000.0
    ));

    app.dsp_rack_cursor = 2;
    app.dsp_parameter_cursor = 0;
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 2),
        EffectDeviceKind::Delay { sync, .. } if !*sync
    ));
    app.dsp_parameter_cursor = 1;
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 2),
        EffectDeviceKind::Delay { time_left_ms, .. } if *time_left_ms > 250.0
    ));

    app.dsp_rack_cursor = 3;
    app.dsp_parameter_cursor = 2;
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 3),
        EffectDeviceKind::Reverb { decay_s, .. } if *decay_s > 2.5
    ));

    app.dsp_rack_cursor = 4;
    app.dsp_parameter_cursor = 1;
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    assert!(matches!(
        track_effect_kind(&app, 4),
        EffectDeviceKind::Compressor { ratio, .. } if *ratio > 4.0
    ));
}

#[test]
fn dsp_rack_mouse_selects_and_adjusts_parameter() {
    let mut app = App::default();

    enter_command(&mut app, "dsp track gain 1.000");
    enter_command(&mut app, "focus dsp");

    let viewport = MouseViewport {
        terminal_width: 160,
        terminal_height: 40,
        visible_rows: 12,
        visible_tracks: 4,
    };

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 19,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );
    assert_eq!(app.tui_dsp_rack_view().selected_parameter_index, 0);

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 4,
            row: 19,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );
    assert!(matches!(
        track_effect_kind(&app, 0),
        EffectDeviceKind::Gain { gain } if *gain > 1.0
    ));
}

fn track_effect_kind(app: &App, index: usize) -> &EffectDeviceKind {
    let track = app.song.tracks[app.cursor.track].id;
    &app.song
        .mixer
        .tracks
        .iter()
        .find(|mixer| mixer.track == track)
        .expect("track mixer exists")
        .effects[index]
        .kind
}
