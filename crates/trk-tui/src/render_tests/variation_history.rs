use super::*;
use ratatui::{backend::TestBackend, Terminal};

use super::render_test_support::terminal_buffer_text;

#[test]
fn pattern_variation_modal_renders_metadata_active_badge_and_double_border() {
    let entries = [
        PatternVariationEntryView {
            id: 1,
            timestamp: 100,
            description: "AI bass variation",
            source: "AI",
            pattern_index: 0,
            track_index: Some(0),
            active: false,
        },
        PatternVariationEntryView {
            id: 2,
            timestamp: 200,
            description: "Euclidean 5/16 rotation 2",
            source: "Euclidean",
            pattern_index: 0,
            track_index: Some(1),
            active: true,
        },
    ];
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &Song::empty(),
                TuiState {
                    variation_history: Some(PatternVariationHistoryViewState {
                        entries: &entries,
                        selected: 1,
                    }),
                    ..super::render_test_support::render_test_state()
                },
            );
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);
    assert!(rendered.contains("Pattern Variation History"));
    assert!(rendered.contains("v001"));
    assert!(rendered.contains("v002"));
    assert!(rendered.contains("P01/T02"));
    assert!(rendered.contains("Euclidean"));
    assert!(rendered.contains("[ACTIVE]"));
    assert!(rendered.contains('╔'));
    assert!(rendered.contains('╝'));
}

#[test]
fn empty_pattern_variation_modal_has_an_actionable_empty_state() {
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &Song::empty(),
                TuiState {
                    variation_history: Some(PatternVariationHistoryViewState {
                        entries: &[],
                        selected: 0,
                    }),
                    ..super::render_test_support::render_test_state()
                },
            );
        })
        .expect("draw");

    assert!(terminal_buffer_text(&terminal).contains("No generated pattern variations yet"));
}
