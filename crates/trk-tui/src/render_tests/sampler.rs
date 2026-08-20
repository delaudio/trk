use super::render_test_support::{render_test_state, test_waveform};
use super::*;
use crate::{InteractionPayload, SamplerAction, ScrollTarget};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use trk_core::Song;
use trk_sampler::WaveformBucket;

fn sampler_interactions(
    width: u16,
    height: u16,
    sampler: Option<SamplerViewState<'_>>,
) -> InteractionMap {
    render_sampler(width, height, sampler).1
}

fn render_sampler(
    width: u16,
    height: u16,
    sampler: Option<SamplerViewState<'_>>,
) -> (Buffer, InteractionMap) {
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
    (terminal.backend().buffer().clone(), interactions)
}

fn loaded_sampler(overview: &WaveformOverview) -> SamplerViewState<'_> {
    SamplerViewState {
        color_mode: TerminalColorMode::TrueColor,
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
fn only_loaded_waveforms_expose_a_sampler_scroll_target() {
    let overview = test_waveform(vec![
        WaveformBucket {
            min: -0.5,
            max: 0.5,
        };
        32
    ]);

    for (width, height) in [(80, 24), (140, 36)] {
        let loaded = sampler_interactions(width, height, Some(loaded_sampler(&overview)));
        let waveform = loaded
            .region(interaction_region::SAMPLER_WAVEFORM)
            .expect("loaded waveform region");
        assert_eq!(
            loaded.scroll_target_at(waveform.area.x, waveform.area.y),
            Some(ScrollTarget::SamplerWaveform)
        );

        let empty = sampler_interactions(width, height, None);
        assert!(empty.region(interaction_region::SAMPLER_WAVEFORM).is_none());
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

#[test]
fn rendered_waveform_buffer_honors_each_terminal_color_mode() {
    let overview = test_waveform(vec![
        WaveformBucket {
            min: -0.2,
            max: 0.2,
        },
        WaveformBucket {
            min: -0.9,
            max: 0.9,
        },
        WaveformBucket {
            min: -0.4,
            max: 0.5,
        },
    ]);

    for mode in [
        TerminalColorMode::TrueColor,
        TerminalColorMode::Indexed256,
        TerminalColorMode::Ansi16,
        TerminalColorMode::Monochrome,
    ] {
        let mut sampler = loaded_sampler(&overview);
        sampler.color_mode = mode;
        sampler.start_frame = Some(0);
        sampler.end_frame = Some(overview.frames);
        sampler.loop_start_frame = Some(overview.frames / 3);
        sampler.loop_end_frame = Some(overview.frames * 2 / 3);
        let (buffer, interactions) = render_sampler(100, 28, Some(sampler));
        let waveform = interactions
            .region(interaction_region::SAMPLER_WAVEFORM)
            .expect("waveform region");
        let buffer = &buffer;
        let cells = (waveform.area.y..waveform.area.y + waveform.area.height)
            .flat_map(|y| {
                (waveform.area.x..waveform.area.x + waveform.area.width)
                    .map(move |x| (buffer[(x, y)].fg, buffer[(x, y)].bg))
            })
            .collect::<Vec<_>>();

        match mode {
            TerminalColorMode::TrueColor => assert!(cells
                .iter()
                .any(|(foreground, _)| matches!(foreground, Color::Rgb(..)))),
            TerminalColorMode::Indexed256 => {
                assert!(cells
                    .iter()
                    .any(|(foreground, _)| matches!(foreground, Color::Indexed(_))));
                assert!(cells.iter().all(|(foreground, background)| {
                    !matches!(foreground, Color::Rgb(..)) && !matches!(background, Color::Rgb(..))
                }));
            }
            TerminalColorMode::Ansi16 => assert!(cells.iter().all(|(foreground, background)| {
                !matches!(foreground, Color::Rgb(..) | Color::Indexed(_))
                    && !matches!(background, Color::Rgb(..) | Color::Indexed(_))
            })),
            TerminalColorMode::Monochrome => {
                assert!(cells.iter().all(|(foreground, background)| {
                    matches!(foreground, Color::Reset) && matches!(background, Color::Reset)
                }));
            }
        }
    }
}
