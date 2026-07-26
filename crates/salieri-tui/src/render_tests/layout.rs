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
    interaction_map_with_state(width, height, render_test_state())
}

fn interaction_map_with_state(width: u16, height: u16, state: TuiState<'_>) -> InteractionMap {
    let song = Song::empty();
    interaction_map_with_song_and_state(width, height, &song, state)
}

fn interaction_map_with_song_and_state(
    width: u16,
    height: u16,
    song: &Song,
    state: TuiState<'_>,
) -> InteractionMap {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut map = InteractionMap::new();
    terminal
        .draw(|frame| {
            map = render_with_interactions(frame, song, state);
        })
        .expect("draw");
    map
}

#[test]
fn exposes_offset_pattern_cells_at_representative_sizes() {
    let mut song = Song::empty();
    while song.tracks.len() < 8 {
        song.create_track();
    }
    let mut state = render_test_state();
    state.cursor.row = 5;
    state.cursor.track = 1;
    state.row_offset = 5;
    state.track_offset = 1;

    for (width, height, first_cell) in [
        (72, 24, (6_u16, 5_u16)),
        (100, 28, (34_u16, 5_u16)),
        (140, 36, (21_u16, 10_u16)),
    ] {
        let map = interaction_map_with_song_and_state(width, height, &song, state);
        let region = map
            .hit_test(first_cell.0, first_cell.1)
            .expect("first visible pattern cell");

        assert_eq!(region.id, interaction_region::PATTERN_CELL);
        assert_eq!(
            region.payload,
            crate::InteractionPayload::PatternCell { row: 5, track: 1 }
        );
        assert_ne!(
            map.hit_test(first_cell.0, first_cell.1.saturating_sub(1))
                .map(|region| region.id),
            Some(interaction_region::PATTERN_CELL)
        );
    }
}

#[test]
fn overlay_regions_override_covered_workspace_regions() {
    let mut help_state = render_test_state();
    help_state.show_help = true;
    let help = interaction_map_with_state(100, 28, help_state);
    let help_area = help
        .region(interaction_region::OVERLAY_HELP)
        .expect("help overlay")
        .area;
    assert_eq!(
        help.hit_test(help_area.x, help_area.y)
            .map(|region| region.id),
        Some(interaction_region::OVERLAY_HELP)
    );

    let mut confirmation_state = render_test_state();
    confirmation_state.quit_confirmation = true;
    let confirmation = interaction_map_with_state(100, 28, confirmation_state);
    let confirmation_area = confirmation
        .region(interaction_region::OVERLAY_QUIT_CONFIRMATION)
        .expect("quit confirmation")
        .area;
    assert_eq!(
        confirmation
            .hit_test(confirmation_area.x, confirmation_area.y)
            .map(|region| region.id),
        Some(interaction_region::OVERLAY_QUIT_CONFIRMATION)
    );
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
fn exposes_large_renoise_workspace_panel_regions() {
    let large = interaction_map(140, 36);

    let expected = [
        (interaction_region::PANEL_ANALYZER, Rect::new(0, 3, 140, 4)),
        (interaction_region::PANEL_UTIL, Rect::new(0, 7, 15, 21)),
        (interaction_region::PANEL_PATTERN, Rect::new(15, 7, 87, 21)),
        (
            interaction_region::PANEL_INSPECTOR,
            Rect::new(102, 7, 38, 21),
        ),
        (interaction_region::PANEL_EFFECTS, Rect::new(0, 28, 34, 7)),
        (interaction_region::PANEL_MIXER, Rect::new(34, 28, 39, 7)),
        (interaction_region::PANEL_VU, Rect::new(73, 28, 28, 7)),
        (
            interaction_region::PANEL_DEVICE_CHAIN,
            Rect::new(101, 28, 39, 7),
        ),
    ];

    for (id, area) in expected {
        assert_eq!(large.region(id).map(|region| region.area), Some(area));
        assert_eq!(
            large.hit_test(area.x, area.y).map(|region| region.id),
            Some(id)
        );
    }
}
