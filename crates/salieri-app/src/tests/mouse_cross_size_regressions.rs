use super::*;
use ratatui::{backend::TestBackend, Terminal};

const VIEWPORTS: [(u16, u16); 4] = [(72, 24), (80, 24), (100, 28), (140, 36)];

fn render(app: &mut App, size: (u16, u16)) -> InteractionMap {
    let (width, height) = size;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut rendered = InteractionMap::new();
    terminal
        .draw(|frame| rendered = app.render_interactions(frame))
        .expect("render application");
    app.interaction_map = rendered.clone();
    rendered
}

fn region(
    map: &InteractionMap,
    id: salieri_tui::InteractionRegionId,
    predicate: impl Fn(InteractionPayload) -> bool,
) -> salieri_tui::InteractionRegion {
    map.regions()
        .iter()
        .copied()
        .find(|region| region.id == id && predicate(region.payload))
        .unwrap_or_else(|| panic!("missing rendered region {}", id.as_str()))
}

fn point(region: salieri_tui::InteractionRegion) -> (u16, u16) {
    (
        region.area.x + region.area.width.saturating_sub(1) / 2,
        region.area.y + region.area.height.saturating_sub(1) / 2,
    )
}

fn inert_neighbor(
    map: &InteractionMap,
    target: salieri_tui::InteractionRegion,
    size: (u16, u16),
) -> (u16, u16) {
    let (width, height) = size;
    let candidates = [
        (target.area.x.saturating_sub(1), target.area.y),
        (
            target.area.x.saturating_add(target.area.width),
            target.area.y,
        ),
        (target.area.x, target.area.y.saturating_sub(1)),
        (
            target.area.x,
            target.area.y.saturating_add(target.area.height),
        ),
    ];
    candidates
        .into_iter()
        .filter(|(x, y)| *x < width && *y < height)
        .find(|(x, y)| {
            map.hit_test(*x, *y)
                .is_some_and(|hit| hit.payload == InteractionPayload::None)
        })
        .expect("immediately adjacent inert rendered region")
}

fn dispatch(app: &mut App, size: (u16, u16), kind: MouseEventKind, at: (u16, u16)) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column: at.0,
            row: at.1,
            modifiers: KeyModifiers::NONE,
        },
        MouseViewport {
            terminal_width: size.0,
            terminal_height: size.1,
        },
    );
}

fn click(app: &mut App, size: (u16, u16), at: (u16, u16)) {
    dispatch(app, size, MouseEventKind::Down(MouseButton::Left), at);
}

fn add_tracks_and_sequence(app: &mut App, tracks: usize, patterns: usize) {
    while app.song.tracks.len() < tracks {
        app.song.create_track();
    }
    while app.song.patterns.len() < patterns {
        let pattern = app.song.create_pattern(64);
        app.song
            .push_sequence_pattern(pattern)
            .expect("append sequence pattern");
    }
}

#[test]
fn rendered_pattern_cells_dispatch_at_every_supported_size() {
    for size in VIEWPORTS {
        let mut app = App::default();
        add_tracks_and_sequence(&mut app, 8, 1);
        app.row_offset = 12;
        app.cursor.row = 15;
        let map = render(&mut app, size);
        let target = region(&map, interaction_region::PATTERN_CELL, |payload| {
            matches!(
                payload,
                InteractionPayload::PatternCell { row, track: 0 } if row > 12 && row != 15
            )
        });
        let InteractionPayload::PatternCell { row, track } = target.payload else {
            unreachable!()
        };

        let before = app.cursor;
        click(&mut app, size, inert_neighbor(&map, target, size));
        assert_eq!(app.cursor, before, "outside click at {size:?}");

        click(&mut app, size, point(target));
        assert_eq!((app.cursor.row, app.cursor.track), (row, track), "{size:?}");
    }
}

