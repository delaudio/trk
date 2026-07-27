use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 120,
        terminal_height: 36,
    }
}

fn wheel(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
}

fn register(app: &mut App, id: salieri_tui::InteractionRegionId, x: u16) {
    app.interaction_map
        .register(id, ratatui::layout::Rect::new(x, 4, 18, 20));
}

#[test]
fn vertical_wheel_routes_composite_panels_without_cross_mutation() {
    let mut app = App::default();
    while app.song.tracks.len() < 6 {
        app.song.create_track();
    }
    let pattern = app.song.patterns[0].id;
    while app.song.sequence.len() < 6 {
        app.song
            .push_sequence_pattern(pattern)
            .expect("append sequence pattern");
    }
    app.cursor.row = 10;
    register(&mut app, interaction_region::PANEL_TRACKS, 0);
    register(&mut app, interaction_region::PANEL_SEQUENCE, 20);
    register(&mut app, interaction_region::PANEL_PATTERN, 40);

    wheel(&mut app, MouseEventKind::ScrollDown, 42, 8);
    assert_eq!(
        (app.cursor.row, app.cursor.track, app.sequence_cursor),
        (13, 0, 0)
    );

    wheel(&mut app, MouseEventKind::ScrollDown, 2, 8);
    assert_eq!(
        (app.cursor.row, app.cursor.track, app.sequence_cursor),
        (13, 3, 0)
    );

    wheel(&mut app, MouseEventKind::ScrollDown, 22, 8);
    assert_eq!(
        (app.cursor.row, app.cursor.track, app.sequence_cursor),
        (13, 3, 3)
    );
}

#[test]
fn non_scrollable_regions_and_unsupported_horizontal_axes_are_no_ops() {
    let mut app = App::default();
    while app.song.tracks.len() < 6 {
        app.song.create_track();
    }
    app.cursor.row = 8;
    app.cursor.track = 2;
    register(&mut app, interaction_region::PANEL_INSPECTOR, 0);
    register(&mut app, interaction_region::PANEL_TRACKS, 20);
    register(&mut app, interaction_region::PANEL_PATTERN, 40);

    wheel(&mut app, MouseEventKind::ScrollDown, 2, 8);
    wheel(&mut app, MouseEventKind::ScrollRight, 22, 8);
    assert_eq!((app.cursor.row, app.cursor.track), (8, 2));

    wheel(&mut app, MouseEventKind::ScrollRight, 42, 8);
    assert_eq!((app.cursor.row, app.cursor.track), (8, 3));
}

#[test]
fn modal_modes_capture_both_wheel_axes_outside_their_scrollable_content() {
    for mode in [
        AppMode::CommandPalette,
        AppMode::Help,
        AppMode::MidiSettings,
        AppMode::Dialog,
    ] {
        let mut app = App {
            mode,
            ..App::default()
        };
        app.cursor.row = 8;
        app.cursor.track = 1;
        register(&mut app, interaction_region::PANEL_PATTERN, 0);

        wheel(&mut app, MouseEventKind::ScrollDown, 2, 8);
        wheel(&mut app, MouseEventKind::ScrollRight, 2, 8);

        assert_eq!((app.cursor.row, app.cursor.track), (8, 1), "{mode:?}");
    }
}

#[test]
fn manager_clip_and_midi_lists_move_their_own_bounded_selection() {
    let mut patterns = App::default();
    while patterns.song.patterns.len() < 6 {
        patterns.song.create_pattern(64);
    }
    register(&mut patterns, interaction_region::VIEW_PATTERNS, 0);
    wheel(&mut patterns, MouseEventKind::ScrollDown, 2, 8);
    assert_eq!(patterns.pattern_index, 3);

    let mut clips = App::default();
    while clips.song.tracks.len() < 4 {
        clips.song.create_track();
    }
    while clips.song.clip_scenes.len() < 6 {
        clips
            .song
            .create_clip_scene_from_pattern("Scene", 0)
            .expect("create clip scene");
    }
    register(&mut clips, interaction_region::VIEW_CLIPS, 0);
    wheel(&mut clips, MouseEventKind::ScrollDown, 2, 8);
    wheel(&mut clips, MouseEventKind::ScrollRight, 2, 8);
    assert_eq!((clips.clip_scene_cursor, clips.clip_track_cursor), (3, 1));

    let mut midi = App {
        mode: AppMode::MidiSettings,
        midi_ports: (0..6)
            .map(|index| MidiOutputPort {
                index,
                name: format!("Port {index}"),
            })
            .collect(),
        ..App::default()
    };
    midi.interaction_map.register_with_payload(
        interaction_region::MIDI_SETTINGS_PORT,
        ratatui::layout::Rect::new(0, 4, 40, 1),
        InteractionPayload::MidiPortRow { index: 0 },
    );
    wheel(&mut midi, MouseEventKind::ScrollDown, 2, 4);
    assert_eq!(midi.midi_port_cursor, 3);
}

