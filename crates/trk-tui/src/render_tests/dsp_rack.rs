use super::render_test_support::render_test_state;
use super::*;
use crate::{DspRackChain, InteractionPayload};
use ratatui::{backend::TestBackend, Terminal};
use trk_core::{DelaySpec, EffectDevice, Song};

fn rack_interactions(
    width: u16,
    height: u16,
    track_effects: &[EffectDevice],
    master_effects: &[EffectDevice],
) -> InteractionMap {
    rack_view_interactions(
        width,
        height,
        DspRackViewState {
            track_name: "Track 01",
            track_number: 1,
            track_effects,
            master_effects,
            selected_target: DspRackTargetView::Track,
            selected_index: 0,
            selected_parameter_index: 0,
            selected_lock_status: DspParameterLockStatusView::Unlocked,
            device_palette: None,
        },
    )
}

fn rack_view_interactions(width: u16, height: u16, rack: DspRackViewState<'_>) -> InteractionMap {
    let song = Song::empty();
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut interactions = InteractionMap::new();
    terminal
        .draw(|frame| {
            let mut state = render_test_state();
            state.active_view = TuiView::DspRack;
            state.dsp_rack = Some(rack);
            interactions = render_with_interactions(frame, &song, state);
        })
        .expect("draw");
    interactions
}

#[test]
fn dsp_parameter_rows_follow_rendered_panel_at_multiple_heights() {
    let effects = [EffectDevice::filter(1, trk_core::FilterSpec::default())];
    let mut first_rows = Vec::new();

    for height in [24, 32, 48] {
        let interactions = rack_interactions(100, height, &effects, &[]);
        let rows = interactions
            .regions()
            .iter()
            .filter(|region| region.id == interaction_region::DSP_PARAMETER_ROW)
            .collect::<Vec<_>>();

        assert_eq!(rows.len(), 5, "height {height}");
        for (index, region) in rows.iter().enumerate() {
            assert_eq!(
                region.payload,
                InteractionPayload::DspParameterRow { index }
            );
            assert_eq!(region.area.height, 1);
        }
        first_rows.push(rows[0].area.y);
        assert_ne!(
            interactions
                .hit_test(rows[0].area.x, rows.last().unwrap().area.y + 1)
                .map(|region| region.id),
            Some(interaction_region::DSP_PARAMETER_ROW)
        );
    }

    assert!(first_rows.windows(2).all(|rows| rows[0] != rows[1]));
}

#[test]
fn dsp_palette_rows_carry_scrolled_absolute_indices() {
    let entry = DspDevicePaletteEntryView {
        label: "Device",
        summary: "summary",
    };
    let entries = [entry; 16];
    let interactions = rack_view_interactions(
        80,
        16,
        DspRackViewState {
            track_name: "Track 01",
            track_number: 1,
            track_effects: &[],
            master_effects: &[],
            selected_target: DspRackTargetView::Track,
            selected_index: 0,
            selected_parameter_index: 0,
            selected_lock_status: DspParameterLockStatusView::Unlocked,
            device_palette: Some(DspDevicePaletteViewState {
                entries: &entries,
                selected: 14,
            }),
        },
    );
    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::DSP_PALETTE_ENTRY)
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.first().map(|region| region.payload),
        Some(InteractionPayload::DspPaletteEntry { index: 11 })
    );
    assert_eq!(
        rows.last().map(|region| region.payload),
        Some(InteractionPayload::DspPaletteEntry { index: 15 })
    );
    assert!(rows
        .windows(2)
        .all(|rows| rows[1].area.y == rows[0].area.y + 1));
    assert_ne!(
        interactions
            .hit_test(rows[0].area.x, rows[0].area.y.saturating_sub(1))
            .map(|region| region.id),
        Some(interaction_region::DSP_PALETTE_ENTRY)
    );
}

#[test]
fn dsp_target_controls_expose_distinct_exact_targets_at_responsive_widths() {
    for width in [40, 80, 160] {
        let interactions = rack_interactions(width, 24, &[], &[]);
        let targets = interactions
            .regions()
            .iter()
            .filter(|region| region.id == interaction_region::DSP_RACK_TARGET)
            .collect::<Vec<_>>();

        assert_eq!(targets.len(), 2, "width {width}");
        assert_eq!(
            targets[0].payload,
            InteractionPayload::DspRackTarget {
                target: DspRackChain::Track,
            }
        );
        assert_eq!(
            targets[1].payload,
            InteractionPayload::DspRackTarget {
                target: DspRackChain::Master,
            }
        );
        assert_eq!(targets[0].area.height, 1);
        assert_eq!(targets[0].area.width, "[Track]".len() as u16);
        assert_eq!(targets[1].area.height, 1);
        assert_eq!(targets[1].area.width, "[Master]".len() as u16);
        assert_eq!(
            interactions
                .hit_test(targets[0].area.x, targets[0].area.y)
                .map(|region| region.payload),
            Some(targets[0].payload)
        );
    }
}

