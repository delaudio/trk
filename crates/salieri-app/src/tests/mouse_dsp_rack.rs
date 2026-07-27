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
