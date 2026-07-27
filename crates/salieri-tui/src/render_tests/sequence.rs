use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::{NoteEvent, Song};

use super::render_test_support::*;

#[test]
fn sequence_panel_scrolls_to_active_position() {
    let song = long_sequence_song(40);
    let backend = TestBackend::new(32, 8);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_sequence(
                frame,
                Rect::new(0, 0, 32, 8),
                &song,
                Some(30),
                &mut interactions,
            );
        })
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);

    assert!(rendered.contains("Song Slots 28-33 / 40"));
    assert!(rendered.contains("> 30 P31"));
    assert!(rendered.contains("Pattern 31"));
    assert!(!rendered.contains(" 00 Pattern 01"));
    assert_eq!(
        interactions.scroll_target_at(1, 1),
        Some(crate::ScrollTarget::Sequence)
    );
    assert_eq!(interactions.scroll_target_at(1, 0), None);
    assert_eq!(interactions.scroll_target_at(1, 7), None);
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
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_sequence_editor(
                frame,
                Rect::new(0, 0, 80, 10),
                &song,
                Some(1),
                &mut interactions,
            );
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

#[test]
fn sequence_editor_rows_expose_scrolled_absolute_positions_only_for_content_rows() {
    let song = long_sequence_song(40);
    let backend = TestBackend::new(48, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_sequence_editor(
                frame,
                Rect::new(0, 0, 48, 10),
                &song,
                Some(30),
                &mut interactions,
            );
        })
        .expect("draw");

    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::SEQUENCE_EDITOR_ROW)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().any(|region| {
        region.payload == crate::InteractionPayload::SequenceEditorRow { position: 30 }
    }));
    assert!(rows.windows(2).all(|pair| {
        matches!(
            (pair[0].payload, pair[1].payload),
            (
                crate::InteractionPayload::SequenceEditorRow {
                    position: previous
                },
                crate::InteractionPayload::SequenceEditorRow { position: next },
            ) if next == previous + 1
        ) && pair[1].area.y == pair[0].area.y + 1
    }));
    assert!(interactions.hit_test(1, 0).is_none());
    assert!(interactions.hit_test(1, 1).is_none());
    assert!(interactions.hit_test(1, 5).is_none());
    assert_eq!(
        interactions.scroll_target_at(rows[0].area.x, rows[0].area.y),
        Some(crate::ScrollTarget::Sequence)
    );
    assert_eq!(interactions.scroll_target_at(1, 1), None);
    assert_eq!(interactions.scroll_target_at(1, 5), None);
}

#[test]
fn narrow_sequence_editor_keeps_rendered_rows_aligned_with_targets() {
    let song = long_sequence_song(3);
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            render_sequence_editor(
                frame,
                Rect::new(0, 0, 20, 10),
                &song,
                Some(0),
                &mut interactions,
            );
        })
        .expect("draw");

    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::SEQUENCE_EDITOR_ROW)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    for (expected_position, region) in rows.into_iter().enumerate() {
        let rendered_row = (0..20)
            .map(|x| {
                terminal.backend().buffer()[(x, region.area.y)]
                    .symbol()
                    .to_string()
            })
            .collect::<String>();
        assert!(
            rendered_row.contains(&format!("{expected_position:02}")),
            "row {} should render sequence position {expected_position}: {rendered_row:?}",
            region.area.y
        );
    }
}