#[test]
fn dsp_target_controls_do_not_claim_clipped_text() {
    let interactions = rack_interactions(30, 16, &[], &[]);
    let targets = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::DSP_RACK_TARGET)
        .collect::<Vec<_>>();

    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets[0].payload,
        InteractionPayload::DspRackTarget {
            target: DspRackChain::Track,
        }
    );
}

#[test]
fn dsp_device_rows_carry_chain_and_visible_absolute_index() {
    let track_effects = (0..20)
        .map(|index| EffectDevice::gain(index + 1, 1.0))
        .collect::<Vec<_>>();
    let master_effects = [EffectDevice::pan(30, 0.0)];
    let interactions = rack_interactions(80, 16, &track_effects, &master_effects);
    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::DSP_DEVICE_ROW)
        .collect::<Vec<_>>();
    let track_rows = rows
        .iter()
        .filter(|region| {
            matches!(
                region.payload,
                InteractionPayload::DspDeviceRow {
                    target: DspRackChain::Track,
                    ..
                }
            )
        })
        .collect::<Vec<_>>();

    assert!(!track_rows.is_empty());
    assert!(track_rows.len() < track_effects.len());
    for (index, region) in track_rows.iter().enumerate() {
        assert_eq!(
            region.payload,
            InteractionPayload::DspDeviceRow {
                target: DspRackChain::Track,
                index,
            }
        );
        assert_eq!(region.area.height, 1);
    }
    assert_eq!(
        rows.last().map(|region| region.payload),
        Some(InteractionPayload::DspDeviceRow {
            target: DspRackChain::Master,
            index: 0,
        })
    );
}

#[test]
fn dsp_device_rows_scroll_to_the_selected_absolute_index() {
    let track_effects = (0..20)
        .map(|index| EffectDevice::gain(index + 1, 1.0))
        .collect::<Vec<_>>();
    let interactions = rack_view_interactions(
        72,
        24,
        DspRackViewState {
            track_name: "Track 01",
            track_number: 1,
            track_effects: &track_effects,
            master_effects: &[],
            selected_target: DspRackTargetView::Track,
            selected_index: 18,
            selected_parameter_index: 0,
            selected_lock_status: DspParameterLockStatusView::Unlocked,
            device_palette: None,
        },
    );
    let rows = interactions
        .regions()
        .iter()
        .filter(|region| {
            matches!(
                region.payload,
                InteractionPayload::DspDeviceRow {
                    target: DspRackChain::Track,
                    ..
                }
            )
        })
        .collect::<Vec<_>>();

    assert!(matches!(
        rows.first().map(|region| region.payload),
        Some(InteractionPayload::DspDeviceRow { index, .. }) if index > 0
    ));
    assert!(rows.iter().any(|region| {
        region.payload
            == InteractionPayload::DspDeviceRow {
                target: DspRackChain::Track,
                index: 18,
            }
    }));
}

#[test]
fn dsp_parameter_rows_scroll_to_the_selected_absolute_index() {
    let effects = [EffectDevice::delay(1, DelaySpec::default())];
    let interactions = rack_view_interactions(
        72,
        16,
        DspRackViewState {
            track_name: "Track 01",
            track_number: 1,
            track_effects: &effects,
            master_effects: &[],
            selected_target: DspRackTargetView::Track,
            selected_index: 0,
            selected_parameter_index: 5,
            selected_lock_status: DspParameterLockStatusView::Unlocked,
            device_palette: None,
        },
    );
    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::DSP_PARAMETER_ROW)
        .collect::<Vec<_>>();

    assert!(matches!(
        rows.first().map(|region| region.payload),
        Some(InteractionPayload::DspParameterRow { index }) if index > 0
    ));
    assert!(rows
        .iter()
        .any(|region| { region.payload == InteractionPayload::DspParameterRow { index: 5 } }));
}

#[test]
fn empty_chain_and_non_row_geometry_have_no_device_payload() {
    let master_effects = [EffectDevice::gain(1, 1.0)];
    let interactions = rack_interactions(80, 24, &[], &master_effects);
    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::DSP_DEVICE_ROW)
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].payload,
        InteractionPayload::DspDeviceRow {
            target: DspRackChain::Master,
            index: 0,
        }
    );
    assert!(interactions.regions().iter().all(|region| !matches!(
        region.payload,
        InteractionPayload::DspDeviceRow {
            target: DspRackChain::Track,
            ..
        }
    )));
    assert_eq!(
        interactions
            .hit_test(rows[0].area.x, rows[0].area.y.saturating_sub(1))
            .map(|region| region.id),
        Some(interaction_region::DSP_CHAIN)
    );
    assert!(interactions
        .regions()
        .iter()
        .all(|region| region.id != interaction_region::DSP_PARAMETER_ROW));
}
