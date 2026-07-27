use super::*;

fn large_mouse_viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 160,
        terminal_height: 40,
    }
}

#[test]
fn mouse_wheel_moves_tracker_cursor_through_shared_viewport() {
    let mut app = App::default();
    app.interaction_map.register(
        interaction_region::PATTERN_GRID,
        ratatui::layout::Rect::new(20, 3, 100, 30),
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 40,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );
    app.keep_cursor_visible(2);
    app.keep_track_visible(app.cursor.track, 4);

    assert_eq!(app.cursor.row, 3);
    assert_eq!(app.row_offset, 2);

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 40,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );
    app.keep_cursor_visible(2);
    app.keep_track_visible(app.cursor.track, 4);

    assert_eq!(app.cursor.row, 0);
    assert_eq!(app.row_offset, 0);
}

#[test]
fn mouse_click_moves_tracker_cursor_to_grid_cell() {
    let mut app = App::default();
    app.interaction_map.register_with_payload(
        interaction_region::PATTERN_CELL,
        ratatui::layout::Rect::new(45, 14, 12, 1),
        InteractionPayload::PatternCell { row: 4, track: 2 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 45,
            row: 14,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.cursor.row, 4);
    assert_eq!(app.cursor.track, 2);
}

#[test]
fn mouse_click_selects_composite_track_without_moving_the_cell_cursor() {
    let mut app = App::default();
    while app.song.tracks.len() < 6 {
        app.song.create_track();
    }
    app.cursor.row = 7;
    app.cursor.field = CellField::Effect;
    app.cursor.digit = 1;
    app.interaction_map.register_with_payload(
        interaction_region::COMPOSITE_TRACK_ROW,
        ratatui::layout::Rect::new(1, 8, 26, 1),
        InteractionPayload::CompositeTrackRow { track: 5 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);
    assert_eq!(app.cursor.track, 5);
    assert_eq!(app.cursor.row, 7);
    assert_eq!(app.cursor.field, CellField::Effect);
    assert_eq!(app.cursor.digit, 1);
}

#[test]
fn composite_track_click_rejects_out_of_range_payloads() {
    let mut app = App::default();
    app.cursor.row = 4;
    app.cursor.track = 0;
    app.interaction_map.register_with_payload(
        interaction_region::COMPOSITE_TRACK_ROW,
        ratatui::layout::Rect::new(1, 8, 26, 1),
        InteractionPayload::CompositeTrackRow { track: usize::MAX },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.cursor.row, 4);
    assert_eq!(app.cursor.track, 0);
}

#[test]
fn composite_track_rows_ignore_drag_and_secondary_clicks() {
    for kind in [
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
    ] {
        let mut app = App::default();
        while app.song.tracks.len() < 4 {
            app.song.create_track();
        }
        app.interaction_map.register_with_payload(
            interaction_region::COMPOSITE_TRACK_ROW,
            ratatui::layout::Rect::new(1, 8, 26, 1),
            InteractionPayload::CompositeTrackRow { track: 3 },
        );

        app.handle_mouse(
            MouseEvent {
                kind,
                column: 2,
                row: 8,
                modifiers: KeyModifiers::NONE,
            },
            large_mouse_viewport(),
        );

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.cursor.track, 0);
    }
}

#[test]
fn mouse_click_selects_composite_song_slot_without_starting_playback() {
    let mut app = App::default();
    let second = app.song.create_pattern(64);
    let third = app.song.create_pattern(64);
    app.song
        .push_sequence_pattern(second)
        .expect("second sequence slot");
    app.song
        .push_sequence_pattern(third)
        .expect("third sequence slot");
    app.pattern_index = 1;
    app.sequence_cursor = 0;
    app.interaction_map.register_with_payload(
        interaction_region::COMPOSITE_SEQUENCE_ROW,
        ratatui::layout::Rect::new(1, 16, 26, 1),
        InteractionPayload::CompositeSequenceRow { position: 2 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 16,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);
    assert_eq!(app.sequence_cursor, 2);
    assert_eq!(app.pattern_index, 1);
    assert!(!app.is_playing);
    assert_eq!(app.sequence_position, None);
    assert_eq!(
        app.notification
            .as_ref()
            .map(|value| value.message.as_str()),
        Some("Sequence position 02")
    );
}

#[test]
fn composite_song_slot_click_rejects_out_of_range_payloads() {
    let mut app = App::default();
    app.interaction_map.register_with_payload(
        interaction_region::COMPOSITE_SEQUENCE_ROW,
        ratatui::layout::Rect::new(1, 16, 26, 1),
        InteractionPayload::CompositeSequenceRow {
            position: usize::MAX,
        },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 16,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.sequence_cursor, 0);
    assert!(!app.is_playing);
    assert!(app.notification.is_none());
}

#[test]
fn composite_song_slot_rows_ignore_drag_and_secondary_clicks() {
    for kind in [
        MouseEventKind::Drag(MouseButton::Left),
        MouseEventKind::Down(MouseButton::Right),
    ] {
        let mut app = App::default();
        let second = app.song.create_pattern(64);
        app.song
            .push_sequence_pattern(second)
            .expect("second sequence slot");
        app.interaction_map.register_with_payload(
            interaction_region::COMPOSITE_SEQUENCE_ROW,
            ratatui::layout::Rect::new(1, 16, 26, 1),
            InteractionPayload::CompositeSequenceRow { position: 1 },
        );

        app.handle_mouse(
            MouseEvent {
                kind,
                column: 2,
                row: 16,
                modifiers: KeyModifiers::NONE,
            },
            large_mouse_viewport(),
        );

        assert_eq!(app.sequence_cursor, 0);
        assert!(!app.is_playing);
    }
}

#[test]
fn pattern_manager_mouse_selects_and_secondary_click_opens_tracker() {
    let mut app = App::default();
    app.song.create_pattern(64);
    app.song.create_pattern(64);
    app.open_patterns_view();
    app.interaction_map.register_with_payload(
        interaction_region::PATTERN_MANAGER_ROW,
        ratatui::layout::Rect::new(2, 8, 40, 1),
        InteractionPayload::PatternManagerRow { index: 2 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.pattern_index, 2);
    assert_eq!(app.mode, AppMode::Patterns);
    assert_eq!(app.tui_active_view(), TuiView::Patterns);

    app.interaction_map = InteractionMap::new();
    app.interaction_map.register_with_payload(
        interaction_region::PATTERN_MANAGER_ROW,
        ratatui::layout::Rect::new(2, 9, 40, 1),
        InteractionPayload::PatternManagerRow { index: 1 },
    );
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 3,
            row: 9,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.pattern_index, 1);
    assert_eq!(app.mode, AppMode::Normal);
    assert_eq!(app.tui_active_view(), TuiView::Pattern);
}

#[test]
fn pattern_manager_mouse_ignores_drag_and_invalid_payloads() {
    let mut app = App::default();
    app.song.create_pattern(64);
    app.open_patterns_view();
    app.interaction_map.register_with_payload(
        interaction_region::PATTERN_MANAGER_ROW,
        ratatui::layout::Rect::new(2, 8, 40, 1),
        InteractionPayload::PatternManagerRow { index: 1 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 3,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );
    assert_eq!(app.pattern_index, 0);
    assert_eq!(app.mode, AppMode::Patterns);

    app.interaction_map = InteractionMap::new();
    app.interaction_map.register_with_payload(
        interaction_region::PATTERN_MANAGER_ROW,
        ratatui::layout::Rect::new(2, 9, 40, 1),
        InteractionPayload::PatternManagerRow { index: usize::MAX },
    );
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 3,
            row: 9,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.pattern_index, 0);
    assert_eq!(app.mode, AppMode::Patterns);
}

#[test]
fn mouse_click_ignores_pattern_headers_gutters_and_panels() {
    let mut app = App::default();
    app.cursor.row = 3;
    app.cursor.track = 1;
    app.interaction_map.register(
        interaction_region::PANEL_PATTERN,
        ratatui::layout::Rect::new(10, 5, 80, 20),
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 6,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(app.cursor.row, 3);
    assert_eq!(app.cursor.track, 1);
}

#[test]
fn mouse_click_selects_project_browser_entries() {
    let mut app = App {
        mode: AppMode::ProjectBrowser,
        project_browser_view: Some(AppProjectBrowserView {
            current_dir: PathBuf::from("/tmp/projects"),
            entries: vec![
                AppProjectBrowserEntry {
                    path: PathBuf::from("/tmp/projects/a.trk"),
                    name: "a.trk".to_string(),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "A".to_string(),
                },
                AppProjectBrowserEntry {
                    path: PathBuf::from("/tmp/projects/b.trk"),
                    name: "b.trk".to_string(),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "B".to_string(),
                },
            ],
            cursor: 0,
            message: None,
        }),
        ..App::default()
    };
    app.interaction_map.register_with_payload(
        interaction_region::PROJECT_BROWSER_ENTRY,
        ratatui::layout::Rect::new(3, 8, 30, 1),
        InteractionPayload::ProjectBrowserEntry { index: 1 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(
        app.project_browser_view
            .as_ref()
            .map(|browser| browser.cursor),
        Some(1)
    );
}

#[test]
fn mouse_click_selects_sample_browser_entries() {
    let mut app = App {
        mode: AppMode::SampleBrowser,
        sample_browser_view: Some(AppSampleBrowserView {
            current_dir: PathBuf::from("/tmp/samples"),
            entries: vec![
                AppSampleBrowserEntry {
                    path: PathBuf::from("/tmp/samples/kick.wav"),
                    name: "kick.wav".to_string(),
                    kind: SampleBrowserEntryKind::SupportedSample,
                },
                AppSampleBrowserEntry {
                    path: PathBuf::from("/tmp/samples/readme.txt"),
                    name: "readme.txt".to_string(),
                    kind: SampleBrowserEntryKind::UnsupportedFile,
                },
            ],
            cursor: 0,
            preview: None,
            message: None,
        }),
        ..App::default()
    };
    app.interaction_map.register_with_payload(
        interaction_region::SAMPLE_BROWSER_ENTRY,
        ratatui::layout::Rect::new(3, 8, 30, 1),
        InteractionPayload::SampleBrowserEntry { index: 1 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 8,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    let browser = app.sample_browser_view.as_ref().expect("browser");
    assert_eq!(browser.cursor, 1);
    assert_eq!(browser.message.as_deref(), Some("Unsupported file type"));
}

#[test]
fn browser_clicks_ignore_borders_headers_and_empty_rows() {
    let mut sample_app = App {
        mode: AppMode::SampleBrowser,
        sample_browser_view: Some(AppSampleBrowserView {
            current_dir: PathBuf::from("/tmp/samples"),
            entries: vec![
                AppSampleBrowserEntry {
                    path: PathBuf::from("/tmp/samples/kick.wav"),
                    name: "kick.wav".to_string(),
                    kind: SampleBrowserEntryKind::SupportedSample,
                },
                AppSampleBrowserEntry {
                    path: PathBuf::from("/tmp/samples/snare.wav"),
                    name: "snare.wav".to_string(),
                    kind: SampleBrowserEntryKind::SupportedSample,
                },
            ],
            cursor: 0,
            preview: None,
            message: None,
        }),
        ..App::default()
    };
    sample_app.interaction_map.register(
        interaction_region::VIEW_SAMPLE_BROWSER,
        ratatui::layout::Rect::new(0, 3, 160, 34),
    );

    sample_app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 12,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(
        sample_app
            .sample_browser_view
            .as_ref()
            .map(|browser| browser.cursor),
        Some(0)
    );

    let mut project_app = App {
        mode: AppMode::ProjectBrowser,
        project_browser_view: Some(AppProjectBrowserView {
            current_dir: PathBuf::from("/tmp/projects"),
            entries: vec![
                AppProjectBrowserEntry {
                    path: PathBuf::from("/tmp/projects/a.trk"),
                    name: "a.trk".to_string(),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "A".to_string(),
                },
                AppProjectBrowserEntry {
                    path: PathBuf::from("/tmp/projects/b.trk"),
                    name: "b.trk".to_string(),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "B".to_string(),
                },
            ],
            cursor: 0,
            message: None,
        }),
        ..App::default()
    };
    project_app.interaction_map.register(
        interaction_region::VIEW_PROJECT_BROWSER,
        ratatui::layout::Rect::new(0, 3, 160, 34),
    );

    project_app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2,
            row: 12,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    assert_eq!(
        project_app
            .project_browser_view
            .as_ref()
            .map(|browser| browser.cursor),
        Some(0)
    );
}

#[test]
fn mouse_right_click_assigns_sample_browser_entry_to_current_track() {
    let dir = std::env::temp_dir().join(format!("trk-mouse-sample-assign-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create sample dir");
    let stale_sample_path = dir.join("kick.wav");
    let sample_path = dir.join("hat.wav");
    std::fs::write(
        &stale_sample_path,
        wav_pcm16_bytes(44_100, 1, &[0, i16::MIN]),
    )
    .expect("write stale wav");
    std::fs::write(&sample_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX])).expect("write wav");

    let mut app = App {
        mode: AppMode::SampleBrowser,
        cursor: Cursor {
            track: 1,
            ..Cursor::new()
        },
        sample_browser_view: Some(AppSampleBrowserView {
            current_dir: dir.clone(),
            entries: vec![
                AppSampleBrowserEntry {
                    path: stale_sample_path.clone(),
                    name: "kick.wav".to_string(),
                    kind: SampleBrowserEntryKind::SupportedSample,
                },
                AppSampleBrowserEntry {
                    path: sample_path.clone(),
                    name: "hat.wav".to_string(),
                    kind: SampleBrowserEntryKind::SupportedSample,
                },
            ],
            cursor: 0,
            preview: None,
            message: None,
        }),
        ..App::default()
    };
    app.interaction_map.register_with_payload(
        interaction_region::SAMPLE_BROWSER_ENTRY,
        ratatui::layout::Rect::new(3, 7, 30, 1),
        InteractionPayload::SampleBrowserEntry { index: 1 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 4,
            row: 7,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    let track_id = app.song.tracks[1].id;
    let assignment = app
        .song
        .sample_assignment_for_track(track_id)
        .expect("sample assignment");
    let sample = app
        .song
        .sample_for_id(assignment.sample)
        .expect("sample reference");
    assert_eq!(sample.path, sample_path.to_string_lossy());
    assert_eq!(app.mode, AppMode::Sampler);

    let _ = std::fs::remove_file(&stale_sample_path);
    let _ = std::fs::remove_file(&sample_path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn mouse_click_renoise_sidebar_tabs_open_sections() {
    let mut app = App::default();
    let viewport = large_mouse_viewport();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 139,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );
    assert_eq!(app.mode, AppMode::SampleBrowser);

    app.open_tracker_view();
    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 131,
            row: 10,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );
    assert_eq!(app.mode, AppMode::Tracks);
}

#[test]
fn mouse_click_renoise_sidebar_pattern_selects_pattern() {
    let mut app = App::default();
    app.create_pattern();
    app.create_pattern();
    app.select_pattern(0);
    let viewport = large_mouse_viewport();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 126,
            row: 24,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );

    assert_eq!(app.pattern_index, 1);
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn mouse_click_renoise_sidebar_sample_selects_assigned_track() {
    let mut app = App::default();
    let sample = app.song.upsert_sample_reference("samples/kick.wav", "Kick");
    let track = app.song.tracks[1].id;
    app.song
        .assign_sample_to_track(track, sample)
        .expect("assign sample");
    app.cursor.track = 0;
    let viewport = large_mouse_viewport();

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 126,
            row: 22,
            modifiers: KeyModifiers::NONE,
        },
        viewport,
    );

    assert_eq!(app.cursor.track, 1);
    assert_eq!(app.mode, AppMode::Normal);
}

#[test]
fn mouse_click_assigns_dsp_device_from_palette_to_master() {
    let mut app = App::default();
    app.open_dsp_rack_view();
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    app.interaction_map.register_with_payload(
        interaction_region::DSP_PALETTE_ENTRY,
        ratatui::layout::Rect::new(3, 15, 30, 1),
        InteractionPayload::DspPaletteEntry { index: 7 },
    );

    app.handle_mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 15,
            modifiers: KeyModifiers::NONE,
        },
        large_mouse_viewport(),
    );

    let rack = app.tui_dsp_rack_view();
    assert!(rack.device_palette.is_none());
    assert_eq!(rack.master_effects.len(), 1);
    assert!(matches!(
        rack.master_effects[0].kind,
        EffectDeviceKind::Reverb { .. }
    ));
}
