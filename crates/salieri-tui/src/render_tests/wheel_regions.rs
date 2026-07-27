use super::render_test_support::render_test_state;
use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;

#[test]
fn pattern_grid_scroll_target_excludes_header_and_border_rows() {
    let song = Song::empty();
    let backend = TestBackend::new(160, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_pattern_with_interactions(
                frame,
                Rect::new(0, 0, 160, 10),
                &song,
                render_test_state(),
                &mut interactions,
            );
        })
        .expect("draw");

    let grid = interactions
        .region(interaction_region::PATTERN_GRID)
        .expect("pattern grid");
    assert_eq!(
        interactions.scroll_target_at(grid.area.x, grid.area.y),
        Some(crate::ScrollTarget::PatternRows)
    );
    assert_eq!(
        interactions.scroll_target_at(grid.area.x, grid.area.y.saturating_sub(1)),
        None
    );
    assert_eq!(
        interactions.scroll_target_at(grid.area.x, grid.area.y.saturating_add(grid.area.height)),
        None
    );
    assert!(grid.area.x.saturating_add(grid.area.width) < 159);
    assert_eq!(
        interactions.scroll_target_at(grid.area.x.saturating_add(grid.area.width), grid.area.y),
        None
    );
}

#[test]
fn large_workspace_pattern_gutter_is_a_bounded_scroll_target() {
    let song = Song::empty();
    let backend = TestBackend::new(140, 36);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            interactions = render_with_interactions(frame, &song, render_test_state());
        })
        .expect("draw");

    let grid = interactions
        .region(interaction_region::PATTERN_GRID)
        .expect("large pattern grid");
    assert_eq!(
        interactions.scroll_target_at(grid.area.x, grid.area.y),
        Some(crate::ScrollTarget::PatternRows)
    );
    assert_eq!(
        interactions.scroll_target_at(grid.area.x, grid.area.y.saturating_sub(1)),
        None
    );
    assert_eq!(
        interactions.scroll_target_at(grid.area.x.saturating_add(grid.area.width), grid.area.y),
        None
    );
}
