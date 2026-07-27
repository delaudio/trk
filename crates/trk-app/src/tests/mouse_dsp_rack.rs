use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 100,
        terminal_height: 32,
    }
}

fn click(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
}

fn register_target(app: &mut App, target: DspRackChain, column: u16) {
    app.interaction_map.register_with_payload(
        interaction_region::DSP_RACK_TARGET,
        ratatui::layout::Rect::new(column, 5, 8, 1),
        InteractionPayload::DspRackTarget { target },
    );
}

fn register_device(app: &mut App, target: DspRackChain, index: usize, row: u16) {
    app.interaction_map.register_with_payload(
        interaction_region::DSP_DEVICE_ROW,
        ratatui::layout::Rect::new(2, row, 40, 1),
        InteractionPayload::DspDeviceRow { target, index },
    );
}

fn register_parameter(app: &mut App, index: usize, row: u16) {
    app.interaction_map.register_with_payload(
        interaction_region::DSP_PARAMETER_ROW,
        ratatui::layout::Rect::new(2, row, 40, 1),
        InteractionPayload::DspParameterRow { index },
    );
}

fn register_palette_entry(app: &mut App, index: usize, row: u16) {
    app.interaction_map.register_with_payload(
        interaction_region::DSP_PALETTE_ENTRY,
        ratatui::layout::Rect::new(2, row, 40, 1),
        InteractionPayload::DspPaletteEntry { index },
    );
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

#[test]
fn primary_click_selects_track_and_master_targets() {
    let mut app = App::default();
    enter_command(&mut app, "dsp track gain 1.000");
    enter_command(&mut app, "dsp master gain 1.000");
    enter_command(&mut app, "focus dsp");

    register_target(&mut app, DspRackChain::Master, 20);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 22, 5);
    assert_eq!(
        app.tui_dsp_rack_view().selected_target,
        DspRackTargetView::Master
    );

    register_target(&mut app, DspRackChain::Track, 8);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 10, 5);
    assert_eq!(
        app.tui_dsp_rack_view().selected_target,
        DspRackTargetView::Track
    );
}

#[test]
fn device_click_selects_clicked_chain_and_refreshes_parameter_selection() {
    let mut app = App::default();
    enter_command(&mut app, "dsp track filter lowpass 2000 0.250 0.000 0.500");
    enter_command(&mut app, "dsp master gain 1.000");
    enter_command(&mut app, "focus dsp");
    app.dsp_parameter_cursor = 4;

    register_device(&mut app, DspRackChain::Master, 0, 9);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 9);

    let rack = app.tui_dsp_rack_view();
    assert_eq!(rack.selected_target, DspRackTargetView::Master);
    assert_eq!(rack.selected_index, 0);
    assert_eq!(rack.selected_parameter_index, 0);
    assert!(matches!(
        rack.master_effects[0].kind,
        EffectDeviceKind::Gain { .. }
    ));
}

#[test]
fn empty_stale_and_invalid_rows_do_not_change_device_selection() {
    let mut app = App::default();
    enter_command(&mut app, "dsp track gain 1.000");
    enter_command(&mut app, "focus dsp");
    let before = app.tui_dsp_rack_view();
    let before_selection = (
        before.selected_target,
        before.selected_index,
        before.selected_parameter_index,
    );

    app.interaction_map.register(
        interaction_region::DSP_CHAIN,
        ratatui::layout::Rect::new(2, 7, 40, 1),
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 7);

    register_device(&mut app, DspRackChain::Master, usize::MAX, 9);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 9);

    app.interaction_map.register_with_payload(
        interaction_region::DSP_DEVICE_ROW,
        ratatui::layout::Rect::new(2, 11, 40, 1),
        InteractionPayload::ConfirmationAction {
            action: ConfirmationAction::Confirm,
        },
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 11);

    let after = app.tui_dsp_rack_view();
    assert_eq!(
        (
            after.selected_target,
            after.selected_index,
            after.selected_parameter_index,
        ),
        before_selection
    );
}

