use super::render_test_support::render_test_state;
use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;

#[test]
fn pattern_grid_scroll_target_excludes_header_and_border_rows() {
    let song = Song::empty();
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_pattern_with_interactions(
                frame,
                Rect::new(0, 0, 60, 10),
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
}
