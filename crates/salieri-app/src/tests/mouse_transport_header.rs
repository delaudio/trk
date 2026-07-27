use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 100,
        terminal_height: 32,
    }
}

fn register_action(app: &mut App, action: TransportAction, column: u16) {
    app.interaction_map.register_with_payload(
        interaction_region::TRANSPORT_ACTION,
        ratatui::layout::Rect::new(column, 1, 1, 1),
        InteractionPayload::TransportAction { action },
    );
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

#[test]
fn play_click_starts_pattern_without_toggling_active_playback() {
    let mut app = App::default();
    register_action(&mut app, TransportAction::Play, 3);

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 3, 1);
    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 3, 1);
    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(0));
}

#[test]
fn stop_click_is_idempotent_when_playback_is_already_stopped() {
    let mut app = App::default();
    register_action(&mut app, TransportAction::Stop, 7);

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 7, 1);
    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);

    app.start_playback();
    assert!(app.is_playing);
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 7, 1);
    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);
}

#[test]
fn stop_click_clears_clip_launcher_and_transport_state_in_clips_view() {
    let mut app = App {
        mode: AppMode::Clips,
        active_clip_scene: Some(0),
        queued_clip_scene: Some(1),
        is_playing: true,
        playhead_row: Some(12),
        ..App::default()
    };
    register_action(&mut app, TransportAction::Stop, 7);

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 7, 1);

    assert_eq!(app.active_clip_scene, None);
    assert_eq!(app.queued_clip_scene, None);
    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);
}

#[test]
fn record_and_non_primary_header_input_do_not_change_playback() {
    let mut record = App::default();
    record.interaction_map.register(
        interaction_region::APP_HEADER,
        ratatui::layout::Rect::new(0, 0, 100, 3),
    );
    click(&mut record, MouseEventKind::Down(MouseButton::Left), 11, 1);
    assert!(!record.is_playing);

    let mut secondary = App::default();
    register_action(&mut secondary, TransportAction::Play, 3);
    click(
        &mut secondary,
        MouseEventKind::Down(MouseButton::Right),
        3,
        1,
    );
    assert!(!secondary.is_playing);

    click(
        &mut secondary,
        MouseEventKind::Drag(MouseButton::Left),
        3,
        1,
    );
    assert!(!secondary.is_playing);
}

#[test]
fn invalid_transport_target_does_not_change_playback() {
    let mut app = App::default();
    app.interaction_map.register_with_payload(
        interaction_region::TRANSPORT_ACTION,
        ratatui::layout::Rect::new(3, 1, 1, 1),
        InteractionPayload::ConfirmationAction {
            action: ConfirmationAction::Confirm,
        },
    );

    click(&mut app, MouseEventKind::Down(MouseButton::Left), 3, 1);

    assert!(!app.is_playing);
    assert_eq!(app.playhead_row, None);
}
