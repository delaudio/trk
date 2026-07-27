use super::render_test_support::{render_test_state, test_waveform};
use super::*;
use crate::{InteractionPayload, SamplerAction};
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;
use salieri_sampler::WaveformBucket;

fn sampler_interactions(
    width: u16,
    height: u16,
    sampler: Option<SamplerViewState<'_>>,
) -> InteractionMap {
    let song = Song::empty();
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut interactions = InteractionMap::new();
    terminal
        .draw(|frame| {
            let mut state = render_test_state();
            state.active_view = TuiView::Sampler;
            state.sampler_view = sampler;
            interactions = render_with_interactions(frame, &song, state);
        })
        .expect("draw");
    interactions
}

fn loaded_sampler(overview: &WaveformOverview) -> SamplerViewState<'_> {
    SamplerViewState {
        name: "break.wav",
        source_path: "/samples/break.wav",
        overview,
        gain: 1.0,
        waveform_start_bucket: 0,
        waveform_end_bucket: overview.buckets.len(),
        waveform_zoom: 2,
        instrument: Some("Break"),
        assigned_track: Some("Drums"),
        assigned_track_count: 1,
        playback_mode: "loop",
        start_frame: None,
        end_frame: None,
        loop_start_frame: None,
        loop_end_frame: None,
        envelope: (0.010, 0.050, 0.750, 0.100),
        selected_envelope: SamplerEnvelopeField::Attack,
    }
}

fn sampler_actions(interactions: &InteractionMap) -> Vec<SamplerAction> {
    interactions
        .regions()
        .iter()
        .filter_map(|region| {
            if region.id != interaction_region::SAMPLER_ACTION {
                return None;
            }
            let InteractionPayload::SamplerAction { action } = region.payload else {
                panic!("sampler action region has mismatched payload");
            };
            assert_eq!(region.area.height, 1);
            Some(action)
        })
        .collect()
}

#[test]
fn compact_and_large_sampler_layouts_expose_the_same_actions() {
    let overview = test_waveform(vec![
        WaveformBucket {
            min: -0.5,
            max: 0.5,
        };
        32
    ]);
    let expected = [
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Attack),
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Decay),
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Sustain),
        SamplerAction::SelectEnvelope(SamplerEnvelopeField::Release),
        SamplerAction::DecrementEnvelope,
        SamplerAction::IncrementEnvelope,
        SamplerAction::ZoomOut,
        SamplerAction::ZoomIn,
        SamplerAction::PanLeft,
        SamplerAction::PanRight,
        SamplerAction::Browse,
    ];

    for (width, height) in [(80, 28), (100, 32), (120, 32), (140, 36)] {
        let interactions = sampler_interactions(width, height, Some(loaded_sampler(&overview)));
        let actions = sampler_actions(&interactions);

        assert_eq!(actions.len(), expected.len(), "{width}x{height}");
        for expected_action in expected {
            assert!(actions.contains(&expected_action), "{width}x{height}");
        }
        for region in interactions
            .regions()
            .iter()
            .filter(|region| region.id == interaction_region::SAMPLER_ACTION)
        {
            assert_eq!(
                interactions
                    .hit_test(region.area.x, region.area.y)
                    .map(|hit| hit.payload),
                Some(region.payload)
            );
        }
    }
}

#[test]
fn empty_sampler_exposes_only_browse_in_both_layouts() {
    for (width, height) in [(80, 24), (140, 36)] {
        let interactions = sampler_interactions(width, height, None);
        assert_eq!(
            sampler_actions(&interactions),
            vec![SamplerAction::Browse],
            "{width}x{height}"
        );
    }
}

#[test]
fn clipped_controls_never_register_partial_targets() {
    let overview = test_waveform(vec![WaveformBucket {
        min: -0.5,
        max: 0.5,
    }]);
    let interactions = sampler_interactions(40, 20, Some(loaded_sampler(&overview)));

    for region in interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::SAMPLER_ACTION)
    {
        assert!(region.area.x.saturating_add(region.area.width) <= 39);
    }
    assert!(!sampler_actions(&interactions).contains(&SamplerAction::Browse));
}
