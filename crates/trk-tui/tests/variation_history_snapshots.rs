use trk_core::Song;
use trk_tui::{PatternVariationEntryView, PatternVariationHistoryViewState, TuiState};

#[allow(dead_code)]
mod support;
use support::{assert_snapshot, render_snapshot, test_state};

#[test]
fn snapshots_pattern_variation_history_overlay() {
    let entries = [
        PatternVariationEntryView {
            id: 7,
            timestamp: 1_777_000_000,
            description: "AI bass variation",
            source: "AI",
            pattern_index: 0,
            track_index: Some(0),
            active: false,
        },
        PatternVariationEntryView {
            id: 8,
            timestamp: 1_777_000_120,
            description: "Euclidean 5/16 rotation 2",
            source: "Euclidean",
            pattern_index: 0,
            track_index: Some(1),
            active: true,
        },
    ];

    assert_snapshot(
        "pattern-variation-history",
        render_snapshot(
            Song::empty(),
            TuiState {
                variation_history: Some(PatternVariationHistoryViewState {
                    entries: &entries,
                    selected: 1,
                }),
                ..test_state()
            },
            100,
            28,
        ),
    );
}
