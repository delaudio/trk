use super::render_test_support::{long_sequence_song, long_track_song, render_test_state};
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
fn transport_symbols_expose_distinct_play_and_stop_targets_at_supported_widths() {
    for width in [16, 72, 100, 140] {
        let map = interaction_map(width, 24);
        let play = map
            .regions()
            .iter()
            .find(|region| {
                region.payload
                    == crate::InteractionPayload::TransportAction {
                        action: crate::TransportAction::Play,
                    }
            })
            .expect("visible Play target");
        let stop = map
            .regions()
            .iter()
            .find(|region| {
                region.payload
                    == crate::InteractionPayload::TransportAction {
                        action: crate::TransportAction::Stop,
                    }
            })
            .expect("visible Stop target");

        assert_eq!(play.area, ratatui::layout::Rect::new(3, 1, 1, 1));
        assert_eq!(stop.area, ratatui::layout::Rect::new(7, 1, 1, 1));
        assert_eq!(
            map.hit_test(3, 1).map(|region| region.payload),
            Some(crate::InteractionPayload::TransportAction {
                action: crate::TransportAction::Play,
            })
        );
        assert_eq!(
            map.hit_test(7, 1).map(|region| region.payload),
            Some(crate::InteractionPayload::TransportAction {
                action: crate::TransportAction::Stop,
            })
        );
        assert_ne!(
            map.hit_test(11, 1).map(|region| region.id),
            Some(interaction_region::TRANSPORT_ACTION),
            "Record must not be a transport action at width {width}"
        );
    }
}

#[test]
fn transport_header_uses_complete_width_appropriate_segments() {
    let song = Song::empty();
    let state = render_test_state();
    let cases = [
        (
            72,
            " [▷] [■] [●×] BPM:120 LPB:4 STOPPED PAT:01 ROW:0000/0000",
        ),
        (
            80,
            " [▷] [■] [●×] BPM:120 LPB:4 STOPPED PAT:01 ROW:0000/0000  Sync: Internal",
        ),
        (
            100,
            " [▷] [■] [●×] BPM:120 LPB:4 STOPPED PAT:01 ROW:0000/0000  Sync: Internal",
        ),
        (
            140,
            " [▷] [■] [●×] BPM:120 LPB:4 Oct:4 V:100 Sw:0% Syn:Int CPU0% STOPPED PAT:01 ROW:0000/0000 MIDI Disconnected ORD:00 LOOP:ON TRK:01 FLD:NOTE",
        ),
    ];

    for (terminal_width, expected) in cases {
        let available_width = terminal_width - 2;
        let header = compose_transport_header(&song, state, available_width);
        let actual = super::render_test_support::line_text(&header.line);

        assert_eq!(actual, expected, "terminal width {terminal_width}");
        assert!(
            header.line.width() <= usize::from(available_width),
            "terminal width {terminal_width}: {actual}"
        );
    }

    let loop_off = compose_transport_header(
        &song,
        TuiState {
            loop_pattern: false,
            ..state
        },
        138,
    );
    let loop_off_text = super::render_test_support::line_text(&loop_off.line);
    assert_eq!(loop_off.line.width(), 138);
    assert!(loop_off_text.contains("MIDI Disconnected"));
    assert!(loop_off_text.contains("LOOP:OFF"));
}

#[test]
fn optional_header_markers_are_omitted_atomically_when_they_do_not_fit() {
    let song = Song::empty();
    let state = TuiState {
        selection: Some(SelectionRect {
            row_start: 0,
            row_end: 1,
            track_start: 0,
            track_end: 1,
        }),
        dirty: true,
        ..render_test_state()
    };

    let constrained = compose_transport_header(&song, state, 137);
    let constrained_text = super::render_test_support::line_text(&constrained.line);
    assert_eq!(constrained.line.width(), 137);
    assert!(!constrained_text.contains(" SEL"));
    assert!(!constrained_text.ends_with(" *"));

    let wide = compose_transport_header(&song, state, 143);
    let wide_text = super::render_test_support::line_text(&wide.line);
    assert_eq!(wide.line.width(), 143);
    assert!(wide_text.ends_with(" SEL *"));
}

