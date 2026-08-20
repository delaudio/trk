use trk_core::{Cursor, NoteEvent, Song};
use trk_tui::{TuiState, TuiView};

#[allow(dead_code)]
mod support;
use support::{assert_snapshot, render_snapshot, test_state};

#[test]
fn snapshots_piano_roll_with_gate_and_ghost_notes() {
    let mut song = Song::empty();
    song.patterns[0]
        .set_note(4, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("active note");
    song.patterns[0].set_gate(4, 0, Some(4)).expect("gate");
    song.patterns[0]
        .set_note(6, 1, NoteEvent::Note { pitch: 64 }, 80)
        .expect("ghost note");

    assert_snapshot(
        "piano-roll",
        render_snapshot(
            song,
            TuiState {
                cursor: Cursor {
                    row: 4,
                    track: 0,
                    ..Cursor::new()
                },
                active_view: TuiView::PianoRoll {
                    pitch: 60,
                    rows: 16,
                    ghosts: true,
                },
                mode_label: "ROLL",
                playhead_row: Some(6),
                ..test_state()
            },
            100,
            28,
        ),
    );
}
