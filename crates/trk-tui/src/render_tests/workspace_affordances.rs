use super::render_test_support::{render_test_state, test_waveform};
use super::*;
use crate::InteractionPayload;
use ratatui::{backend::TestBackend, style::Modifier, Terminal};
use trk_core::Song;
use trk_sampler::WaveformBucket;

fn render_large(song: Song, state: TuiState<'_>) -> (Terminal<TestBackend>, InteractionMap) {
    let backend = TestBackend::new(140, 36);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut interactions = InteractionMap::new();
    terminal
        .draw(|frame| {
            interactions = render_with_interactions(frame, &song, state);
        })
        .expect("draw");
    (terminal, interactions)
}

fn disabled_cell_coordinates(terminal: &Terminal<TestBackend>) -> Vec<(u16, u16)> {
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    let mut coordinates = Vec::new();
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let cell = &buffer[(x, y)];
            if cell.symbol() == "×" {
                assert_eq!(cell.fg, theme::BORDER);
                assert!(cell.modifier.contains(Modifier::DIM));
                coordinates.push((x, y));
            }
        }
    }
    coordinates
}

fn find_text_start(terminal: &Terminal<TestBackend>, text: &str) -> Option<(u16, u16)> {
    let expected = text.chars().map(|ch| ch.to_string()).collect::<Vec<_>>();
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    for y in area.y..area.y.saturating_add(area.height) {
        let symbols = (area.x..area.x.saturating_add(area.width))
            .map(|x| buffer[(x, y)].symbol())
            .collect::<Vec<_>>();
        if let Some(index) = symbols
            .windows(expected.len())
            .position(|window| window == expected)
        {
            return Some((area.x.saturating_add(index as u16), y));
        }
    }
    None
}

#[test]
fn pattern_workspace_distinguishes_enabled_and_disabled_chrome() {
    let mut state = render_test_state();
    state.active_view = TuiView::Pattern;
    let (terminal, interactions) = render_large(Song::empty(), state);
    let disabled = disabled_cell_coordinates(&terminal);

    assert!(disabled.len() >= 2);
    for (x, y) in disabled {
        assert_eq!(
            interactions.hit_test(x, y).map(|region| region.payload),
            Some(InteractionPayload::None)
        );
    }
    for label in ["Instr.", "Samples"] {
        let (x, y) = find_text_start(&terminal, label).expect("enabled inspector tab");
        let cell = &terminal.backend().buffer()[(x, y)];
        assert_eq!(cell.fg, theme::ACCENT);
        assert!(!cell.modifier.contains(Modifier::DIM));
    }
}

#[test]
fn sampler_workspace_disables_placeholders_but_keeps_direct_actions_enabled() {
    let overview = test_waveform(vec![
        WaveformBucket {
            min: -0.5,
            max: 0.5,
        };
        16
    ]);
    let sampler = SamplerViewState {
        name: "break.wav",
        source_path: "/samples/break.wav",
        overview: &overview,
        gain: 1.0,
        waveform_start_bucket: 0,
        waveform_end_bucket: overview.buckets.len(),
        waveform_zoom: 1,
        instrument: Some("Break"),
        assigned_track: Some("Drums"),
        assigned_track_count: 1,
        playback_mode: "one-shot",
        start_frame: None,
        end_frame: None,
        loop_start_frame: None,
        loop_end_frame: None,
        envelope: (0.010, 0.050, 0.750, 0.100),
        selected_envelope: SamplerEnvelopeField::Attack,
    };
    let mut state = render_test_state();
    state.active_view = TuiView::Sampler;
    state.sampler_view = Some(sampler);
    let (terminal, interactions) = render_large(Song::empty(), state);
    let disabled = disabled_cell_coordinates(&terminal);

    assert!(disabled.len() >= 10);
    for (x, y) in disabled {
        assert_ne!(
            interactions.hit_test(x, y).map(|region| region.id),
            Some(interaction_region::SAMPLER_ACTION)
        );
    }
    let actions = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::SAMPLER_ACTION)
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 11);
    for region in actions {
        let cell = &terminal.backend().buffer()[(region.area.x, region.area.y)];
        assert!(!cell.modifier.contains(Modifier::DIM));
    }
}
