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
                        .unwrap_or_else(|_| "trk=info".into())
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
            println!("trk {}", env!("CARGO_PKG_VERSION"));
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
        CliCommand::ExportPlan(plan_args) => {
            run_export_plan(plan_args)?;
            return Ok(());
        }
        CliCommand::ExportAudio(export_args) => {
            run_export_audio(export_args)?;
            return Ok(());
        }
        CliCommand::ExportStems(stems_args) => {
            run_export_stems(stems_args)?;
            return Ok(());
        }
        CliCommand::ExportStrudel(strudel_args) => {
            run_export_strudel(strudel_args)?;
            return Ok(());
        }
        CliCommand::ExportMusicXml(musicxml_args) => {
            run_export_musicxml(musicxml_args)?;
            return Ok(());
        }
        CliCommand::ReportProject(report_args) => {
            run_report_project(report_args)?;
            return Ok(());
        }
        CliCommand::ReportCritique(report_args) => {
            run_report_critique(report_args)?;
            return Ok(());
        }
        CliCommand::Analyze(analysis_args) => {
            run_analyze(analysis_args)?;
            return Ok(());
        }
        CliCommand::Compare(compare_args) => {
            run_compare(compare_args)?;
            return Ok(());
        }
        CliCommand::ValidateRoundTrip(validation_args) => {
            run_validate_round_trip(validation_args)?;
            return Ok(());
        }
        CliCommand::GraphValidate(graph_args) => {
            run_graph_validate(graph_args)?;
            return Ok(());
        }
        CliCommand::GraphCompile(graph_args) => {
            run_graph_compile(graph_args)?;
            return Ok(());
        }
        CliCommand::ImportXrns(import_args) => {
            run_import_xrns(import_args)?;
            return Ok(());
        }
        CliCommand::ImportMidi(import_args) => {
            run_import_midi(import_args)?;
            return Ok(());
        }
        CliCommand::ImportMusicXml(import_args) => {
            run_import_musicxml(import_args)?;
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
    if let Err(error) = app.load_ai_session() {
        tracing::warn!(?error, "failed to load configured AI session");
    }
    let mut terminal = TerminalGuard::enter()?;
    if std::env::var_os("TRK_DEBUG_PANIC_AFTER_TERMINAL_ENTER").is_some() {
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
            app.interaction_map = app.render_interactions(frame);
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
                    app.handle_mouse(
                        mouse,
                        MouseViewport {
                            terminal_width: terminal.size().0,
                            terminal_height: terminal.size().1,
                        },
                    );
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