#[test]
fn secondary_clicks_and_drags_do_not_select_dsp_targets_or_devices() {
    for kind in [
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        let mut app = App::default();
        enter_command(&mut app, "dsp track gain 1.000");
        enter_command(&mut app, "dsp master gain 1.000");
        enter_command(&mut app, "focus dsp");
        register_target(&mut app, DspRackChain::Master, 20);
        register_device(&mut app, DspRackChain::Master, 0, 9);

        click(&mut app, kind, 22, 5);
        click(&mut app, kind, 4, 9);

        let rack = app.tui_dsp_rack_view();
        assert_eq!(rack.selected_target, DspRackTargetView::Track);
        assert_eq!(rack.selected_index, 0);
    }
}

#[test]
fn parameter_click_selects_payload_and_right_click_adjusts_clicked_parameter() {
    let mut app = App::default();
    enter_command(&mut app, "dsp track filter lowpass 2000 0.250 0.000 0.500");
    enter_command(&mut app, "focus dsp");

    register_parameter(&mut app, 2, 5);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 5);
    assert_eq!(app.tui_dsp_rack_view().selected_parameter_index, 2);

    register_parameter(&mut app, 1, 7);
    click(&mut app, MouseEventKind::Down(MouseButton::Right), 4, 7);
    assert_eq!(app.tui_dsp_rack_view().selected_parameter_index, 1);
    assert!(matches!(
        track_effect_kind(&app, 0),
        EffectDeviceKind::Filter { cutoff_hz, .. } if *cutoff_hz > 2000.0
    ));
}

#[test]
fn parameter_help_drag_invalid_and_stale_targets_are_no_ops() {
    let mut app = App::default();
    enter_command(&mut app, "dsp track filter lowpass 2000 0.250 0.000 0.500");
    enter_command(&mut app, "focus dsp");
    app.dsp_parameter_cursor = 2;

    app.interaction_map.register(
        interaction_region::VIEW_DSP_RACK,
        ratatui::layout::Rect::new(2, 5, 40, 1),
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 5);

    register_parameter(&mut app, 1, 7);
    click(&mut app, MouseEventKind::Drag(MouseButton::Left), 4, 7);

    register_parameter(&mut app, usize::MAX, 9);
    click(&mut app, MouseEventKind::Down(MouseButton::Right), 4, 9);

    app.interaction_map.register_with_payload(
        interaction_region::DSP_PARAMETER_ROW,
        ratatui::layout::Rect::new(2, 11, 40, 1),
        InteractionPayload::DspPaletteEntry { index: 1 },
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 11);

    assert_eq!(app.tui_dsp_rack_view().selected_parameter_index, 2);
    assert!(matches!(
        track_effect_kind(&app, 0),
        EffectDeviceKind::Filter { cutoff_hz, .. } if (*cutoff_hz - 2000.0).abs() < f32::EPSILON
    ));
}

#[test]
fn scrolled_palette_payload_assigns_exact_device_type() {
    let mut app = App::default();
    enter_command(&mut app, "focus dsp");
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    register_palette_entry(&mut app, 12, 5);

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 4, 5);

    let rack = app.tui_dsp_rack_view();
    assert!(rack.device_palette.is_none());
    assert!(matches!(
        rack.track_effects[0].kind,
        EffectDeviceKind::Phaser { .. }
    ));
}

#[test]
fn palette_border_secondary_drag_invalid_and_stale_targets_are_no_ops() {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        let mut app = App::default();
        enter_command(&mut app, "focus dsp");
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
            app.interaction_map.register_with_payload(
                interaction_region::DSP_PALETTE_ENTRY,
                ratatui::layout::Rect::new(2, 5, 40, 1),
                InteractionPayload::DspParameterRow { index: 1 },
            );
        } else {
            register_palette_entry(&mut app, 7, 5);
        }

        click(&mut app, kind, 4, 5);

        let rack = app.tui_dsp_rack_view();
        assert!(rack.device_palette.is_some());
        assert!(rack.track_effects.is_empty());
    }

    let mut stale = App::default();
    enter_command(&mut stale, "focus dsp");
    stale.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    register_palette_entry(&mut stale, usize::MAX, 5);
    click(&mut stale, MouseEventKind::Down(MouseButton::Left), 4, 5);
    assert!(stale.tui_dsp_rack_view().device_palette.is_some());
    assert!(stale.tui_dsp_rack_view().track_effects.is_empty());
}
