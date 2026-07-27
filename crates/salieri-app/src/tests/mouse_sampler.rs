use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 100,
        terminal_height: 32,
    }
}

fn click(app: &mut App, kind: MouseEventKind) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column: 12,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
}

fn register_action(app: &mut App, action: SamplerAction) {
    app.interaction_map.register_with_payload(
        interaction_region::SAMPLER_ACTION,
        ratatui::layout::Rect::new(10, 10, 12, 1),
        InteractionPayload::SamplerAction { action },
    );
}

fn load_test_sample(app: &mut App, label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "salieri-sampler-mouse-{label}-{}.wav",
        std::process::id()
    ));
    let samples = (0..8_192)
        .map(|index| ((index % 256) as i16).saturating_mul(64))
        .collect::<Vec<_>>();
    std::fs::write(&path, wav_pcm16_bytes(44_100, 1, &samples)).expect("write wav");
    enter_command(app, &format!("sample view {}", path.display()));
    assert_eq!(app.mode, AppMode::Sampler);
    path
}

#[test]
fn sampler_mouse_selects_and_adjusts_the_exact_envelope_field() {
    let mut app = App::default();
    let path = load_test_sample(&mut app, "envelope");
    let before = app.tui_sampler_view().expect("sampler").envelope.1;

    register_action(
        &mut app,
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Decay),
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Decay);
    assert_eq!(app.tui_sampler_view().expect("sampler").envelope.1, before);

    register_action(&mut app, SamplerAction::IncrementEnvelope);
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert!(app.tui_sampler_view().expect("sampler").envelope.1 > before);

    register_action(&mut app, SamplerAction::DecrementEnvelope);
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.tui_sampler_view().expect("sampler").envelope.1, before);

    let _ = std::fs::remove_file(path);
}

#[test]
fn sampler_mouse_zoom_and_pan_use_existing_waveform_actions() {
    let mut app = App::default();
    let path = load_test_sample(&mut app, "waveform");

    register_action(&mut app, SamplerAction::ZoomIn);
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sample_waveform_zoom, 2);

    register_action(&mut app, SamplerAction::PanRight);
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert!(app.sample_waveform_offset > 0);

    register_action(&mut app, SamplerAction::PanLeft);
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sample_waveform_offset, 0);

    register_action(&mut app, SamplerAction::ZoomOut);
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sample_waveform_zoom, 1);

    let _ = std::fs::remove_file(path);
}

#[test]
fn sampler_wheel_pans_only_over_the_loaded_waveform_region() {
    let mut app = App::default();
    let path = load_test_sample(&mut app, "wheel-waveform");
    app.zoom_sample_waveform_in();
    app.interaction_map.register(
        interaction_region::VIEW_SAMPLER,
        ratatui::layout::Rect::new(0, 3, 100, 28),
    );

    click(&mut app, MouseEventKind::ScrollDown);
    assert_eq!(app.sample_waveform_offset, 0);

    app.interaction_map.register(
        interaction_region::SAMPLER_WAVEFORM,
        ratatui::layout::Rect::new(10, 8, 70, 12),
    );
    click(&mut app, MouseEventKind::ScrollDown);
    assert!(app.sample_waveform_offset > 0);

    click(&mut app, MouseEventKind::ScrollLeft);
    assert_eq!(app.sample_waveform_offset, 0);

    let _ = std::fs::remove_file(path);
}

#[test]
fn sampler_browse_control_opens_browser_without_a_loaded_sample() {
    let mut app = App::default();
    app.open_sampler_view();
    register_action(&mut app, SamplerAction::Browse);

    click(&mut app, MouseEventKind::Down(MouseButton::Left));

    assert_eq!(app.mode, AppMode::SampleBrowser);
    assert!(app.sample_browser_view.is_some());
}

#[test]
fn sampler_secondary_drag_mismatched_and_stale_targets_are_no_ops() {
    let mut app = App::default();
    let path = load_test_sample(&mut app, "no-ops");
    register_action(
        &mut app,
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Release),
    );

    click(&mut app, MouseEventKind::Down(MouseButton::Right));
    click(&mut app, MouseEventKind::Drag(MouseButton::Left));
    assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Attack);

    app.interaction_map.register_with_payload(
        interaction_region::SAMPLER_ACTION,
        ratatui::layout::Rect::new(10, 10, 12, 1),
        InteractionPayload::DspPaletteEntry { index: 0 },
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Attack);

    app.interaction_map.register_with_payload(
        interaction_region::VIEW_SAMPLER,
        ratatui::layout::Rect::new(10, 10, 12, 1),
        InteractionPayload::SamplerAction {
            action: SamplerAction::SelectEnvelope(SamplerEnvelopeField::Decay),
        },
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Attack);

    app.sample_view = None;
    register_action(
        &mut app,
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Sustain),
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Attack);

    let _ = std::fs::remove_file(path);
}