#[test]
fn browser_and_dsp_regions_route_to_the_hovered_list() {
    let sample_entries = (0..6)
        .map(|index| AppSampleBrowserEntry {
            path: PathBuf::from(format!("dir-{index}")),
            name: format!("dir-{index}"),
            kind: SampleBrowserEntryKind::Directory,
        })
        .collect();
    let mut samples = App {
        mode: AppMode::SampleBrowser,
        sample_browser_view: Some(AppSampleBrowserView {
            current_dir: PathBuf::from("."),
            entries: sample_entries,
            cursor: 0,
            preview: None,
            message: None,
        }),
        ..App::default()
    };
    register(&mut samples, interaction_region::VIEW_SAMPLE_BROWSER, 0);
    wheel(&mut samples, MouseEventKind::ScrollDown, 2, 8);
    assert_eq!(
        samples.sample_browser_view.as_ref().map(|view| view.cursor),
        Some(3)
    );

    let project_entries = (0..6)
        .map(|index| AppProjectBrowserEntry {
            path: PathBuf::from(format!("project-{index}")),
            name: format!("project-{index}"),
            kind: ProjectBrowserEntryKind::Directory,
            detail: "directory".to_string(),
        })
        .collect();
    let mut projects = App {
        mode: AppMode::ProjectBrowser,
        project_browser_view: Some(AppProjectBrowserView {
            current_dir: PathBuf::from("."),
            entries: project_entries,
            cursor: 0,
            message: None,
        }),
        ..App::default()
    };
    register(&mut projects, interaction_region::VIEW_PROJECT_BROWSER, 0);
    wheel(&mut projects, MouseEventKind::ScrollDown, 2, 8);
    assert_eq!(
        projects
            .project_browser_view
            .as_ref()
            .map(|view| view.cursor),
        Some(3)
    );

    let mut dsp = App::default();
    enter_command(&mut dsp, "dsp track gain 1.000");
    enter_command(&mut dsp, "dsp track pan 0.000");
    enter_command(&mut dsp, "dsp track filter lowpass 2000 0.250 0.000 0.500");
    enter_command(&mut dsp, "dsp master gain 1.000");
    enter_command(&mut dsp, "dsp master pan 0.000");
    enter_command(&mut dsp, "focus dsp");
    dsp.interaction_map.register_with_payload(
        interaction_region::DSP_CHAIN,
        ratatui::layout::Rect::new(0, 4, 40, 10),
        InteractionPayload::DspRackTarget {
            target: DspRackChain::Track,
        },
    );
    wheel(&mut dsp, MouseEventKind::ScrollDown, 2, 8);
    assert_eq!(dsp.dsp_rack_cursor, 2);

    dsp.interaction_map.register_with_payload(
        interaction_region::DSP_PARAMETER_ROW,
        ratatui::layout::Rect::new(0, 20, 40, 1),
        InteractionPayload::DspParameterRow { index: 0 },
    );
    wheel(&mut dsp, MouseEventKind::ScrollDown, 2, 20);
    assert_eq!(dsp.dsp_parameter_cursor, 3);

    dsp.open_dsp_device_palette();
    dsp.interaction_map.register_with_payload(
        interaction_region::DSP_PALETTE_ENTRY,
        ratatui::layout::Rect::new(50, 8, 40, 1),
        InteractionPayload::DspPaletteEntry { index: 0 },
    );
    let device_before = dsp.dsp_rack_cursor;
    wheel(&mut dsp, MouseEventKind::ScrollDown, 2, 8);
    assert_eq!(dsp.dsp_rack_cursor, device_before);

    wheel(&mut dsp, MouseEventKind::ScrollDown, 52, 8);
    assert_eq!(dsp.dsp_device_palette_cursor, 3);
    dsp.close_dsp_device_palette();

    dsp.interaction_map.register_with_payload(
        interaction_region::DSP_CHAIN,
        ratatui::layout::Rect::new(90, 4, 30, 10),
        InteractionPayload::DspRackTarget {
            target: DspRackChain::Master,
        },
    );
    wheel(&mut dsp, MouseEventKind::ScrollDown, 92, 8);
    assert_eq!(
        (dsp.tui_dsp_rack_view().selected_target, dsp.dsp_rack_cursor),
        (DspRackTargetView::Master, 1)
    );
}