#[test]
fn measured_header_candidates_never_exceed_dynamic_boundaries() {
    let song = Song::empty();
    let state = TuiState {
        cursor: Cursor {
            row: 12_345,
            track: 123,
            field: CellField::Instrument,
            digit: 0,
        },
        pattern_index: 123,
        playhead_row: Some(54_321),
        midi_status: "MIDI Disconnected | MIDI In Rec+Clock",
        sequence_position: Some(123),
        ..render_test_state()
    };

    for available_width in [0, 1, 7, 8, 13, 14, 60, 61, 70, 78, 98, 118, 132, 138] {
        let header = compose_transport_header(&song, state, available_width);
        let text = super::render_test_support::line_text(&header.line);
        assert!(
            header.line.width() <= usize::from(available_width),
            "available width {available_width}: {}",
            text
        );
        if matches!(available_width, 70 | 78) {
            for required in ["BPM:", "LPB:", "STOPPED", "PAT:>4", "ROW:>321/>345"] {
                assert!(text.contains(required), "missing {required}: {text}");
            }
        }
    }
}

#[test]
fn variable_midi_status_is_rendered_whole_or_omitted() {
    let song = Song::empty();
    for status in [
        "MIDI Disconnected",
        "MIDI Disconnected | MIDI In Rec",
        "MIDI Connecting 2",
        "MIDI No Outputs",
        "MIDI Error: device unavailable",
    ] {
        let state = TuiState {
            midi_status: status,
            ..render_test_state()
        };
        for available_width in [98, 118, 120, 138, 158] {
            let header = compose_transport_header(&song, state, available_width);
            let text = super::render_test_support::line_text(&header.line);
            assert!(header.line.width() <= usize::from(available_width));
            assert!(
                !text.contains("MIDI:On") && !text.contains("MIDI:Off") && !text.contains("MIDI:—")
            );
            if text.contains("MIDI") {
                assert!(
                    text.contains(status),
                    "partial or inferred MIDI status at width {available_width}: {text}"
                );
            }
            if available_width < 138 {
                assert!(!text.contains(status));
            }
            if available_width >= 138 {
                for static_segment in [" ORD:", " LOOP:", " TRK:", " FLD:"] {
                    assert!(
                        text.contains(static_segment),
                        "missing {static_segment} at width {available_width}: {text}"
                    );
                }
            }
        }
    }
}

#[test]
fn full_pattern_cells_fill_the_registered_interaction_width() {
    let spans = cell_spans(
        &PatternCell::default(),
        CellField::Note,
        false,
        false,
        false,
        false,
        PatternFieldLayout::Full,
    );
    let rendered_width = spans.iter().map(Span::width).sum::<usize>();

    assert_eq!(rendered_width, pattern_cell_width(PatternFieldLayout::Full));
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
        let second_track = map
            .regions()
            .iter()
            .find(|region| {
                region.payload == crate::InteractionPayload::PatternCell { row: 5, track: 2 }
            })
            .expect("second visible track cell");
        assert_eq!(
            map.hit_test(second_track.area.x, second_track.area.y)
                .map(|region| region.payload),
            Some(crate::InteractionPayload::PatternCell { row: 5, track: 2 })
        );
        assert_ne!(
            map.hit_test(first_cell.0, first_cell.1.saturating_sub(1))
                .map(|region| region.id),
            Some(interaction_region::PATTERN_CELL)
        );
    }
}

