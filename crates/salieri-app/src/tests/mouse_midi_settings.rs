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

fn register_action(app: &mut App, action: MidiSettingsAction) {
    app.interaction_map.register_with_payload(
        interaction_region::MIDI_SETTINGS_ACTION,
        ratatui::layout::Rect::new(60, 20, 12, 1),
        InteractionPayload::MidiSettingsAction { action },
    );
}

#[test]
fn midi_port_click_selects_and_connect_action_uses_selected_port() {
    let mut app = App {
        midi_ports: vec![
            MidiOutputPort {
                index: 0,
                name: "First".to_string(),
            },
            MidiOutputPort {
                index: 4,
                name: "Second".to_string(),
            },
        ],
        mode: AppMode::MidiSettings,
        ..App::default()
    };
    app.interaction_map.register_with_payload(
        interaction_region::MIDI_SETTINGS_PORT,
        ratatui::layout::Rect::new(20, 8, 50, 1),
        InteractionPayload::MidiPortRow { index: 1 },
    );

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 22, 8);
    assert_eq!(app.midi_port_cursor, 1);
    assert_eq!(app.midi_status, "MIDI Disconnected");

    register_action(&mut app, MidiSettingsAction::Connect);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 62, 20);
    assert_eq!(app.midi_status, "MIDI Connecting 4");
}

#[test]
fn midi_close_action_restores_previous_focus() {
    let mut app = App {
        mode: AppMode::MidiSettings,
        ..App::default()
    };
    register_action(&mut app, MidiSettingsAction::Close);

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 62, 20);

    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn midi_settings_ignores_empty_outside_secondary_drag_and_invalid_rows() {
    let mut app = App {
        midi_ports: Vec::new(),
        mode: AppMode::MidiSettings,
        ..App::default()
    };
    app.interaction_map.register_with_payload(
        interaction_region::MIDI_SETTINGS_PORT,
        ratatui::layout::Rect::new(20, 8, 50, 1),
        InteractionPayload::MidiPortRow { index: usize::MAX },
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 22, 8);
    assert_eq!(app.midi_port_cursor, 0);

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert_eq!(app.mode, AppMode::MidiSettings);
    assert!(!app.is_playing);

    for kind in [
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        register_action(&mut app, MidiSettingsAction::Close);
        click(&mut app, kind, 62, 20);
        assert_eq!(app.mode, AppMode::MidiSettings);
    }
}