#[test]
fn rendered_scrolled_composite_rows_dispatch_absolute_items() {
    for size in [(80, 24), (100, 28)] {
        let mut tracks = App::default();
        add_tracks_and_sequence(&mut tracks, 24, 40);
        tracks.cursor.track = 18;
        let map = render(&mut tracks, size);
        let target = region(
            &map,
            interaction_region::COMPOSITE_TRACK_ROW,
            |payload| matches!(payload, InteractionPayload::CompositeTrackRow { track } if track > 0 && track != 18),
        );
        let InteractionPayload::CompositeTrackRow { track } = target.payload else {
            unreachable!()
        };
        click(&mut tracks, size, inert_neighbor(&map, target, size));
        assert_eq!(tracks.cursor.track, 18, "outside track row at {size:?}");
        click(&mut tracks, size, point(target));
        assert_eq!(tracks.cursor.track, track, "{size:?}");

        let mut sequence = App::default();
        add_tracks_and_sequence(&mut sequence, 24, 40);
        sequence.sequence_cursor = 30;
        let map = render(&mut sequence, size);
        let target = region(
            &map,
            interaction_region::COMPOSITE_SEQUENCE_ROW,
            |payload| matches!(payload, InteractionPayload::CompositeSequenceRow { position } if position > 0 && position != 30),
        );
        let InteractionPayload::CompositeSequenceRow { position } = target.payload else {
            unreachable!()
        };
        click(&mut sequence, size, inert_neighbor(&map, target, size));
        assert_eq!(
            sequence.sequence_cursor, 30,
            "outside sequence row at {size:?}"
        );
        click(&mut sequence, size, point(target));
        assert_eq!(sequence.sequence_cursor, position, "{size:?}");
    }
}

fn sample_browser() -> App {
    let mut app = App {
        mode: AppMode::SampleBrowser,
        sample_browser_view: Some(AppSampleBrowserView {
            current_dir: PathBuf::from("/samples"),
            entries: (0..40)
                .map(|index| AppSampleBrowserEntry {
                    path: PathBuf::from(format!("/samples/{index:02}.wav")),
                    name: format!("{index:02}.wav"),
                    kind: SampleBrowserEntryKind::SupportedSample,
                })
                .collect(),
            cursor: 30,
            preview: None,
            message: None,
        }),
        ..App::default()
    };
    app.focus_panel(FocusPanel::SampleBrowser);
    app
}

fn project_browser() -> App {
    let mut app = App {
        mode: AppMode::ProjectBrowser,
        project_browser_view: Some(AppProjectBrowserView {
            current_dir: PathBuf::from("/projects"),
            entries: (0..40)
                .map(|index| AppProjectBrowserEntry {
                    path: PathBuf::from(format!("/projects/{index:02}.salieri")),
                    name: format!("{index:02}.salieri"),
                    kind: ProjectBrowserEntryKind::Project,
                    detail: "project".to_string(),
                })
                .collect(),
            cursor: 30,
            message: None,
        }),
        ..App::default()
    };
    app.focus_panel(FocusPanel::ProjectBrowser);
    app
}

#[test]
fn rendered_scrolled_browser_entries_dispatch_absolute_items() {
    for (size, mut app, id) in [
        (
            (72, 24),
            sample_browser(),
            interaction_region::SAMPLE_BROWSER_ENTRY,
        ),
        (
            (140, 36),
            sample_browser(),
            interaction_region::SAMPLE_BROWSER_ENTRY,
        ),
        (
            (80, 24),
            project_browser(),
            interaction_region::PROJECT_BROWSER_ENTRY,
        ),
        (
            (100, 28),
            project_browser(),
            interaction_region::PROJECT_BROWSER_ENTRY,
        ),
    ] {
        let map = render(&mut app, size);
        let target = region(&map, id, |payload| {
            matches!(
                payload,
                InteractionPayload::SampleBrowserEntry { index }
                    | InteractionPayload::ProjectBrowserEntry { index }
                    if index > 0 && index != 30
            )
        });
        click(&mut app, size, inert_neighbor(&map, target, size));
        let cursor = app
            .sample_browser_view
            .as_ref()
            .map(|browser| browser.cursor)
            .or_else(|| {
                app.project_browser_view
                    .as_ref()
                    .map(|browser| browser.cursor)
            });
        assert_eq!(cursor, Some(30), "outside browser row at {size:?}");

        click(&mut app, size, point(target));
        let expected = match target.payload {
            InteractionPayload::SampleBrowserEntry { index }
            | InteractionPayload::ProjectBrowserEntry { index } => index,
            _ => unreachable!(),
        };
        let cursor = app
            .sample_browser_view
            .as_ref()
            .map(|browser| browser.cursor)
            .or_else(|| {
                app.project_browser_view
                    .as_ref()
                    .map(|browser| browser.cursor)
            });
        assert_eq!(cursor, Some(expected), "{size:?}");
    }
}

#[test]
fn rendered_help_overlay_captures_adjacent_clicks_and_dispatches_tabs() {
    for size in VIEWPORTS {
        let mut app = App::default();
        app.open_help();
        app.help_scroll = 6;
        let map = render(&mut app, size);
        let target = region(&map, interaction_region::HELP_TAB, |payload| {
            matches!(payload, InteractionPayload::HelpTab { index: 1 })
        });

        click(&mut app, size, inert_neighbor(&map, target, size));
        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.help_tab, HelpTab::Basics);
        assert_eq!(app.help_scroll, 6);

        click(&mut app, size, point(target));
        assert_eq!(app.help_tab, HelpTab::Editing, "{size:?}");
        assert_eq!(app.help_scroll, 0, "{size:?}");
    }
}