#[test]
fn composite_track_rows_expose_scrolled_absolute_indices() {
    let song = long_track_song(20);
    let mut state = render_test_state();
    state.cursor.track = 15;

    let map = interaction_map_with_song_and_state(100, 28, &song, state);
    let regions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::COMPOSITE_TRACK_ROW)
        .collect::<Vec<_>>();
    let first = regions.first().expect("first visible track");
    let last = regions.last().expect("last visible track");

    assert!(matches!(
        first.payload,
        crate::InteractionPayload::CompositeTrackRow { track } if track > 0
    ));
    assert_eq!(
        last.payload,
        crate::InteractionPayload::CompositeTrackRow { track: 19 }
    );
    assert!(regions.iter().any(|region| {
        region.payload == crate::InteractionPayload::CompositeTrackRow { track: 15 }
    }));
    assert!(regions.windows(2).all(|pair| {
        pair[1].area.y == pair[0].area.y.saturating_add(1)
            && matches!(
                (pair[0].payload, pair[1].payload),
                (
                    crate::InteractionPayload::CompositeTrackRow { track: previous },
                    crate::InteractionPayload::CompositeTrackRow { track: next },
                ) if next == previous + 1
            )
    }));
    assert_eq!(
        map.hit_test(first.area.x, first.area.y)
            .map(|region| region.payload),
        Some(first.payload)
    );
}

#[test]
fn composite_track_borders_and_empty_rows_are_not_track_targets() {
    let song = long_track_song(2);
    let map = interaction_map_with_song_and_state(100, 28, &song, render_test_state());
    let panel = map
        .regions()
        .iter()
        .find(|region| region.id == interaction_region::PANEL_TRACKS)
        .expect("tracks panel");
    let rows = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::COMPOSITE_TRACK_ROW)
        .collect::<Vec<_>>();
    let last = rows.last().expect("last track row");

    assert_ne!(
        map.hit_test(panel.area.x, last.area.y)
            .map(|region| region.id),
        Some(interaction_region::COMPOSITE_TRACK_ROW)
    );
    assert_ne!(
        map.hit_test(last.area.x, last.area.y.saturating_add(1))
            .map(|region| region.id),
        Some(interaction_region::COMPOSITE_TRACK_ROW)
    );
}

#[test]
fn composite_sequence_rows_expose_scrolled_absolute_positions() {
    let song = long_sequence_song(40);
    let mut state = render_test_state();
    state.sequence_position = Some(30);

    let map = interaction_map_with_song_and_state(100, 28, &song, state);
    let regions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::COMPOSITE_SEQUENCE_ROW)
        .collect::<Vec<_>>();
    let first = regions.first().expect("first visible song slot");

    assert!(matches!(
        first.payload,
        crate::InteractionPayload::CompositeSequenceRow { position } if position > 0
    ));
    assert!(regions.iter().any(|region| {
        region.payload == crate::InteractionPayload::CompositeSequenceRow { position: 30 }
    }));
    assert!(regions.windows(2).all(|pair| {
        pair[1].area.y == pair[0].area.y.saturating_add(1)
            && matches!(
                (pair[0].payload, pair[1].payload),
                (
                    crate::InteractionPayload::CompositeSequenceRow { position: previous },
                    crate::InteractionPayload::CompositeSequenceRow { position: next },
                ) if next == previous + 1
            )
    }));
    assert_eq!(
        map.hit_test(first.area.x, first.area.y)
            .map(|region| region.payload),
        Some(first.payload)
    );
}

#[test]
fn composite_sequence_borders_and_empty_rows_are_not_slot_targets() {
    let song = long_sequence_song(2);
    let map = interaction_map_with_song_and_state(100, 28, &song, render_test_state());
    let panel = map
        .regions()
        .iter()
        .find(|region| region.id == interaction_region::PANEL_SEQUENCE)
        .expect("song slots panel");
    let rows = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::COMPOSITE_SEQUENCE_ROW)
        .collect::<Vec<_>>();
    let last = rows.last().expect("last song slot row");

    assert_ne!(
        map.hit_test(panel.area.x, last.area.y)
            .map(|region| region.id),
        Some(interaction_region::COMPOSITE_SEQUENCE_ROW)
    );
    assert_ne!(
        map.hit_test(last.area.x, last.area.y.saturating_add(1))
            .map(|region| region.id),
        Some(interaction_region::COMPOSITE_SEQUENCE_ROW)
    );
}

