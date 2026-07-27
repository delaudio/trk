use super::render_test_support::{long_sequence_song, terminal_buffer_text};
use super::*;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn pattern_manager_scrolls_to_active_pattern() {
    let song = long_sequence_song(40);
    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_pattern_manager(frame, Rect::new(0, 0, 48, 10), &song, 30, &mut interactions);
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("Pattern Manager 30-32 / 40"));
    assert!(rendered.contains(">31  Pattern 31"));
    assert!(!rendered.contains(" 01  Pattern 01"));
}

#[test]
fn narrow_pattern_manager_keeps_rendered_rows_aligned_with_targets() {
    let song = long_sequence_song(3);
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_pattern_manager(frame, Rect::new(0, 0, 20, 10), &song, 0, &mut interactions);
        })
        .expect("draw");

    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::PATTERN_MANAGER_ROW)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        interactions.scroll_target_at(rows[0].area.x, rows[0].area.y),
        Some(crate::ScrollTarget::Patterns)
    );
    assert_eq!(interactions.scroll_target_at(1, 0), None);
    assert_eq!(interactions.scroll_target_at(1, 1), None);
    assert_eq!(interactions.scroll_target_at(1, 6), None);
    for (expected_index, region) in rows.into_iter().enumerate() {
        let rendered_row = (0..20)
            .map(|x| {
                terminal.backend().buffer()[(x, region.area.y)]
                    .symbol()
                    .to_string()
            })
            .collect::<String>();
        assert!(
            rendered_row.contains(&format!("{:02}", expected_index + 1)),
            "row {} should render pattern {}: {rendered_row:?}",
            region.area.y,
            expected_index + 1
        );
    }
}
