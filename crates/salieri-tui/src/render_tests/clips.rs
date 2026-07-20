use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::{NoteEvent, Song};

use super::render_test_support::*;

#[test]
fn clip_launcher_renders_scene_grid_statuses() {
    let mut song = Song::empty();
    song.patterns[0]
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("note");
    song.create_clip_scene_from_pattern("Intro", 0)
        .expect("scene");
    song.tracks[1].muted = true;

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render_clip_launcher(
                frame,
                Rect::new(0, 0, 80, 10),
                &song,
                TuiState {
                    active_view: TuiView::Clips,
                    pattern_index: 0,
                    cursor: Cursor {
                        track: 0,
                        ..Cursor::new()
                    },
                    is_playing: true,
                    sequence_position: None,
                    ..render_test_state()
                },
            );
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("Clip Launcher"));
    assert!(rendered.contains("?00 Intro"));
    assert!(rendered.contains("[Q]"));
    assert!(rendered.contains(" M "));
    assert!(rendered.contains(" · "));
    assert!(rendered.contains("States: ■ stopped  A active  Q queued"));
}