#[test]
fn pattern_manager_rows_expose_scrolled_absolute_indices() {
    let song = long_sequence_song(40);
    let mut state = render_test_state();
    state.active_view = TuiView::Patterns;
    state.pattern_index = 30;

    let map = interaction_map_with_song_and_state(100, 28, &song, state);
    let regions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::PATTERN_MANAGER_ROW)
        .collect::<Vec<_>>();
    let first = regions.first().expect("first visible pattern");

    assert!(matches!(
        first.payload,
        crate::InteractionPayload::PatternManagerRow { index } if index > 0
    ));
    assert!(regions.iter().any(|region| {
        region.payload == crate::InteractionPayload::PatternManagerRow { index: 30 }
    }));
    assert!(regions.windows(2).all(|pair| {
        pair[1].area.y == pair[0].area.y.saturating_add(1)
            && matches!(
                (pair[0].payload, pair[1].payload),
                (
                    crate::InteractionPayload::PatternManagerRow { index: previous },
                    crate::InteractionPayload::PatternManagerRow { index: next },
                ) if next == previous + 1
            )
    }));
}

#[test]
fn pattern_manager_headers_borders_and_empty_rows_are_not_pattern_targets() {
    let mut state = render_test_state();
    state.active_view = TuiView::Patterns;
    let map = interaction_map_with_state(100, 28, state);
    let row = map
        .regions()
        .iter()
        .find(|region| region.id == interaction_region::PATTERN_MANAGER_ROW)
        .expect("pattern row");

    assert_ne!(
        map.hit_test(row.area.x.saturating_sub(1), row.area.y)
            .map(|region| region.id),
        Some(interaction_region::PATTERN_MANAGER_ROW)
    );
    assert_ne!(
        map.hit_test(row.area.x, row.area.y.saturating_sub(1))
            .map(|region| region.id),
        Some(interaction_region::PATTERN_MANAGER_ROW)
    );
    assert_ne!(
        map.hit_test(row.area.x, row.area.y.saturating_add(1))
            .map(|region| region.id),
        Some(interaction_region::PATTERN_MANAGER_ROW)
    );
}

#[test]
fn sample_browser_entry_regions_preserve_nonzero_viewport_offsets() {
    let entries = vec![
        SampleBrowserEntryView {
            name: "a-very-long-sample-name-that-must-stay-on-one-row.wav",
            kind: SampleBrowserEntryKind::SupportedSample,
        };
        40
    ];
    let mut state = render_test_state();
    state.active_view = TuiView::SampleBrowser;
    state.sample_browser = Some(SampleBrowserViewState {
        current_dir: "/tmp/samples",
        entries: &entries,
        selected: 30,
        preview: None,
        message: None,
    });

    let map = interaction_map_with_state(72, 24, state);
    let regions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::SAMPLE_BROWSER_ENTRY)
        .collect::<Vec<_>>();
    let first = regions.first().expect("first visible sample");
    let last = regions.last().expect("last visible sample");

    assert!(matches!(
        first.payload,
        crate::InteractionPayload::SampleBrowserEntry { index } if index > 0
    ));
    assert_eq!(
        last.payload,
        crate::InteractionPayload::SampleBrowserEntry { index: 30 }
    );
    assert_eq!(
        map.hit_test(first.area.x, first.area.y)
            .map(|region| region.payload),
        Some(first.payload)
    );
    assert_ne!(
        map.hit_test(first.area.x.saturating_sub(1), first.area.y)
            .map(|region| region.id),
        Some(interaction_region::SAMPLE_BROWSER_ENTRY)
    );
    assert!(regions
        .windows(2)
        .all(|pair| pair[1].area.y == pair[0].area.y.saturating_add(1)));
}

