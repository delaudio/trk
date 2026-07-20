use super::*;

pub(crate) fn run_cli() -> Result<()> {
    let args = CliArgs::parse(std::env::args().skip(1));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            args.log_level
                .as_deref()
                .map(tracing_subscriber::EnvFilter::new)
                .unwrap_or_else(|| {
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "salieri=info".into())
                }),
        )
        .init();

    let result = run(args);
    if let Err(error) = &result {
        tracing::error!(?error, "application exited with an error");
    }
    result
}

fn run(args: CliArgs) -> Result<()> {
    match &args.command {
        CliCommand::Help => {
            print_help();
            return Ok(());
        }
        CliCommand::Version => {
            println!("salieri {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        CliCommand::ListMidiOutputs => {
            print_midi_outputs()?;
            return Ok(());
        }
        CliCommand::ListMidiInputs => {
            print_midi_inputs()?;
            return Ok(());
        }
        CliCommand::TransformEuclidean(transform_args) => {
            run_transform_euclidean(transform_args)?;
            return Ok(());
        }
        CliCommand::SampleInspect(sample_args) => {
            run_sample_inspect(sample_args)?;
            return Ok(());
        }
        CliCommand::ExportAudio(export_args) => {
            run_export_audio(export_args)?;
            return Ok(());
        }
        CliCommand::ImportXrns(import_args) => {
            run_import_xrns(import_args)?;
            return Ok(());
        }
        CliCommand::Run | CliCommand::MidiTest => {}
    }

    let loaded_config = load_config(
        args.config_path.as_deref(),
        ConfigOverrides {
            midi_log_file: args.midi_log_path,
        },
    )?;
    tracing::debug!(config = %loaded_config.metadata().summary(), "configuration resolved");
    if args.command == CliCommand::MidiTest {
        run_midi_test(loaded_config.config(), &args.midi_test)?;
        return Ok(());
    }
    let config = loaded_config.into_config();

    let project_path = args.project_path;
    let mut app = match &project_path {
        Some(path) => App::from_file(path, config)
            .with_context(|| format!("failed to open project {}", path.display()))?,
        None => App::new(config),
    };
    let mut terminal = TerminalGuard::enter()?;
    if std::env::var_os("SALIERI_DEBUG_PANIC_AFTER_TERMINAL_ENTER").is_some() {
        panic!("debug panic after terminal enter");
    }

    loop {
        app.drain_task_updates();
        app.drain_playback_updates();
        app.drain_midi_input();
        app.expire_notification();
        app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
            visible_rows: terminal.visible_pattern_rows(),
            visible_tracks: terminal.visible_pattern_tracks(),
        }));
        terminal.draw(|frame| {
            let midi_ports = app.tui_midi_ports();
            let midi_settings = app.tui_midi_settings(&midi_ports);
            let notification = app.tui_notification();
            let sample_browser_entries = app.tui_sample_browser_entries();
            let sample_browser = app.tui_sample_browser_view(&sample_browser_entries);
            let project_browser_entries = app.tui_project_browser_entries();
            let project_browser = app.tui_project_browser_view(&project_browser_entries);
            let command_palette_entries = app.tui_command_palette_entries();
            let command_palette = app.tui_command_palette(&command_palette_entries);
            let midi_status = app.tui_midi_status();
            render(
                frame,
                &app.song,
                TuiState {
                    cursor: app.cursor,
                    row_offset: app.row_offset,
                    track_offset: app.track_offset,
                    pattern_index: app.pattern_index,
                    active_view: app.tui_active_view(),
                    selection: app.selection_rect(),
                    mode_label: app.mode.label(),
                    octave: app.octave,
                    dirty: app.dirty,
                    show_line_numbers_hex: app.show_line_numbers_hex,
                    command_line: app.command_line(),
                    notification,
                    show_help: app.mode == AppMode::Help,
                    help_scroll: app.help_scroll,
                    help_tab: app.help_tab,
                    is_playing: app.is_playing,
                    loop_pattern: app.loop_pattern,
                    playhead_row: app.playhead_row,
                    midi_status: midi_status.as_str(),
                    sequence_position: app.tui_sequence_position(),
                    quit_confirmation: app.quit_confirmation(),
                    delete_confirmation: app.delete_confirmation_message(),
                    midi_settings,
                    command_palette,
                    sampler_view: app.tui_sampler_view(),
                    sample_browser,
                    project_browser,
                    tracker_layout: app.tracker_layout,
                },
            );
        })?;

        if app.should_quit || terminal.interrupted() {
            break;
        }

        let timeout = UI_TICK_RATE
            .checked_sub(app.last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    app.handle_key(key);
                    if let Some((sample_browser, request)) = app.take_sample_browser_request() {
                        let result = terminal
                            .suspend(|| run_external_sample_browser(&sample_browser, &request));
                        match result {
                            Ok(browser_result) => app.finish_sample_browser(browser_result),
                            Err(error) => app.finish_sample_browser(Err(error)),
                        }
                    }
                    app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
                        visible_rows: terminal.visible_pattern_rows(),
                        visible_tracks: terminal.visible_pattern_tracks(),
                    }));
                }
                Event::Resize(_, _) => {
                    app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
                        visible_rows: terminal.visible_pattern_rows(),
                        visible_tracks: terminal.visible_pattern_tracks(),
                    }))
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse_wheel(mouse.kind);
                    app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
                        visible_rows: terminal.visible_pattern_rows(),
                        visible_tracks: terminal.visible_pattern_tracks(),
                    }));
                }
                _ => {}
            }
        }

        if app.last_tick.elapsed() >= UI_TICK_RATE {
            app.last_tick = Instant::now();
            app.dispatch_event(AppEvent::Runtime(RuntimeEvent::ViewportRefresh {
                visible_rows: terminal.visible_pattern_rows(),
                visible_tracks: terminal.visible_pattern_tracks(),
            }));
        }
    }

    Ok(())
}
