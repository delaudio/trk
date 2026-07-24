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
