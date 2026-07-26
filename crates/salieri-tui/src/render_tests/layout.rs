use super::render_test_support::render_test_state;
use super::*;
use crate::{interaction_region, InteractionMap};
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn classifies_responsive_layout_breakpoints() {
    assert_eq!(layout_kind(79), LayoutKind::Small);
    assert_eq!(layout_kind(80), LayoutKind::Medium);
    assert_eq!(layout_kind(119), LayoutKind::Medium);
    assert_eq!(layout_kind(120), LayoutKind::Large);
}

fn interaction_map(width: u16, height: u16) -> InteractionMap {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let song = Song::empty();
    let mut map = InteractionMap::new();
    terminal
        .draw(|frame| {
            map = render_with_interactions(frame, &song, render_test_state());
        })
        .expect("draw");
    map
}

#[test]
fn exposes_top_level_regions_at_representative_sizes() {
    for (width, height) in [(72, 24), (100, 28), (140, 36)] {
        let map = interaction_map(width, height);

        assert_eq!(
            map.region(interaction_region::APP_HEADER)
                .map(|region| region.area),
            Some(Rect::new(0, 0, width, 3))
        );
        assert_eq!(
            map.region(interaction_region::APP_BODY)
                .map(|region| region.area),
            Some(Rect::new(0, 3, width, height - 4))
        );
        assert_eq!(
            map.region(interaction_region::APP_STATUS)
                .map(|region| region.area),
            Some(Rect::new(0, height - 1, width, 1))
        );
        assert_eq!(
            map.region(interaction_region::VIEW_PATTERN)
                .map(|region| region.area),
            Some(Rect::new(0, 3, width, height - 4))
        );
    }
}

#[test]
fn exposes_resolved_panels_for_small_and_medium_pattern_layouts() {
    let small = interaction_map(72, 24);
    assert_eq!(
        small
            .region(interaction_region::PANEL_PATTERN)
            .map(|region| region.area),
        Some(Rect::new(0, 3, 72, 20))
    );
    assert!(small.region(interaction_region::PANEL_TRACKS).is_none());

    let medium = interaction_map(100, 28);
    let pattern = medium
        .region(interaction_region::PANEL_PATTERN)
        .expect("pattern panel");
    let tracks = medium
        .region(interaction_region::PANEL_TRACKS)
        .expect("tracks panel");
    let sequence = medium
        .region(interaction_region::PANEL_SEQUENCE)
        .expect("sequence panel");
    let track_desk = medium
        .region(interaction_region::PANEL_TRACK_DESK)
        .expect("track desk panel");

    assert_eq!(tracks.area, Rect::new(0, 3, 28, 14));
    assert_eq!(sequence.area, Rect::new(0, 17, 28, 10));
    assert_eq!(pattern.area, Rect::new(28, 3, 72, 14));
    assert_eq!(track_desk.area, Rect::new(28, 17, 72, 10));
}

#[test]
fn large_pattern_workspace_uses_view_region_until_subregions_migrate() {
    let large = interaction_map(140, 36);

    assert!(large.region(interaction_region::PANEL_PATTERN).is_none());
    assert_eq!(
        large.hit_test(70, 10).map(|region| region.id),
        Some(interaction_region::VIEW_PATTERN)
    );
}