#[test]
fn project_browser_entry_regions_preserve_nonzero_viewport_offsets() {
    let entries = vec![
        ProjectBrowserEntryView {
            name: "a-very-long-project-name-that-must-stay-on-one-row.salieri",
            path: "/tmp/project.salieri",
            kind: ProjectBrowserEntryKind::Project,
            detail: "project",
        };
        40
    ];
    let mut state = render_test_state();
    state.active_view = TuiView::ProjectBrowser;
    state.project_browser = Some(ProjectBrowserViewState {
        current_dir: "/tmp/projects",
        entries: &entries,
        selected: 30,
        message: None,
    });

    let map = interaction_map_with_state(72, 24, state);
    let regions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::PROJECT_BROWSER_ENTRY)
        .collect::<Vec<_>>();
    let first = regions.first().expect("first visible project");
    let last = regions.last().expect("last visible project");

    assert!(matches!(
        first.payload,
        crate::InteractionPayload::ProjectBrowserEntry { index } if index > 0
    ));
    assert_eq!(
        last.payload,
        crate::InteractionPayload::ProjectBrowserEntry { index: 30 }
    );
    assert_eq!(
        map.hit_test(first.area.x, first.area.y)
            .map(|region| region.payload),
        Some(first.payload)
    );
    assert_ne!(
        map.hit_test(first.area.x.saturating_sub(1), first.area.y)
            .map(|region| region.id),
        Some(interaction_region::PROJECT_BROWSER_ENTRY)
    );
    assert!(regions
        .windows(2)
        .all(|pair| pair[1].area.y == pair[0].area.y.saturating_add(1)));
}

#[test]
fn grouped_project_browser_section_headers_are_not_entry_regions() {
    let entries = [
        ProjectBrowserEntryView {
            name: "Samples",
            path: "/tmp/renoise-demos/Samples",
            kind: ProjectBrowserEntryKind::Directory,
            detail: "samples",
        },
        ProjectBrowserEntryView {
            name: "DemoSong - Example.xrns",
            path: "/tmp/renoise-demos/example.xrns",
            kind: ProjectBrowserEntryKind::Project,
            detail: "song",
        },
    ];
    let mut state = render_test_state();
    state.active_view = TuiView::ProjectBrowser;
    state.project_browser = Some(ProjectBrowserViewState {
        current_dir: "/tmp/renoise-demos",
        entries: &entries,
        selected: 0,
        message: None,
    });

    let map = interaction_map_with_state(100, 28, state);
    let first_entry = map
        .regions()
        .iter()
        .find(|region| region.id == interaction_region::PROJECT_BROWSER_ENTRY)
        .expect("grouped project entry");

    assert_ne!(
        map.hit_test(first_entry.area.x, first_entry.area.y.saturating_sub(1))
            .map(|region| region.id),
        Some(interaction_region::PROJECT_BROWSER_ENTRY)
    );
}

#[test]
fn grouped_project_browser_scrolls_entry_regions_to_selected_item() {
    let entries = vec![
        ProjectBrowserEntryView {
            name: "DemoSong - A very long project name that must stay on one row.xrns",
            path: "/tmp/renoise-demos/example.xrns",
            kind: ProjectBrowserEntryKind::Project,
            detail: "song",
        };
        40
    ];
    let mut state = render_test_state();
    state.active_view = TuiView::ProjectBrowser;
    state.project_browser = Some(ProjectBrowserViewState {
        current_dir: "/tmp/renoise-demos",
        entries: &entries,
        selected: 30,
        message: None,
    });

    let map = interaction_map_with_state(72, 24, state);
    let regions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::PROJECT_BROWSER_ENTRY)
        .collect::<Vec<_>>();
    let first = regions.first().expect("first grouped project entry");
    let last = regions.last().expect("selected grouped project entry");

    assert!(matches!(
        first.payload,
        crate::InteractionPayload::ProjectBrowserEntry { index } if index > 0
    ));
    assert_eq!(
        last.payload,
        crate::InteractionPayload::ProjectBrowserEntry { index: 30 }
    );
    assert!(regions
        .windows(2)
        .all(|pair| pair[1].area.y == pair[0].area.y.saturating_add(1)));
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