#[test]
fn rendered_scrolled_dsp_rows_and_palette_dispatch_absolute_items() {
    let size = (72, 24);
    let mut devices = App::default();
    for command in [
        "dsp track gain 1.000",
        "dsp track pan 0.000",
        "dsp track balance 0.000",
        "dsp track width 1.000",
        "dsp track phase on off",
        "dsp track filter lowpass 2000 0.250 0.000 0.500",
        "dsp track delay sync 250 500 0.350 0.250",
        "dsp track reverb 0.500 20 2.500 0.250",
    ] {
        enter_command(&mut devices, command);
    }
    enter_command(&mut devices, "focus dsp");
    devices.dsp_rack_cursor = 7;
    let map = render(&mut devices, size);
    let target = region(
        &map,
        interaction_region::DSP_DEVICE_ROW,
        |payload| matches!(payload, InteractionPayload::DspDeviceRow { target: DspRackChain::Track, index } if index > 0 && index != 7),
    );
    click(&mut devices, size, inert_neighbor(&map, target, size));
    assert_eq!(devices.dsp_rack_cursor, 7);
    click(&mut devices, size, point(target));
    let InteractionPayload::DspDeviceRow { index, .. } = target.payload else {
        unreachable!()
    };
    assert_eq!(devices.dsp_rack_cursor, index);

    let mut parameters = App::default();
    enter_command(&mut parameters, "dsp track delay sync 250 500 0.350 0.250");
    enter_command(&mut parameters, "focus dsp");
    parameters.dsp_parameter_cursor = 5;
    let map = render(&mut parameters, size);
    let target = region(
        &map,
        interaction_region::DSP_PARAMETER_ROW,
        |payload| matches!(payload, InteractionPayload::DspParameterRow { index } if index > 0 && index != 5),
    );
    click(&mut parameters, size, inert_neighbor(&map, target, size));
    assert_eq!(parameters.dsp_parameter_cursor, 5);
    click(&mut parameters, size, point(target));
    let InteractionPayload::DspParameterRow { index } = target.payload else {
        unreachable!()
    };
    assert_eq!(parameters.dsp_parameter_cursor, index);

    let mut palette = App::default();
    enter_command(&mut palette, "focus dsp");
    palette.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    palette.dsp_device_palette_cursor = 14;
    let map = render(&mut palette, size);
    let target = region(&map, interaction_region::DSP_PALETTE_ENTRY, |payload| {
        matches!(payload, InteractionPayload::DspPaletteEntry { index: 12 })
    });
    click(&mut palette, size, inert_neighbor(&map, target, size));
    assert!(palette.dsp_device_palette_open);
    assert!(palette.tui_dsp_rack_view().track_effects.is_empty());
    click(&mut palette, size, point(target));
    assert!(!palette.dsp_device_palette_open);
    assert!(matches!(
        palette.tui_dsp_rack_view().track_effects[0].kind,
        EffectDeviceKind::Phaser { .. }
    ));
}

fn loaded_sampler() -> App {
    let mut app = App {
        sample_view: Some(AppSampleView {
            source_path: PathBuf::from("/samples/test.wav"),
            sample: Sample {
                name: "test.wav".to_string(),
                sample_rate: 44_100,
                channels: 1,
                frames: 64,
                data: vec![0.0; 64],
            },
            overview: WaveformOverview {
                sample_rate: 44_100,
                channels: 1,
                frames: 64,
                duration_seconds: 64.0 / 44_100.0,
                buckets: vec![
                    WaveformBucket {
                        min: -0.5,
                        max: 0.5,
                    };
                    32
                ],
            },
        }),
        ..App::default()
    };
    app.open_sampler_view();
    app
}

#[test]
fn rendered_sampler_controls_dispatch_at_every_supported_size() {
    for size in VIEWPORTS {
        let mut app = loaded_sampler();
        let map = render(&mut app, size);
        let target = region(&map, interaction_region::SAMPLER_ACTION, |payload| {
            matches!(
                payload,
                InteractionPayload::SamplerAction {
                    action: SamplerAction::SelectEnvelope(SamplerEnvelopeField::Decay)
                }
            )
        });
        click(&mut app, size, inert_neighbor(&map, target, size));
        assert_eq!(app.sampler_envelope_field, SamplerEnvelopeField::Attack);
        click(&mut app, size, point(target));
        assert_eq!(
            app.sampler_envelope_field,
            SamplerEnvelopeField::Decay,
            "{size:?}"
        );
    }
}
