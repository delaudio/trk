use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::{NoteEvent, Song};

use super::render_test_support::*;

#[test]
fn sequence_panel_scrolls_to_active_position() {
    let song = long_sequence_song(40);
    let backend = TestBackend::new(32, 8);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render_sequence(frame, Rect::new(0, 0, 32, 8), &song, Some(30));
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("Song Slots 28-33 / 40"));
    assert!(rendered.contains("> 30 P31"));
    assert!(rendered.contains("Pattern 31"));
    assert!(!rendered.contains(" 00 Pattern 01"));
}

#[test]
fn song_slot_view_renders_selected_muted_empty_and_active_track_clips() {
    let mut song = Song::empty();
    let second = song.create_pattern(64);
    song.push_sequence_pattern(second).expect("sequence");
    song.tracks[1].muted = true;
    song.patterns[0]
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("track one note");
    song.patterns[1]
        .set_note(0, 2, NoteEvent::Note { pitch: 67 }, 100)
        .expect("track three note");
    song.rename_pattern(0, "Verse".to_string()).expect("rename");
    song.rename_pattern(1, "Answer".to_string())
        .expect("rename");

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    terminal
        .draw(|frame| {
            render_sequence_editor(frame, Rect::new(0, 0, 80, 10), &song, Some(1));
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("Song Slot View"));
    assert!(rendered.contains("P01 Verse"));
    assert!(rendered.contains("[■ M · ·]"));
    assert!(rendered.contains(">01"));
    assert!(rendered.contains("P02 Answer"));
    assert!(rendered.contains("[· M ■ ·]"));
    assert!(rendered.contains("Clips: ■ active  · empty  M muted"));
}
