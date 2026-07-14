mod config;
mod persistence;
mod playback_runtime;
mod terminal;

use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use config::{load_config, AppConfig, SampleBrowserConfig};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use persistence::{load_project, save_project};
use playback_runtime::{PlaybackRuntime, PlaybackUpdate};
use salieri_core::{CellField, Cursor, Direction, NoteEvent, PatternCell, Song, TrackerCommand};
use salieri_midi::{list_output_ports, MidiMessage, MidiOutput, MidiOutputPort, MidirMidiOutput};
use salieri_sampler::{Sample, WaveformBucket, WaveformOverview};
use salieri_transform::{apply_euclidean, EuclideanRhythm};
use salieri_tui::{
    render, MidiPortView, MidiSettingsState, NotificationKind, NotificationView, SamplerViewState,
    SelectionRect, TuiState, TuiView,
};
use terminal::TerminalGuard;

const UI_TICK_RATE: Duration = Duration::from_millis(33);
const NOTIFICATION_TTL: Duration = Duration::from_secs(4);
const DEFAULT_NOTE_VELOCITY: u8 = 0x7f;
const UNDO_LIMIT: usize = 100;
const MIN_BPM: u16 = 1;
const MAX_BPM: u16 = 999;
const MIN_LPB: u8 = 1;
const MAX_LPB: u8 = 32;

fn main() -> Result<()> {
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
        CliCommand::TransformEuclidean(transform_args) => {
            run_transform_euclidean(transform_args)?;
            return Ok(());
        }
        CliCommand::SampleInspect(sample_args) => {
            run_sample_inspect(sample_args)?;
            return Ok(());
        }
        CliCommand::Run | CliCommand::MidiTest => {}
    }

    let mut config = load_config(args.config_path.as_deref())?;
    if let Some(midi_log_path) = args.midi_log_path {
        config.midi.log_file = Some(midi_log_path);
    }
    if args.command == CliCommand::MidiTest {
        run_midi_test(&config, &args.midi_test)?;
        return Ok(());
    }

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
        app.drain_playback_updates();
        app.expire_notification();
        app.keep_active_row_visible(terminal.visible_pattern_rows());
        terminal.draw(|frame| {
            let midi_ports = app.tui_midi_ports();
            let midi_settings = app.tui_midi_settings(&midi_ports);
            let notification = app.tui_notification();
            render(
                frame,
                &app.song,
                TuiState {
                    cursor: app.cursor,
                    row_offset: app.row_offset,
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
                    is_playing: app.is_playing,
                    loop_pattern: app.loop_pattern,
                    playhead_row: app.playhead_row,
                    midi_status: app.midi_status.as_str(),
                    sequence_position: app.tui_sequence_position(),
                    quit_confirmation: app.quit_confirmation(),
                    delete_confirmation: app.delete_confirmation_message(),
                    midi_settings,
                    sampler_view: app.tui_sampler_view(),
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
                    app.keep_active_row_visible(terminal.visible_pattern_rows());
                }
                Event::Resize(_, _) => app.keep_active_row_visible(terminal.visible_pattern_rows()),
                _ => {}
            }
        }

        if app.last_tick.elapsed() >= UI_TICK_RATE {
            app.last_tick = Instant::now();
            app.keep_active_row_visible(terminal.visible_pattern_rows());
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    command: CliCommand,
    project_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
    log_level: Option<String>,
    midi_log_path: Option<PathBuf>,
    midi_test: MidiTestArgs,
}

impl CliArgs {
    fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut project_path = None;
        let mut config_path = None;
        let mut log_level = None;
        let mut midi_log_path = None;
        let mut midi_test = MidiTestArgs::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    return Self {
                        command: CliCommand::Help,
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "-V" | "--version" => {
                    return Self {
                        command: CliCommand::Version,
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "--list-midi-outputs" => {
                    return Self {
                        command: CliCommand::ListMidiOutputs,
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "transform" => {
                    return Self {
                        command: parse_transform_command(args),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "sample" => {
                    return Self {
                        command: parse_sample_command(args),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "--midi-test-output" => {
                    midi_test.output = args.next();
                }
                "--midi-test-channel" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u8>().ok()) {
                        midi_test.channel = value;
                    }
                }
                "--midi-test-note" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u8>().ok()) {
                        midi_test.note = value;
                    }
                }
                "--midi-test-duration-ms" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u64>().ok()) {
                        midi_test.duration_ms = value;
                    }
                }
                "--config" => {
                    if let Some(path) = args.next() {
                        config_path = Some(PathBuf::from(path));
                    }
                }
                "--log-level" => {
                    log_level = args.next();
                }
                "--midi-log" => {
                    if let Some(path) = args.next() {
                        midi_log_path = Some(PathBuf::from(path));
                    }
                }
                _ if arg.starts_with("--config=") => {
                    config_path = Some(PathBuf::from(arg.trim_start_matches("--config=")));
                }
                _ if arg.starts_with("--log-level=") => {
                    log_level = Some(arg.trim_start_matches("--log-level=").to_string());
                }
                _ if arg.starts_with("--midi-log=") => {
                    midi_log_path = Some(PathBuf::from(arg.trim_start_matches("--midi-log=")));
                }
                _ if arg.starts_with("--midi-test-output=") => {
                    midi_test.output =
                        Some(arg.trim_start_matches("--midi-test-output=").to_string());
                }
                _ if arg.starts_with("--midi-test-channel=") => {
                    if let Ok(value) = arg.trim_start_matches("--midi-test-channel=").parse::<u8>()
                    {
                        midi_test.channel = value;
                    }
                }
                _ if arg.starts_with("--midi-test-note=") => {
                    if let Ok(value) = arg.trim_start_matches("--midi-test-note=").parse::<u8>() {
                        midi_test.note = value;
                    }
                }
                _ if arg.starts_with("--midi-test-duration-ms=") => {
                    if let Ok(value) = arg
                        .trim_start_matches("--midi-test-duration-ms=")
                        .parse::<u64>()
                    {
                        midi_test.duration_ms = value;
                    }
                }
                _ if project_path.is_none() => project_path = Some(PathBuf::from(arg)),
                _ => {}
            }
        }

        let command = if midi_test.output.is_some() {
            CliCommand::MidiTest
        } else {
            CliCommand::Run
        };

        Self {
            command,
            project_path,
            config_path,
            log_level,
            midi_log_path,
            midi_test,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MidiTestArgs {
    output: Option<String>,
    channel: u8,
    note: u8,
    duration_ms: u64,
}

impl Default for MidiTestArgs {
    fn default() -> Self {
        Self {
            output: None,
            channel: 1,
            note: 60,
            duration_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Run,
    Help,
    Version,
    ListMidiOutputs,
    MidiTest,
    TransformEuclidean(TransformEuclideanArgs),
    SampleInspect(SampleInspectArgs),
}

fn print_help() {
    println!(
        "Salieri Tracker\n\nUsage:\n  salieri [OPTIONS] [FILE]\n  salieri --list-midi-outputs\n  salieri --midi-test-output NAME_OR_INDEX [OPTIONS]\n  salieri transform euclidean INPUT OUTPUT [OPTIONS]\n  salieri sample inspect FILE [OPTIONS]\n  salieri --help\n  salieri --version\n\nOptions:\n  --config PATH                 Load config from PATH\n  --log-level LEVEL             Set tracing filter, e.g. debug or salieri=debug\n  --midi-log PATH               Write sent MIDI messages to PATH\n  --list-midi-outputs           List available MIDI output ports\n  --midi-test-output VALUE      Send one test note to a MIDI output name or index\n  --midi-test-channel CHANNEL   Test channel, 1-16 (default 1)\n  --midi-test-note NOTE         Test MIDI note, 0-127 (default 60)\n  --midi-test-duration-ms MS    Test note length (default 1000)\n\nTransform options:\n  --pattern N                   1-based pattern index (default 1)\n  --track N                     1-based track index (default 1)\n  --steps N                     Euclidean step count (default 16)\n  --pulses N                    Euclidean pulse count (default 4)\n  --rotation N                  Euclidean rotation (default 0)\n  --pitch NOTE                  MIDI note, 0-127 (default 36)\n  --velocity VALUE              Velocity, 0-127 (default 100)\n\nSample inspect options:\n  --format text|json            Output format (default text)\n  --buckets N, --width N        Waveform bucket count (default 64)\n\n  --help                        Show this help\n  --version                     Show version"
    );
}

fn parse_transform_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("euclidean") => CliCommand::TransformEuclidean(parse_transform_euclidean_args(args)),
        _ => CliCommand::Help,
    }
}

fn parse_sample_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("inspect") => CliCommand::SampleInspect(parse_sample_inspect_args(args)),
        _ => CliCommand::Help,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransformEuclideanArgs {
    input_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    pattern: usize,
    track: usize,
    steps: usize,
    pulses: usize,
    rotation: usize,
    pitch: u8,
    velocity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SampleInspectFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleInspectArgs {
    path: Option<PathBuf>,
    format: SampleInspectFormat,
    buckets: usize,
}

impl Default for SampleInspectArgs {
    fn default() -> Self {
        Self {
            path: None,
            format: SampleInspectFormat::Text,
            buckets: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SampleInspection {
    sample: Sample,
    overview: WaveformOverview,
}

impl Default for TransformEuclideanArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
            track: 1,
            steps: 16,
            pulses: 4,
            rotation: 0,
            pitch: 36,
            velocity: 100,
        }
    }
}

fn parse_transform_euclidean_args(
    args: impl IntoIterator<Item = String>,
) -> TransformEuclideanArgs {
    let mut parsed = TransformEuclideanArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pattern" => parse_next_usize(&mut args, &mut parsed.pattern),
            "--track" => parse_next_usize(&mut args, &mut parsed.track),
            "--steps" => parse_next_usize(&mut args, &mut parsed.steps),
            "--pulses" => parse_next_usize(&mut args, &mut parsed.pulses),
            "--rotation" => parse_next_usize(&mut args, &mut parsed.rotation),
            "--pitch" => parse_next_u8(&mut args, &mut parsed.pitch),
            "--velocity" => parse_next_u8(&mut args, &mut parsed.velocity),
            _ if arg.starts_with("--pattern=") => {
                parse_usize_value(arg.trim_start_matches("--pattern="), &mut parsed.pattern);
            }
            _ if arg.starts_with("--track=") => {
                parse_usize_value(arg.trim_start_matches("--track="), &mut parsed.track);
            }
            _ if arg.starts_with("--steps=") => {
                parse_usize_value(arg.trim_start_matches("--steps="), &mut parsed.steps);
            }
            _ if arg.starts_with("--pulses=") => {
                parse_usize_value(arg.trim_start_matches("--pulses="), &mut parsed.pulses);
            }
            _ if arg.starts_with("--rotation=") => {
                parse_usize_value(arg.trim_start_matches("--rotation="), &mut parsed.rotation);
            }
            _ if arg.starts_with("--pitch=") => {
                parse_u8_value(arg.trim_start_matches("--pitch="), &mut parsed.pitch);
            }
            _ if arg.starts_with("--velocity=") => {
                parse_u8_value(arg.trim_start_matches("--velocity="), &mut parsed.velocity);
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }

    parsed
}

fn parse_sample_inspect_args(args: impl IntoIterator<Item = String>) -> SampleInspectArgs {
    let mut parsed = SampleInspectArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--format" => {
                if let Some(value) = args.next() {
                    parse_sample_format(&value, &mut parsed.format);
                }
            }
            "--buckets" | "--width" => parse_next_usize(&mut args, &mut parsed.buckets),
            _ if arg.starts_with("--format=") => {
                parse_sample_format(arg.trim_start_matches("--format="), &mut parsed.format);
            }
            _ if arg.starts_with("--buckets=") => {
                parse_usize_value(arg.trim_start_matches("--buckets="), &mut parsed.buckets);
            }
            _ if arg.starts_with("--width=") => {
                parse_usize_value(arg.trim_start_matches("--width="), &mut parsed.buckets);
            }
            _ if parsed.path.is_none() => parsed.path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }

    parsed.buckets = parsed.buckets.max(1);
    parsed
}

fn parse_sample_format(value: &str, target: &mut SampleInspectFormat) {
    match value {
        "text" => *target = SampleInspectFormat::Text,
        "json" => *target = SampleInspectFormat::Json,
        _ => {}
    }
}

fn parse_next_usize(args: &mut impl Iterator<Item = String>, target: &mut usize) {
    if let Some(value) = args.next() {
        parse_usize_value(&value, target);
    }
}

fn parse_usize_value(value: &str, target: &mut usize) {
    if let Ok(parsed) = value.parse::<usize>() {
        *target = parsed;
    }
}

fn parse_next_u8(args: &mut impl Iterator<Item = String>, target: &mut u8) {
    if let Some(value) = args.next() {
        parse_u8_value(&value, target);
    }
}

fn parse_u8_value(value: &str, target: &mut u8) {
    if let Ok(parsed) = value.parse::<u8>() {
        *target = parsed.min(127);
    }
}

fn run_transform_euclidean(args: &TransformEuclideanArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing transform input path")?;
    let output_path = args
        .output_path
        .as_deref()
        .context("missing transform output path")?;
    if args.pattern == 0 {
        anyhow::bail!("--pattern is 1-based and must be greater than zero");
    }
    if args.track == 0 {
        anyhow::bail!("--track is 1-based and must be greater than zero");
    }

    let mut song = load_project(input_path)?;
    let report = apply_euclidean(
        &mut song,
        args.pattern - 1,
        EuclideanRhythm {
            steps: args.steps,
            pulses: args.pulses,
            rotation: args.rotation,
            track: args.track - 1,
            pitch: args.pitch,
            velocity: args.velocity,
        },
    )?;
    save_project(output_path, &song)?;

    println!(
        "Applied Euclidean transform to {} cells and wrote {}",
        report.touched_cells.len(),
        output_path.display()
    );
    Ok(())
}

fn run_sample_inspect(args: &SampleInspectArgs) -> Result<()> {
    let inspection = inspect_sample(args)?;
    match args.format {
        SampleInspectFormat::Text => print!("{}", format_sample_inspection_text(&inspection)),
        SampleInspectFormat::Json => println!("{}", format_sample_inspection_json(&inspection)?),
    }
    Ok(())
}

fn inspect_sample(args: &SampleInspectArgs) -> Result<SampleInspection> {
    let path = args
        .path
        .as_deref()
        .context("missing sample path: usage is salieri sample inspect FILE")?;
    let sample = Sample::load_wav(path)
        .with_context(|| format!("failed to load sample {}", path.display()))?;
    let overview = sample.waveform_overview(args.buckets.max(1));

    Ok(SampleInspection { sample, overview })
}

fn format_sample_inspection_text(inspection: &SampleInspection) -> String {
    let sample = &inspection.sample;
    let overview = &inspection.overview;
    let waveform = compact_waveform_text(&overview.buckets);

    format!(
        "sample: {}\nsample_rate: {}\nchannels: {}\nframes: {}\nduration_seconds: {:.6}\nwaveform_buckets: {}\nwaveform: {}\n",
        sample.name,
        overview.sample_rate,
        overview.channels,
        overview.frames,
        overview.duration_seconds,
        overview.buckets.len(),
        waveform
    )
}

fn format_sample_inspection_json(inspection: &SampleInspection) -> Result<String> {
    let overview = &inspection.overview;
    let buckets = overview
        .buckets
        .iter()
        .map(|bucket| serde_json::json!({ "min": bucket.min, "max": bucket.max }))
        .collect::<Vec<_>>();
    let output = serde_json::json!({
        "schema_version": 1,
        "sample": {
            "name": inspection.sample.name,
            "sample_rate": overview.sample_rate,
            "channels": overview.channels,
            "frames": overview.frames,
            "duration_seconds": overview.duration_seconds,
        },
        "waveform": {
            "bucket_count": overview.buckets.len(),
            "buckets": buckets,
        }
    });

    serde_json::to_string_pretty(&output).context("failed to encode sample inspection JSON")
}

fn compact_waveform_text(buckets: &[WaveformBucket]) -> String {
    const GLYPHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if buckets.is_empty() {
        return "<empty>".to_string();
    }

    buckets
        .iter()
        .map(|bucket| {
            let amplitude = bucket.min.abs().max(bucket.max.abs()).clamp(0.0, 1.0);
            let index = (amplitude * (GLYPHS.len() - 1) as f32).round() as usize;
            GLYPHS[index]
        })
        .collect()
}

fn run_external_sample_browser(
    config: &SampleBrowserConfig,
    request: &SampleBrowserRequest,
) -> Result<Option<PathBuf>> {
    let command_template = config
        .chooser_command
        .as_deref()
        .filter(|command| !command.trim().is_empty())
        .context("sample browser chooser_command is not configured")?;
    let chooser_file = temporary_chooser_file();
    let start_dir = request
        .start_dir
        .as_deref()
        .or(config.start_dir.as_deref())
        .unwrap_or_else(|| Path::new("."));
    let command = command_template
        .replace("{chooser_file}", &shell_quote(&chooser_file))
        .replace("{start_dir}", &shell_quote(start_dir));
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let status = ProcessCommand::new(shell)
        .arg("-lc")
        .arg(command)
        .env("SALIERI_CHOOSER_FILE", &chooser_file)
        .env("SALIERI_SAMPLE_START_DIR", start_dir)
        .status()
        .context("failed to launch sample browser")?;

    if !status.success() {
        let _ = std::fs::remove_file(&chooser_file);
        anyhow::bail!("sample browser exited with {status}");
    }

    let selected = std::fs::read_to_string(&chooser_file).unwrap_or_default();
    let _ = std::fs::remove_file(&chooser_file);
    let selected = selected.trim();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(selected)))
    }
}

fn temporary_chooser_file() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!(
        "salieri-sample-chooser-{}-{timestamp}.txt",
        std::process::id()
    ))
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn print_midi_outputs() -> Result<()> {
    let ports = match list_output_ports() {
        Ok(ports) => ports,
        Err(error) => {
            println!("MIDI output unavailable: {error}");
            return Ok(());
        }
    };
    if ports.is_empty() {
        println!("No MIDI output ports found");
        return Ok(());
    }

    for port in ports {
        println!("{}: {}", port.index, port.name);
    }

    Ok(())
}

fn run_midi_test(config: &AppConfig, args: &MidiTestArgs) -> Result<()> {
    let ports = list_output_ports().context("failed to list MIDI output ports")?;
    let output = args
        .output
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(config.midi.default_output.as_str());
    let Some((_, port)) = resolve_midi_output_port(&ports, output) else {
        anyhow::bail!("MIDI output not found: {output}");
    };

    let channel = args.channel.clamp(1, 16);
    let note = args.note.min(127);
    let duration = Duration::from_millis(args.duration_ms.max(1));
    let mut output = MidirMidiOutput::connect(port.index, "salieri-midi-test")
        .with_context(|| format!("failed to connect MIDI output {}", port.name))?;

    println!(
        "Sending MIDI test note: port {} ({}) channel {} note {} duration {}ms",
        port.index,
        port.name,
        channel,
        note,
        duration.as_millis()
    );

    send_logged_midi_message(
        &mut output,
        MidiMessage::note_on(channel, note, DEFAULT_NOTE_VELOCITY),
        config.midi.log_file.as_deref(),
    )?;
    thread::sleep(duration);
    send_logged_midi_message(
        &mut output,
        MidiMessage::note_off(channel, note, 0),
        config.midi.log_file.as_deref(),
    )?;
    thread::sleep(Duration::from_millis(20));

    println!("MIDI test complete");
    Ok(())
}

fn send_logged_midi_message(
    output: &mut impl MidiOutput,
    message: MidiMessage,
    log_file: Option<&Path>,
) -> Result<()> {
    output.send(message)?;
    if let Some(log_file) = log_file {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .with_context(|| format!("failed to open MIDI log {}", log_file.display()))?;
        let bytes = message.to_bytes();
        writeln!(
            file,
            "TEST {:?} bytes={:02X} {:02X} {:02X}",
            message, bytes[0], bytes[1], bytes[2]
        )
        .with_context(|| format!("failed to write MIDI log {}", log_file.display()))?;
    }
    Ok(())
}

#[derive(Debug)]
struct App {
    song: Song,
    clean_song: Song,
    project_path: Option<PathBuf>,
    pattern_index: usize,
    cursor: Cursor,
    row_offset: usize,
    mode: AppMode,
    octave: u8,
    edit_step: usize,
    vim_navigation: bool,
    pending_goto_start: bool,
    follow_playhead: bool,
    show_line_numbers_hex: bool,
    command_buffer: String,
    clipboard: Option<Clipboard>,
    selection_anchor: Option<SelectionAnchor>,
    undo_stack: Vec<Song>,
    redo_stack: Vec<Song>,
    playback: PlaybackRuntime,
    is_playing: bool,
    loop_pattern: bool,
    playhead_row: Option<usize>,
    sequence_position: Option<usize>,
    sequence_cursor: usize,
    midi_status: String,
    midi_ports: Vec<MidiOutputPort>,
    midi_port_cursor: usize,
    sample_view: Option<AppSampleView>,
    sample_browser: SampleBrowserConfig,
    pending_sample_browser: Option<SampleBrowserRequest>,
    dirty: bool,
    should_quit: bool,
    dialog: Option<Dialog>,
    notification: Option<Notification>,
    last_tick: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Dialog {
    QuitDirty,
    DeleteTrack {
        track_index: usize,
        message: String,
    },
    DeletePattern {
        pattern_index: usize,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Notification {
    kind: NotificationKind,
    message: String,
    expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq)]
struct AppSampleView {
    source_path: PathBuf,
    sample: Sample,
    overview: WaveformOverview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleBrowserRequest {
    start_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Clipboard {
    Cell(PatternCell),
    Region(ClipboardRegion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClipboardRegion {
    cells: Vec<Vec<PatternCell>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectionAnchor {
    row: usize,
    track: usize,
}

impl Default for App {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}

impl App {
    fn new(config: AppConfig) -> Self {
        let song = Song::empty();
        let default_midi_output = config.midi.default_output.trim().to_string();
        let midi_status = if default_midi_output.is_empty() {
            "MIDI Disconnected".to_string()
        } else {
            format!("MIDI Disconnected ({default_midi_output})")
        };
        let mut app = Self {
            clean_song: song.clone(),
            song,
            project_path: None,
            pattern_index: 0,
            cursor: Cursor::new(),
            row_offset: 0,
            mode: AppMode::Normal,
            octave: config.keyboard.default_octave,
            edit_step: config.keyboard.edit_step.max(1),
            vim_navigation: config.keyboard.vim_navigation,
            pending_goto_start: false,
            follow_playhead: config.ui.follow_playhead,
            show_line_numbers_hex: config.ui.show_line_numbers_hex,
            command_buffer: String::new(),
            clipboard: None,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            playback: PlaybackRuntime::spawn(config.midi.log_file.clone()),
            is_playing: false,
            loop_pattern: true,
            playhead_row: None,
            sequence_position: None,
            sequence_cursor: 0,
            midi_status,
            midi_ports: Vec::new(),
            midi_port_cursor: 0,
            sample_view: None,
            sample_browser: config.sample_browser.clone(),
            pending_sample_browser: None,
            dirty: false,
            should_quit: false,
            dialog: None,
            notification: None,
            last_tick: Instant::now(),
        };
        app.connect_default_midi_output(&default_midi_output);
        app
    }

    fn from_file(path: &Path, config: AppConfig) -> Result<Self> {
        let song = load_project(path)?;
        Ok(Self {
            clean_song: song.clone(),
            song,
            project_path: Some(path.to_path_buf()),
            ..Self::new(config)
        })
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.handle_control_key(key) {
            return;
        }

        match self.mode {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::Edit => self.handle_edit_key(key),
            AppMode::Command => self.handle_command_key(key),
            AppMode::Help => self.handle_help_key(key),
            AppMode::Dialog => self.handle_dialog_key(key),
            AppMode::MidiSettings => self.handle_midi_settings_key(key),
            AppMode::Sequence => self.handle_sequence_key(key),
            AppMode::Tracks => self.handle_tracks_key(key),
            AppMode::Patterns => self.handle_patterns_key(key),
            AppMode::Sampler => self.handle_sampler_key(key),
        }
    }

    fn handle_control_key(&mut self, key: KeyEvent) -> bool {
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
            return false;
        }

        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.start_save_as_command();
                true
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if let Err(error) = self.save() {
                    tracing::error!(?error, "failed to save project");
                    self.notify_error(format!("Save failed: {error}"));
                }
                true
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.create_track();
                true
            }
            KeyCode::Up => {
                self.adjust_bpm(1);
                true
            }
            KeyCode::Down => {
                self.adjust_bpm(-1);
                true
            }
            KeyCode::Right => {
                self.adjust_lpb(1);
                true
            }
            KeyCode::Left => {
                self.adjust_lpb(-1);
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.copy_selection_or_current_cell();
                true
            }
            KeyCode::Char('x') | KeyCode::Char('X') => {
                self.cut_selection_or_current_cell();
                true
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.paste_clipboard();
                true
            }
            KeyCode::Delete => {
                self.delete_current_row();
                true
            }
            KeyCode::Char('z') | KeyCode::Char('Z') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.redo();
                } else {
                    self.undo();
                }
                true
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.redo();
                true
            }
            KeyCode::Char('p') | KeyCode::Char('P')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.panic_midi();
                true
            }
            _ => true,
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        if self.pending_goto_start {
            self.pending_goto_start = false;
            if self.vim_navigation && key.code == KeyCode::Char('g') {
                self.cursor.row = 0;
                return;
            }
        }

        let direction = match key.code {
            KeyCode::Esc => {
                self.selection_anchor = None;
                return;
            }
            KeyCode::Char('q') => {
                self.request_quit(false);
                return;
            }
            KeyCode::Char(' ') if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.start_playback();
                return;
            }
            KeyCode::Char(' ') => {
                self.toggle_playback();
                return;
            }
            KeyCode::F(8) => {
                self.stop_playback();
                return;
            }
            KeyCode::F(1) => {
                self.decrement_octave();
                return;
            }
            KeyCode::F(2) => {
                self.increment_octave();
                return;
            }
            KeyCode::F(3) => {
                self.start_pattern_rename_command();
                return;
            }
            KeyCode::F(4) => {
                self.open_midi_settings();
                return;
            }
            KeyCode::F(7) => {
                self.open_sequence_view();
                return;
            }
            KeyCode::F(9) => {
                self.open_tracks_view();
                return;
            }
            KeyCode::F(10) => {
                self.open_patterns_view();
                return;
            }
            KeyCode::F(11) => {
                self.open_sampler_view();
                return;
            }
            KeyCode::F(6) => {
                self.start_pattern_length_command();
                return;
            }
            KeyCode::Char('r') => {
                self.start_track_rename_command();
                return;
            }
            KeyCode::Char('c') => {
                self.start_track_channel_command();
                return;
            }
            KeyCode::Char('D') => {
                self.duplicate_track(self.cursor.track);
                return;
            }
            KeyCode::Char('{') => {
                self.move_current_track_left();
                return;
            }
            KeyCode::Char('}') => {
                self.move_current_track_right();
                return;
            }
            KeyCode::Char('N') => {
                self.create_pattern();
                return;
            }
            KeyCode::Char('P') => {
                self.duplicate_current_pattern();
                return;
            }
            KeyCode::Char('X') => {
                self.request_delete_current_pattern();
                return;
            }
            KeyCode::Char('A') => {
                self.add_sequence_pattern(self.pattern_index);
                return;
            }
            KeyCode::Char(',') => {
                self.previous_sequence_position();
                return;
            }
            KeyCode::Char('.') => {
                self.next_sequence_position();
                return;
            }
            KeyCode::Char('Y') => {
                self.duplicate_selected_sequence_position();
                return;
            }
            KeyCode::Char('R') => {
                self.remove_selected_sequence_position();
                return;
            }
            KeyCode::Char('T') => {
                self.set_selected_sequence_to_current_pattern();
                return;
            }
            KeyCode::Char('<') => {
                self.move_selected_sequence_position_up();
                return;
            }
            KeyCode::Char('>') => {
                self.move_selected_sequence_position_down();
                return;
            }
            KeyCode::Char('L') => {
                self.toggle_loop();
                return;
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.start_sequence_playback_from_selected_position();
                return;
            }
            KeyCode::Enter => {
                self.start_playback_from_cursor();
                return;
            }
            KeyCode::Char('i') => {
                self.selection_anchor = None;
                self.mode = AppMode::Edit;
                return;
            }
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
                return;
            }
            KeyCode::Char('?') | KeyCode::Char('H') => {
                self.mode = AppMode::Help;
                return;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.start_selection();
                return;
            }
            KeyCode::Char('[') => {
                self.select_pattern(self.pattern_index.saturating_sub(1));
                return;
            }
            KeyCode::Char(']') => {
                self.select_pattern(self.pattern_index.saturating_add(1));
                return;
            }
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Char('k') if self.vim_navigation => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Char('j') if self.vim_navigation => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Char('h') if self.vim_navigation => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            KeyCode::Char('l') if self.vim_navigation => Some(Direction::Right),
            KeyCode::Tab => {
                self.next_track();
                return;
            }
            KeyCode::BackTab => {
                self.previous_track();
                return;
            }
            KeyCode::Home => {
                self.cursor.row = 0;
                return;
            }
            KeyCode::End => {
                self.cursor.row = self.current_row_count().saturating_sub(1);
                return;
            }
            KeyCode::Char('g') if self.vim_navigation => {
                self.pending_goto_start = true;
                return;
            }
            KeyCode::Char('G') if self.vim_navigation => {
                self.cursor.row = self.current_row_count().saturating_sub(1);
                return;
            }
            KeyCode::PageUp => {
                self.page_cursor_up();
                return;
            }
            KeyCode::PageDown => {
                self.page_cursor_down();
                return;
            }
            KeyCode::Insert => {
                self.insert_current_row();
                return;
            }
            KeyCode::Delete => {
                if self.selection_anchor.is_some() {
                    self.clear_selection_region();
                } else {
                    self.request_delete_current_track();
                }
                return;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.toggle_current_mute();
                return;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.toggle_current_solo();
                return;
            }
            _ => None,
        };

        if let Some(direction) = direction {
            self.move_cursor(direction);
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Up => self.move_cursor(Direction::Up),
            KeyCode::Down => self.move_cursor(Direction::Down),
            KeyCode::Left => self.move_cursor(Direction::Left),
            KeyCode::Right => self.move_cursor(Direction::Right),
            KeyCode::Tab => self.next_track(),
            KeyCode::BackTab => self.previous_track(),
            KeyCode::Home => self.cursor.row = 0,
            KeyCode::End => self.cursor.row = self.current_row_count().saturating_sub(1),
            KeyCode::PageUp => self.page_cursor_up(),
            KeyCode::PageDown => self.page_cursor_down(),
            KeyCode::Insert => self.insert_current_row(),
            KeyCode::Delete | KeyCode::Backspace => self.clear_current_cell(),
            KeyCode::F(1) | KeyCode::Char('-') => self.decrement_octave(),
            KeyCode::F(2) | KeyCode::Char('+') | KeyCode::Char('=') => self.increment_octave(),
            KeyCode::Char('o') | KeyCode::Char('O') => self.insert_note_event(NoteEvent::NoteOff),
            KeyCode::Char('.') => self.insert_note_event(NoteEvent::NoteCut),
            KeyCode::Char(value) if self.cursor.field == CellField::Velocity => {
                if let Some(hex) = value.to_digit(16) {
                    self.enter_velocity_digit(hex as u8);
                }
            }
            KeyCode::Char(value) => {
                if let Some(note) = keyboard_note(value, self.octave) {
                    self.insert_note(note);
                }
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_dialog_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => match self.dialog.clone() {
                Some(Dialog::QuitDirty) => {
                    if let Err(error) = self.save() {
                        tracing::error!(?error, "failed to save project");
                        self.notify_error(format!("Save failed: {error}"));
                    } else {
                        self.force_quit();
                    }
                }
                Some(Dialog::DeleteTrack { track_index, .. }) => {
                    self.dialog = None;
                    self.mode = AppMode::Normal;
                    self.delete_track(track_index);
                }
                Some(Dialog::DeletePattern { pattern_index, .. }) => {
                    self.dialog = None;
                    self.mode = AppMode::Normal;
                    self.delete_pattern(pattern_index);
                }
                None => self.mode = AppMode::Normal,
            },
            KeyCode::Char('n') | KeyCode::Char('N') => match self.dialog {
                Some(Dialog::QuitDirty) => self.force_quit(),
                Some(Dialog::DeleteTrack { .. } | Dialog::DeletePattern { .. }) | None => {
                    self.cancel_dialog();
                }
            },
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
                self.cancel_dialog();
            }
            _ => {}
        }
    }

    fn handle_midi_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = AppMode::Normal,
            KeyCode::Up => self.previous_midi_port(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_midi_port(),
            KeyCode::Down => self.next_midi_port(),
            KeyCode::Char('j') if self.vim_navigation => self.next_midi_port(),
            KeyCode::Home => self.midi_port_cursor = 0,
            KeyCode::End => {
                self.midi_port_cursor = self.midi_ports.len().saturating_sub(1);
            }
            KeyCode::Enter => self.connect_selected_midi_port(),
            KeyCode::Char('d') | KeyCode::Char('D') => self.disconnect_midi(),
            KeyCode::Char('p') | KeyCode::Char('P') => self.panic_midi(),
            KeyCode::F(5) | KeyCode::Char('r') | KeyCode::Char('R') => self.refresh_midi_ports(),
            _ => {}
        }
    }

    fn handle_sequence_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.mode = AppMode::Help,
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
            }
            KeyCode::Char(' ') => self.toggle_playback(),
            KeyCode::F(8) => self.stop_playback(),
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(7) => self.mode = AppMode::Normal,
            KeyCode::Up => self.previous_sequence_position(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_sequence_position(),
            KeyCode::Down => self.next_sequence_position(),
            KeyCode::Char('j') if self.vim_navigation => self.next_sequence_position(),
            KeyCode::Home => {
                self.sequence_cursor = 0;
                self.notify_info("Sequence position 00");
            }
            KeyCode::End => {
                self.sequence_cursor = self.song.sequence.len().saturating_sub(1);
                self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
            }
            KeyCode::Char('A') => self.add_sequence_pattern(self.pattern_index),
            KeyCode::Char('Y') => self.duplicate_selected_sequence_position(),
            KeyCode::Char('R') => self.remove_selected_sequence_position(),
            KeyCode::Char('T') => self.set_selected_sequence_to_current_pattern(),
            KeyCode::Char('<') => self.move_selected_sequence_position_up(),
            KeyCode::Char('>') => self.move_selected_sequence_position_down(),
            KeyCode::Enter => self.start_sequence_playback_from_selected_position(),
            _ => {}
        }
    }

    fn handle_tracks_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.mode = AppMode::Help,
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
            }
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(9) => self.mode = AppMode::Normal,
            KeyCode::Up => self.previous_track(),
            KeyCode::Char('k') if self.vim_navigation => self.previous_track(),
            KeyCode::Down => self.next_track(),
            KeyCode::Char('j') if self.vim_navigation => self.next_track(),
            KeyCode::Home => self.cursor.track = 0,
            KeyCode::End => self.cursor.track = self.song.tracks.len().saturating_sub(1),
            KeyCode::Char('N') => self.create_track(),
            KeyCode::Char('D') => self.duplicate_track(self.cursor.track),
            KeyCode::Char('r') => self.start_track_rename_command(),
            KeyCode::Char('c') => self.start_track_channel_command(),
            KeyCode::Delete => self.request_delete_current_track(),
            KeyCode::Char('{') => self.move_current_track_left(),
            KeyCode::Char('}') => self.move_current_track_right(),
            KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_current_mute(),
            KeyCode::Char('s') | KeyCode::Char('S') => self.toggle_current_solo(),
            _ => {}
        }
    }

    fn handle_patterns_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.mode = AppMode::Help,
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
            }
            KeyCode::F(10) => self.mode = AppMode::Normal,
            KeyCode::Up => self.select_pattern(self.pattern_index.saturating_sub(1)),
            KeyCode::Char('k') if self.vim_navigation => {
                self.select_pattern(self.pattern_index.saturating_sub(1));
            }
            KeyCode::Down => self.select_pattern(self.pattern_index.saturating_add(1)),
            KeyCode::Char('j') if self.vim_navigation => {
                self.select_pattern(self.pattern_index.saturating_add(1));
            }
            KeyCode::Home => self.select_pattern(0),
            KeyCode::End => self.select_pattern(self.song.patterns.len().saturating_sub(1)),
            KeyCode::Char('N') => self.create_pattern(),
            KeyCode::Char('P') => self.duplicate_current_pattern(),
            KeyCode::Char('X') | KeyCode::Delete => self.request_delete_current_pattern(),
            KeyCode::Char('r') => self.start_pattern_rename_command(),
            KeyCode::F(6) => self.start_pattern_length_command(),
            KeyCode::Char('1') => self.resize_current_pattern(16),
            KeyCode::Char('2') => self.resize_current_pattern(32),
            KeyCode::Char('3') => self.resize_current_pattern(64),
            KeyCode::Char('4') => self.resize_current_pattern(128),
            KeyCode::Char('5') => self.resize_current_pattern(256),
            KeyCode::Enter => self.mode = AppMode::Normal,
            _ => {}
        }
    }

    fn handle_sampler_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.mode = AppMode::Normal,
            KeyCode::Char('q') => self.request_quit(false),
            KeyCode::Char('?') | KeyCode::Char('H') => self.mode = AppMode::Help,
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
            }
            KeyCode::F(4) => self.open_midi_settings(),
            KeyCode::F(7) => self.open_sequence_view(),
            KeyCode::F(9) => self.open_tracks_view(),
            KeyCode::F(10) => self.open_patterns_view(),
            KeyCode::F(11) => self.mode = AppMode::Normal,
            KeyCode::F(8) => self.stop_playback(),
            KeyCode::Char(' ') => self.toggle_playback(),
            _ => {}
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_buffer.clear();
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => self.execute_command(),
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(value) => self.command_buffer.push(value),
            _ => {}
        }
    }

    fn move_cursor(&mut self, direction: Direction) {
        let row_count = self.current_row_count();
        let track_count = self.song.tracks.len();
        self.cursor.move_in(direction, row_count, track_count);
    }

    fn next_track(&mut self) {
        if self.song.tracks.is_empty() {
            return;
        }
        self.cursor.track = self
            .cursor
            .track
            .saturating_add(1)
            .min(self.song.tracks.len().saturating_sub(1));
        self.cursor.digit = 0;
    }

    fn previous_track(&mut self) {
        self.cursor.track = self.cursor.track.saturating_sub(1);
        self.cursor.digit = 0;
    }

    fn page_cursor_up(&mut self) {
        self.cursor.row = self.cursor.row.saturating_sub(16);
    }

    fn page_cursor_down(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(16)
            .min(self.current_row_count().saturating_sub(1));
    }

    fn insert_note(&mut self, pitch: u8) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.set_note(
                cursor.row,
                cursor.track,
                NoteEvent::Note { pitch },
                DEFAULT_NOTE_VELOCITY,
            );
        });
        self.advance_after_edit();
    }

    fn insert_note_event(&mut self, note: NoteEvent) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.set_note_event(cursor.row, cursor.track, note, None);
        });
        self.advance_after_edit();
    }

    fn enter_velocity_digit(&mut self, digit: u8) {
        let current_digit = self.cursor.digit.min(1);
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let current_velocity = pattern
                .cell(cursor.row, cursor.track)
                .and_then(|cell| cell.velocity)
                .unwrap_or(0);
            let next_velocity = if current_digit == 0 {
                (digit << 4) | (current_velocity & 0x0f)
            } else {
                (current_velocity & 0xf0) | digit
            };
            let _ = pattern.set_velocity(cursor.row, cursor.track, next_velocity);
        });

        if current_digit == 0 {
            self.cursor.digit = 1;
        } else {
            self.cursor.digit = 0;
            self.advance_after_edit();
        }
    }

    fn clear_current_cell(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            let _ = pattern.clear_cell(cursor.row, cursor.track);
        });
    }

    fn copy_current_cell(&mut self) {
        self.clipboard = self
            .song
            .pattern(self.pattern_index)
            .and_then(|pattern| pattern.cell(self.cursor.row, self.cursor.track))
            .cloned()
            .map(Clipboard::Cell);
    }

    fn cut_current_cell(&mut self) {
        self.copy_current_cell();
        self.clear_current_cell();
    }

    fn copy_selection_or_current_cell(&mut self) {
        if let Some(selection) = self.selection_rect() {
            self.copy_selection(selection);
        } else {
            self.copy_current_cell();
        }
    }

    fn cut_selection_or_current_cell(&mut self) {
        if self.selection_anchor.is_some() {
            if let Some(selection) = self.selection_rect() {
                self.copy_selection(selection);
                self.clear_region(selection);
                self.selection_anchor = None;
            }
        } else {
            self.cut_current_cell();
        }
    }

    fn paste_clipboard(&mut self) {
        let Some(clipboard) = self.clipboard.clone() else {
            return;
        };
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            match clipboard {
                Clipboard::Cell(cell) => {
                    let _ = pattern.set_cell(cursor.row, cursor.track, cell);
                }
                Clipboard::Region(region) => {
                    for (row_offset, row) in region.cells.iter().enumerate() {
                        for (track_offset, cell) in row.iter().enumerate() {
                            let _ = pattern.set_cell(
                                cursor.row.saturating_add(row_offset),
                                cursor.track.saturating_add(track_offset),
                                cell.clone(),
                            );
                        }
                    }
                }
            }
        });
    }

    fn start_selection(&mut self) {
        self.selection_anchor = Some(SelectionAnchor {
            row: self.cursor.row,
            track: self.cursor.track,
        });
    }

    fn selection_rect(&self) -> Option<SelectionRect> {
        let anchor = self.selection_anchor?;
        let row_count = self.current_row_count();
        let track_count = self.song.tracks.len();
        if row_count == 0 || track_count == 0 {
            return None;
        }

        let anchor_row = anchor.row.min(row_count.saturating_sub(1));
        let cursor_row = self.cursor.row.min(row_count.saturating_sub(1));
        let anchor_track = anchor.track.min(track_count.saturating_sub(1));
        let cursor_track = self.cursor.track.min(track_count.saturating_sub(1));

        Some(SelectionRect {
            row_start: anchor_row.min(cursor_row),
            row_end: anchor_row.max(cursor_row),
            track_start: anchor_track.min(cursor_track),
            track_end: anchor_track.max(cursor_track),
        })
    }

    fn copy_selection(&mut self, selection: SelectionRect) {
        let Some(pattern) = self.song.pattern(self.pattern_index) else {
            return;
        };
        let cells = (selection.row_start..=selection.row_end)
            .map(|row| {
                (selection.track_start..=selection.track_end)
                    .map(|track| pattern.cell(row, track).cloned().unwrap_or_default())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        self.clipboard = Some(Clipboard::Region(ClipboardRegion { cells }));
    }

    fn clear_selection_region(&mut self) {
        if let Some(selection) = self.selection_rect() {
            self.clear_region(selection);
            self.selection_anchor = None;
        }
    }

    fn clear_region(&mut self, selection: SelectionRect) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, _| {
            let Some(pattern) = song.pattern_mut(pattern_index) else {
                return;
            };
            for row in selection.row_start..=selection.row_end {
                for track in selection.track_start..=selection.track_end {
                    let _ = pattern.clear_cell(row, track);
                }
            }
        });
    }

    fn insert_current_row(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let _ = song.insert_pattern_row(pattern_index, cursor.row);
        });
    }

    fn delete_current_row(&mut self) {
        let pattern_index = self.pattern_index;
        self.mutate_song(|song, cursor| {
            let _ = song.delete_pattern_row(pattern_index, cursor.row);
        });
        self.clamp_cursor();
    }

    fn create_track(&mut self) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            song.create_track();
        });

        if self.song.tracks.len() > before_count {
            self.cursor.track = self.song.tracks.len().saturating_sub(1);
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
        }
    }

    fn request_delete_current_track(&mut self) {
        self.request_delete_track(self.cursor.track);
    }

    fn request_delete_track(&mut self, track_index: usize) {
        if self.song.tracks.len() <= 1 {
            self.notify_warning("Cannot delete the last track");
            return;
        }

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        self.dialog = Some(Dialog::DeleteTrack {
            track_index,
            message: format!("Delete track {:02} {}?", track_index + 1, track.name),
        });
        self.mode = AppMode::Dialog;
        self.notify_warning("Confirm track delete");
    }

    fn delete_track(&mut self, track: usize) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_track(track);
        });

        if self.song.tracks.len() < before_count {
            self.clamp_cursor();
            self.cursor.digit = 0;
            self.notify_success("Track deleted");
        }
    }

    fn duplicate_track(&mut self, track_index: usize) {
        let before_count = self.song.tracks.len();
        self.mutate_song(|song, _| {
            let _ = song.duplicate_track(track_index);
        });

        if self.song.tracks.len() > before_count {
            self.cursor.track = self.song.tracks.len().saturating_sub(1);
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
        }
    }

    fn move_track(&mut self, from: usize, to: usize) {
        let before = self.song.clone();
        self.mutate_song(|song, _| {
            let _ = song.move_track(from, to);
        });

        if self.song != before {
            self.cursor.track = to.min(self.song.tracks.len().saturating_sub(1));
            self.cursor.field = CellField::Note;
            self.cursor.digit = 0;
            self.notify_success("Track moved");
        }
    }

    fn move_current_track_left(&mut self) {
        if self.cursor.track == 0 {
            self.notify_warning("Track already at first position");
            return;
        }

        self.move_track(self.cursor.track, self.cursor.track - 1);
    }

    fn move_current_track_right(&mut self) {
        let next_track = self.cursor.track.saturating_add(1);
        if next_track >= self.song.tracks.len() {
            self.notify_warning("Track already at last position");
            return;
        }

        self.move_track(self.cursor.track, next_track);
    }

    fn toggle_current_mute(&mut self) {
        self.toggle_track_mute(self.cursor.track);
    }

    fn toggle_current_solo(&mut self) {
        self.toggle_track_solo(self.cursor.track);
    }

    fn toggle_track_mute(&mut self, track_index: usize) {
        if track_index >= self.song.tracks.len() {
            self.notify_warning("Track out of range");
            return;
        }

        self.mutate_song(|song, _| {
            let _ = song.toggle_mute(track_index);
        });
    }

    fn toggle_track_solo(&mut self, track_index: usize) {
        if track_index >= self.song.tracks.len() {
            self.notify_warning("Track out of range");
            return;
        }

        self.mutate_song(|song, _| {
            let _ = song.toggle_solo(track_index);
        });
    }

    fn set_track_midi_channel(&mut self, track_index: usize, midi_channel: u8) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_track_midi_channel(track_index, midi_channel) {
            self.notify_warning(format!("Track channel failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success(format!("Track channel set to {midi_channel}"));
    }

    fn rename_track(&mut self, track_index: usize, name: String) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.rename_track(track_index, name) {
            self.notify_warning(format!("Track rename failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success("Track renamed");
    }

    fn start_track_rename_command(&mut self) {
        self.command_buffer = format!("track rename {} ", self.cursor.track + 1);
        self.mode = AppMode::Command;
        self.notify_info("Rename current track");
    }

    fn start_track_channel_command(&mut self) {
        self.command_buffer = format!("track channel {} ", self.cursor.track + 1);
        self.mode = AppMode::Command;
        self.notify_info("Set current track MIDI channel");
    }

    fn set_bpm(&mut self, bpm: u16) {
        self.mutate_song(|song, _| {
            song.transport.bpm = bpm;
        });
    }

    fn adjust_bpm(&mut self, delta: i16) {
        let bpm = (i32::from(self.song.transport.bpm) + i32::from(delta))
            .clamp(i32::from(MIN_BPM), i32::from(MAX_BPM)) as u16;
        self.set_bpm(bpm);
        self.notify_info(format!("BPM {bpm}"));
    }

    fn set_lpb(&mut self, lpb: u8) {
        self.mutate_song(|song, _| {
            song.transport.lines_per_beat = lpb;
        });
    }

    fn adjust_lpb(&mut self, delta: i8) {
        let lpb = (i16::from(self.song.transport.lines_per_beat) + i16::from(delta))
            .clamp(i16::from(MIN_LPB), i16::from(MAX_LPB)) as u8;
        self.set_lpb(lpb);
        self.notify_info(format!("LPB {lpb}"));
    }

    fn create_pattern(&mut self) {
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            song.create_pattern(64);
        });
        if self.song.patterns.len() > before_count {
            self.pattern_index = self.song.patterns.len().saturating_sub(1);
            self.cursor.row = 0;
            self.row_offset = 0;
        }
    }

    fn duplicate_current_pattern(&mut self) {
        let pattern_index = self.pattern_index;
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.duplicate_pattern(pattern_index);
        });
        if self.song.patterns.len() > before_count {
            self.pattern_index = self.song.patterns.len().saturating_sub(1);
            self.cursor.row = 0;
            self.row_offset = 0;
        }
    }

    fn request_delete_current_pattern(&mut self) {
        if self.song.patterns.len() <= 1 {
            self.notify_warning("Cannot delete the last pattern");
            return;
        }

        let pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
        let Some(pattern) = self.song.patterns.get(pattern_index) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        self.dialog = Some(Dialog::DeletePattern {
            pattern_index,
            message: format!("Delete pattern {:02} {}?", pattern_index + 1, pattern.name),
        });
        self.mode = AppMode::Dialog;
        self.notify_warning("Confirm pattern delete");
    }

    fn delete_pattern(&mut self, pattern_index: usize) {
        let before_count = self.song.patterns.len();
        self.mutate_song(|song, _| {
            let _ = song.delete_pattern(pattern_index);
        });
        if self.song.patterns.len() < before_count {
            self.pattern_index = self
                .pattern_index
                .min(self.song.patterns.len().saturating_sub(1));
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.row_offset = 0;
            self.notify_success("Pattern deleted");
        }
    }

    fn resize_current_pattern(&mut self, row_count: usize) {
        let pattern_index = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.resize_pattern(pattern_index, row_count) {
            self.notify_warning(format!("Pattern length failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.clamp_cursor();
        self.keep_cursor_visible(1);
        self.notify_success(format!("Pattern length set to {row_count}"));
    }

    fn start_pattern_length_command(&mut self) {
        self.command_buffer = "pattern length ".to_string();
        self.mode = AppMode::Command;
        self.notify_info("Set current pattern length");
    }

    fn rename_current_pattern(&mut self, name: String) {
        let pattern_index = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.rename_pattern(pattern_index, name) {
            self.notify_warning(format!("Pattern rename failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success("Pattern renamed");
    }

    fn start_pattern_rename_command(&mut self) {
        self.command_buffer = "pattern rename ".to_string();
        self.mode = AppMode::Command;
        self.notify_info("Rename current pattern");
    }

    fn select_pattern(&mut self, pattern_index: usize) {
        if pattern_index < self.song.patterns.len() {
            self.pattern_index = pattern_index;
            self.clamp_cursor();
            self.row_offset = 0;
        }
    }

    fn selected_sequence_position(&mut self) -> Option<usize> {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return None;
        }

        self.clamp_sequence_cursor();
        Some(self.sequence_cursor)
    }

    fn previous_sequence_position(&mut self) {
        self.sequence_cursor = self.sequence_cursor.saturating_sub(1);
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    fn next_sequence_position(&mut self) {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return;
        }

        self.sequence_cursor = self
            .sequence_cursor
            .saturating_add(1)
            .min(self.song.sequence.len().saturating_sub(1));
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    fn add_sequence_pattern(&mut self, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        let before_len = self.song.sequence.len();
        self.mutate_song(|song, _| {
            let _ = song.push_sequence_pattern(pattern_id);
        });
        if self.song.sequence.len() > before_len {
            self.sequence_cursor = self.song.sequence.len().saturating_sub(1);
        }
        self.notify_success(format!("Sequence added pattern {:02}", pattern_index + 1));
    }

    fn remove_sequence_position(&mut self, position: usize) {
        let before_len = self.song.sequence.len();
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.remove_sequence_position(position) {
            self.notify_warning(format!("Sequence remove failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        if self.song.sequence.len() < before_len {
            self.sequence_cursor = position.min(self.song.sequence.len().saturating_sub(1));
        }
        self.clamp_sequence_cursor();
        self.notify_success(format!("Sequence removed position {position:02}"));
    }

    fn duplicate_sequence_position(&mut self, position: usize) {
        let before_len = self.song.sequence.len();
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.duplicate_sequence_position(position) {
            self.notify_warning(format!("Sequence duplicate failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        if self.song.sequence.len() > before_len {
            self.sequence_cursor = position.saturating_add(1);
            self.clamp_sequence_cursor();
        }
        self.notify_success(format!("Sequence duplicated position {position:02}"));
    }

    fn duplicate_selected_sequence_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.duplicate_sequence_position(position);
        }
    }

    fn remove_selected_sequence_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.remove_sequence_position(position);
        }
    }

    fn set_sequence_pattern(&mut self, position: usize, pattern_index: usize) {
        let Some(pattern_id) = self.song.pattern(pattern_index).map(|pattern| pattern.id) else {
            self.notify_warning("Pattern out of range");
            return;
        };
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_sequence_pattern(position, pattern_id) {
            self.notify_warning(format!("Sequence set failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.notify_success(format!(
            "Sequence position {position:02} set to pattern {:02}",
            pattern_index + 1
        ));
    }

    fn set_selected_sequence_to_current_pattern(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.set_sequence_pattern(position, self.pattern_index);
        }
    }

    fn move_sequence_position(&mut self, from: usize, to: usize) {
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.move_sequence_position(from, to) {
            self.notify_warning(format!("Sequence move failed: {error}"));
            return;
        }

        self.mutate_song(|song, _| {
            *song = next_song;
        });
        self.sequence_cursor = to;
        self.notify_success(format!("Sequence moved position {from:02} to {to:02}"));
    }

    fn move_selected_sequence_position_up(&mut self) {
        let Some(position) = self.selected_sequence_position() else {
            return;
        };
        if position == 0 {
            self.notify_warning("Sequence already at first position");
            return;
        }
        self.move_sequence_position(position, position - 1);
    }

    fn move_selected_sequence_position_down(&mut self) {
        let Some(position) = self.selected_sequence_position() else {
            return;
        };
        let next_position = position.saturating_add(1);
        if next_position >= self.song.sequence.len() {
            self.notify_warning("Sequence already at last position");
            return;
        }
        self.move_sequence_position(position, next_position);
    }

    fn handle_fx_command(&mut self, values: &[&str]) {
        match values {
            ["clear"] | ["off"] | ["none"] => {
                self.set_current_fx(None);
                self.notify_success("Effect cleared");
            }
            [packed] if packed.len() >= 2 => {
                let mut chars = packed.chars();
                let Some(code) = chars.next() else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                    return;
                };
                let value = chars.collect::<String>();
                if let Some(value) = parse_hex_byte(&value) {
                    self.set_current_fx(Some(TrackerCommand::from_code_char(code, value)));
                    self.notify_success(format!("Effect {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                }
            }
            [code, value] => {
                let Some(code) = code.chars().next() else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                    return;
                };
                if let Some(value) = parse_hex_byte(value) {
                    self.set_current_fx(Some(TrackerCommand::from_code_char(code, value)));
                    self.notify_success(format!("Effect {}{value:02X}", code.to_ascii_uppercase()));
                } else {
                    self.notify_warning("Usage: :fx CODE VALUE");
                }
            }
            _ => self.notify_warning("Usage: :fx CODE VALUE or :fx clear"),
        }
    }

    fn set_current_fx(&mut self, command: Option<TrackerCommand>) {
        self.mutate_song(|song, cursor| {
            if let Some(pattern) = song.current_pattern_mut() {
                if let Some(cell) = pattern.cell_mut(cursor.row, cursor.track) {
                    cell.command = command;
                }
            }
        });
    }

    fn execute_command(&mut self) {
        let command = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = AppMode::Normal;

        let mut parts = command.split_whitespace();
        let Some(name) = parts.next() else {
            return;
        };

        match name {
            "h" | "help" => {
                self.mode = AppMode::Help;
            }
            "q" | "quit" => {
                self.request_quit(false);
            }
            "q!" | "quit!" => {
                self.force_quit();
            }
            "w" | "write" | "save" => {
                let path = parts.collect::<Vec<_>>().join(" ");
                let result = if path.is_empty() {
                    self.save()
                } else {
                    self.save_as(PathBuf::from(path))
                };
                if let Err(error) = result {
                    tracing::error!(?error, "failed to save project");
                    self.notify_error(format!("Save failed: {error}"));
                }
            }
            "saveas" | "writeas" => {
                let path = parts.collect::<Vec<_>>().join(" ");
                if !path.is_empty() {
                    if let Err(error) = self.save_as(PathBuf::from(path)) {
                        tracing::error!(?error, "failed to save project");
                        self.notify_error(format!("Save failed: {error}"));
                    }
                } else {
                    self.notify_warning("Usage: :saveas PATH");
                }
            }
            "wq" => {
                if let Err(error) = self.save() {
                    tracing::error!(?error, "failed to save project");
                    self.notify_error(format!("Save failed: {error}"));
                    return;
                }
                self.stop_playback();
                self.should_quit = true;
            }
            "bpm" => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u16>().ok()) {
                    self.set_bpm(value);
                    self.notify_success(format!("BPM set to {value}"));
                } else {
                    self.notify_warning("Usage: :bpm 140");
                }
            }
            "lpb" => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u8>().ok()) {
                    self.set_lpb(value);
                    self.notify_success(format!("LPB set to {value}"));
                } else {
                    self.notify_warning("Usage: :lpb 4");
                }
            }
            "fx" | "effect" => {
                let values = parts.collect::<Vec<_>>();
                self.handle_fx_command(&values);
            }
            "loop" => match parts.next() {
                Some("on") => {
                    self.loop_pattern = true;
                    self.notify_info("Pattern loop ON");
                }
                Some("off") => {
                    self.loop_pattern = false;
                    self.notify_info("Pattern loop OFF");
                }
                Some("toggle") | None => self.toggle_loop(),
                Some(_) => self.notify_warning("Usage: :loop [on|off|toggle]"),
            },
            "midi" => match parts.next() {
                Some("outputs") | Some("settings") | Some("ports") => self.open_midi_settings(),
                Some("connect") => {
                    if let Some(port_index) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.connect_midi(port_index);
                    } else {
                        self.notify_warning("Usage: :midi connect PORT_INDEX");
                    }
                }
                Some("disconnect") => self.disconnect_midi(),
                Some("panic") => self.panic_midi(),
                None | Some(_) => {
                    self.notify_warning("Usage: :midi outputs|connect|disconnect|panic")
                }
            },
            "play" => match parts.next() {
                Some("sequence") | Some("seq") => {
                    let start_sequence_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0);
                    self.start_sequence_playback_at(start_sequence_index);
                }
                Some("pattern") | Some("pat") | None => self.start_playback(),
                Some(_) => self.notify_warning("Usage: :play [pattern|sequence [position]]"),
            },
            "stop" => self.stop_playback(),
            "track" => match parts.next() {
                Some("new") => self.create_track(),
                Some("duplicate") | Some("dup") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.duplicate_track(track_index);
                }
                Some("delete") | Some("del") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.request_delete_track(track_index);
                }
                Some("move") | Some("mv") => {
                    let from = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    let to = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map(|value| value.saturating_sub(1));
                    if let Some(to) = to {
                        self.move_track(from, to);
                    } else {
                        self.notify_warning("Usage: :track move FROM TO");
                    }
                }
                Some("mute") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.toggle_track_mute(track_index);
                }
                Some("solo") => {
                    let track_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.cursor.track, |value| value.saturating_sub(1));
                    self.toggle_track_solo(track_index);
                }
                Some("rename") => {
                    let values = parts.collect::<Vec<_>>();
                    if let Some((track_index, name)) =
                        parse_optional_numbered_name(&values, self.cursor.track)
                    {
                        self.rename_track(track_index, name);
                    }
                }
                Some("channel") | Some("ch") => {
                    let first = parts.next().and_then(|value| value.parse::<u8>().ok());
                    let second = parts.next().and_then(|value| value.parse::<u8>().ok());
                    match (first, second) {
                        (Some(channel), None) => {
                            self.set_track_midi_channel(self.cursor.track, channel);
                        }
                        (Some(track_number), Some(channel)) => {
                            self.set_track_midi_channel(
                                usize::from(track_number.saturating_sub(1)),
                                channel,
                            );
                        }
                        _ => {}
                    }
                }
                None | Some(_) => self.notify_warning(
                    "Usage: :track new|duplicate|delete|move|mute|solo|rename|channel",
                ),
            },
            "pattern" => match parts.next() {
                Some("new") => self.create_pattern(),
                Some("duplicate") | Some("dup") => self.duplicate_current_pattern(),
                Some("delete") | Some("del") => self.request_delete_current_pattern(),
                Some("length") | Some("len") => {
                    if let Some(row_count) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.resize_current_pattern(row_count);
                    }
                }
                Some("rename") => {
                    let name = parts.collect::<Vec<_>>().join(" ");
                    self.rename_current_pattern(name);
                }
                Some("next") => self.select_pattern(self.pattern_index.saturating_add(1)),
                Some("prev") => self.select_pattern(self.pattern_index.saturating_sub(1)),
                Some(value) => {
                    if let Ok(pattern_number) = value.parse::<usize>() {
                        self.select_pattern(pattern_number.saturating_sub(1));
                    }
                }
                None => {}
            },
            "sequence" | "seq" => match parts.next() {
                Some("add") => {
                    let pattern_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map_or(self.pattern_index, |value| value.saturating_sub(1));
                    self.add_sequence_pattern(pattern_index);
                }
                Some("remove") | Some("rm") => {
                    if let Some(position) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.remove_sequence_position(position);
                    }
                }
                Some("duplicate") | Some("dup") => {
                    if let Some(position) =
                        parts.next().and_then(|value| value.parse::<usize>().ok())
                    {
                        self.duplicate_sequence_position(position);
                    }
                }
                Some("set") => {
                    let position = parts.next().and_then(|value| value.parse::<usize>().ok());
                    let pattern_index = parts
                        .next()
                        .and_then(|value| value.parse::<usize>().ok())
                        .map(|value| value.saturating_sub(1));
                    if let (Some(position), Some(pattern_index)) = (position, pattern_index) {
                        self.set_sequence_pattern(position, pattern_index);
                    }
                }
                Some("move") | Some("mv") => {
                    let from = parts.next().and_then(|value| value.parse::<usize>().ok());
                    let to = parts.next().and_then(|value| value.parse::<usize>().ok());
                    if let (Some(from), Some(to)) = (from, to) {
                        self.move_sequence_position(from, to);
                    }
                }
                None | Some(_) => {
                    self.notify_warning("Usage: :sequence add|remove|duplicate|set|move")
                }
            },
            "sample" | "sampler" => match parts.next() {
                Some("view") | Some("inspect") | Some("load") => {
                    let path = parts.collect::<Vec<_>>().join(" ");
                    if path.is_empty() {
                        self.open_sampler_view();
                    } else {
                        self.load_sampler_view(PathBuf::from(path));
                    }
                }
                Some("browse") | Some("browser") | Some("choose") => {
                    let path = parts.collect::<Vec<_>>().join(" ");
                    self.request_sample_browser(if path.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(path))
                    });
                }
                Some("assign") => {
                    let track_index = parts
                        .next()
                        .and_then(parse_track_number)
                        .unwrap_or(self.cursor.track);
                    self.assign_loaded_sample_to_track(track_index);
                }
                Some("replace") | Some("swap") => {
                    let track_index = parts
                        .next()
                        .and_then(parse_track_number)
                        .unwrap_or(self.cursor.track);
                    self.replace_track_sample_with_loaded_sample(track_index);
                }
                Some("unassign") | Some("clear") => {
                    let track_index = parts
                        .next()
                        .and_then(parse_track_number)
                        .unwrap_or(self.cursor.track);
                    self.unassign_sample_from_track(track_index);
                }
                Some("unload") => {
                    self.unload_current_sample();
                }
                Some("cleanup") | Some("prune") => {
                    self.cleanup_unused_sample_references();
                }
                Some("assignments") | Some("assigned") | Some("list") => {
                    self.show_sample_assignments();
                }
                None => self.open_sampler_view(),
                Some(_) => self.notify_warning(
                    "Usage: :sample view PATH | browse [DIR] | assign [TRACK] | replace [TRACK] | unassign [TRACK] | unload | cleanup | assignments",
                ),
            },
            _ => self.notify_warning(format!("Unknown command: {name}")),
        }
    }

    fn request_quit(&mut self, force: bool) {
        if force || !self.dirty {
            self.force_quit();
        } else {
            self.stop_playback();
            self.mode = AppMode::Dialog;
            self.dialog = Some(Dialog::QuitDirty);
            self.notify_warning("Unsaved changes");
        }
    }

    fn force_quit(&mut self) {
        self.stop_playback();
        self.dialog = None;
        self.should_quit = true;
    }

    fn cancel_dialog(&mut self) {
        self.dialog = None;
        self.mode = AppMode::Normal;
        self.notify_info("Cancelled");
    }

    fn toggle_playback(&mut self) {
        if self.is_playing {
            self.stop_playback();
        } else {
            self.start_playback();
        }
    }

    fn toggle_loop(&mut self) {
        self.loop_pattern = !self.loop_pattern;
        let state = if self.loop_pattern { "ON" } else { "OFF" };
        self.notify_info(format!("Pattern loop {state}"));
    }

    fn start_playback(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = None;
        self.playback.start_pattern_from(
            self.song.clone(),
            self.pattern_index,
            0,
            self.loop_pattern,
        );
        self.notify_info("Playing pattern from start");
    }

    fn start_playback_from_cursor(&mut self) {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return;
        }

        self.is_playing = true;
        self.playhead_row = Some(self.cursor.row);
        self.sequence_position = None;
        self.playback.start_pattern_from(
            self.song.clone(),
            self.pattern_index,
            self.cursor.row,
            self.loop_pattern,
        );
        self.notify_info(format!("Playing pattern from row {:02}", self.cursor.row));
    }

    fn start_sequence_playback_at(&mut self, start_sequence_index: usize) {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return;
        }

        if start_sequence_index >= self.song.sequence.len() {
            self.notify_warning("Sequence position out of range");
            return;
        }

        if let Some(first_pattern_id) = self.song.sequence.get(start_sequence_index) {
            if let Some(pattern_index) = self
                .song
                .patterns
                .iter()
                .position(|pattern| pattern.id == *first_pattern_id)
            {
                self.pattern_index = pattern_index;
            }
        }

        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = Some(start_sequence_index);
        self.playback
            .start_sequence(self.song.clone(), start_sequence_index);
        self.notify_info(format!("Playing sequence from {start_sequence_index}"));
    }

    fn start_sequence_playback_from_selected_position(&mut self) {
        if let Some(position) = self.selected_sequence_position() {
            self.start_sequence_playback_at(position);
        }
    }

    fn stop_playback(&mut self) {
        self.playback.stop();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
        self.notify_info("Playback stopped");
    }

    fn connect_midi(&mut self, port_index: usize) {
        self.midi_status = format!("MIDI Connecting {port_index}");
        self.playback.connect_midi(port_index);
        self.notify_info(format!("Connecting MIDI output {port_index}"));
    }

    fn open_midi_settings(&mut self) {
        self.refresh_midi_ports();
        self.mode = AppMode::MidiSettings;
    }

    fn open_sequence_view(&mut self) {
        self.clamp_sequence_cursor();
        self.mode = AppMode::Sequence;
        self.notify_info(format!("Sequence position {:02}", self.sequence_cursor));
    }

    fn open_tracks_view(&mut self) {
        self.cursor.track = self
            .cursor
            .track
            .min(self.song.tracks.len().saturating_sub(1));
        self.mode = AppMode::Tracks;
        self.notify_info(format!("Track {:02}", self.cursor.track + 1));
    }

    fn open_patterns_view(&mut self) {
        self.clamp_pattern_index();
        self.mode = AppMode::Patterns;
        self.notify_info(format!("Pattern {:02}", self.pattern_index + 1));
    }

    fn open_sampler_view(&mut self) {
        self.mode = AppMode::Sampler;
        if let Some(sample) = &self.sample_view {
            self.notify_info(format!("Sample {}", sample.sample.name));
        } else {
            self.notify_info("Sampler view");
        }
    }

    fn load_sampler_view(&mut self, path: PathBuf) {
        match Sample::load_wav(&path) {
            Ok(sample) => {
                let overview = sample.waveform_overview(96);
                let name = sample.name.clone();
                self.sample_view = Some(AppSampleView {
                    source_path: path,
                    sample,
                    overview,
                });
                self.mode = AppMode::Sampler;
                self.notify_success(format!("Sample loaded: {name}"));
            }
            Err(error) => {
                self.mode = AppMode::Sampler;
                self.notify_error(format!("Sample load failed: {error}"));
            }
        }
    }

    fn request_sample_browser(&mut self, start_dir: Option<PathBuf>) {
        if self
            .sample_browser
            .chooser_command
            .as_deref()
            .is_none_or(str::is_empty)
        {
            self.mode = AppMode::Sampler;
            self.notify_warning("Sample browser not configured");
            return;
        }

        self.pending_sample_browser = Some(SampleBrowserRequest { start_dir });
        self.mode = AppMode::Sampler;
        self.notify_info("Opening sample browser");
    }

    fn take_sample_browser_request(
        &mut self,
    ) -> Option<(SampleBrowserConfig, SampleBrowserRequest)> {
        self.pending_sample_browser
            .take()
            .map(|request| (self.sample_browser.clone(), request))
    }

    fn finish_sample_browser(&mut self, result: Result<Option<PathBuf>>) {
        match result {
            Ok(Some(path)) => self.load_sampler_view(path),
            Ok(None) => {
                self.mode = AppMode::Sampler;
                self.notify_info("Sample browser closed");
            }
            Err(error) => {
                self.mode = AppMode::Sampler;
                self.notify_error(format!("Sample browser failed: {error}"));
            }
        }
    }

    fn assign_loaded_sample_to_track(&mut self, track_index: usize) {
        let Some(sample_view) = &self.sample_view else {
            self.mode = AppMode::Sampler;
            self.notify_warning("Load a sample before assigning it");
            return;
        };

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };

        let track_id = track.id;
        let track_name = track.name.clone();
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();

        self.mutate_song(|song, _| {
            let sample_id = song.upsert_sample_reference(sample_path, sample_name);
            let _ = song.assign_sample_to_track(track_id, sample_id);
        });
        self.mode = AppMode::Sampler;
        self.notify_success(format!("Sample assigned to {track_name}"));
    }

    fn unassign_sample_from_track(&mut self, track_index: usize) {
        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        let track_id = track.id;
        let track_name = track.name.clone();

        self.mutate_song(|song, _| {
            song.unassign_sample_from_track(track_id);
        });
        self.notify_success(format!("Sample unassigned from {track_name}"));
    }

    fn replace_track_sample_with_loaded_sample(&mut self, track_index: usize) {
        let Some(sample_view) = &self.sample_view else {
            self.mode = AppMode::Sampler;
            self.notify_warning("Load a sample before replacing an assignment");
            return;
        };

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };

        let track_id = track.id;
        let track_name = track.name.clone();
        let previous_sample = self
            .song
            .sample_assignment_for_track(track_id)
            .map(|assignment| assignment.sample);
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();

        self.mutate_song(|song, _| {
            let sample_id = song
                .replace_track_sample(track_id, sample_path, sample_name)
                .expect("track exists and sample was just upserted");
            if let Some(previous) = previous_sample {
                if previous != sample_id && !song.is_sample_assigned(previous) {
                    let _ = song.remove_sample_reference(previous);
                }
            }
        });
        self.mode = AppMode::Sampler;
        self.notify_success(format!("Sample replaced on {track_name}"));
    }

    fn unload_current_sample(&mut self) {
        let Some(sample_view) = &self.sample_view else {
            self.mode = AppMode::Sampler;
            self.notify_warning("No sample loaded");
            return;
        };

        let sample_path = sample_view.source_path.to_string_lossy();
        let sample_id = self
            .song
            .samples
            .iter()
            .find(|sample| sample.path == sample_path)
            .map(|sample| sample.id);

        match sample_id {
            Some(sample_id) if self.song.is_sample_assigned(sample_id) => {
                self.mode = AppMode::Sampler;
                self.notify_warning("Unassign or replace sample before unloading it");
            }
            Some(sample_id) => {
                self.mutate_song(|song, _| {
                    let _ = song.remove_sample_reference(sample_id);
                });
                self.sample_view = None;
                self.mode = AppMode::Sampler;
                self.notify_success("Sample unloaded");
            }
            None => {
                self.sample_view = None;
                self.mode = AppMode::Sampler;
                self.notify_info("Sample view cleared");
            }
        }
    }

    fn cleanup_unused_sample_references(&mut self) {
        let mut removed = 0;
        self.mutate_song(|song, _| {
            removed = song.prune_unused_sample_references();
        });

        if removed == 0 {
            self.notify_info("No unused sample references");
        } else {
            self.notify_success(format!("Removed {removed} unused sample reference(s)"));
        }
    }

    fn show_sample_assignments(&mut self) {
        if self.song.sample_assignments.is_empty() {
            self.notify_info("No sample assignments");
            return;
        }

        let assignments = self
            .song
            .sample_assignments
            .iter()
            .filter_map(|assignment| {
                let track = self
                    .song
                    .tracks
                    .iter()
                    .find(|track| track.id == assignment.track)?;
                let sample = self.song.sample_for_id(assignment.sample)?;
                Some(format!("{}={}", track.name, sample.name))
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.notify_info(format!("Samples: {assignments}"));
    }

    fn refresh_midi_ports(&mut self) {
        match list_output_ports() {
            Ok(ports) => {
                self.midi_ports = ports;
                self.midi_port_cursor = self
                    .midi_port_cursor
                    .min(self.midi_ports.len().saturating_sub(1));
                if self.midi_ports.is_empty() {
                    self.midi_status = "MIDI No Outputs".to_string();
                    self.notify_warning("No MIDI output ports found");
                } else {
                    self.notify_info(format!("Found {} MIDI output(s)", self.midi_ports.len()));
                }
            }
            Err(error) => {
                self.midi_ports.clear();
                self.midi_port_cursor = 0;
                self.midi_status = format!("MIDI Error: {error}");
                self.notify_error(format!("MIDI output list failed: {error}"));
            }
        }
    }

    fn next_midi_port(&mut self) {
        self.midi_port_cursor = self
            .midi_port_cursor
            .saturating_add(1)
            .min(self.midi_ports.len().saturating_sub(1));
    }

    fn previous_midi_port(&mut self) {
        self.midi_port_cursor = self.midi_port_cursor.saturating_sub(1);
    }

    fn connect_selected_midi_port(&mut self) {
        if let Some(port) = self.midi_ports.get(self.midi_port_cursor) {
            self.connect_midi(port.index);
        } else {
            self.midi_status = "MIDI No Outputs".to_string();
            self.notify_warning("No MIDI output selected");
        }
    }

    fn connect_default_midi_output(&mut self, output_name: &str) {
        if output_name.trim().is_empty() {
            return;
        }

        match list_output_ports() {
            Ok(ports) => {
                self.midi_ports = ports;
                if let Some((position, port)) =
                    resolve_midi_output_port(&self.midi_ports, output_name)
                {
                    self.midi_port_cursor = position;
                    self.midi_status = format!("MIDI Connecting {} ({})", port.index, port.name);
                    self.playback.connect_midi(port.index);
                } else {
                    self.midi_status = format!("MIDI Output Not Found ({output_name})");
                    self.notify_error(format!("MIDI output not found: {output_name}"));
                }
            }
            Err(error) => {
                self.midi_status = format!("MIDI Error: {error}");
                self.notify_error(format!("MIDI output list failed: {error}"));
            }
        }
    }

    fn disconnect_midi(&mut self) {
        self.playback.disconnect_midi();
        self.notify_info("Disconnecting MIDI output");
    }

    fn panic_midi(&mut self) {
        self.playback.panic_all_notes_off();
        self.is_playing = false;
        self.playhead_row = None;
        self.sequence_position = None;
        self.notify_warning("MIDI panic sent");
    }

    fn drain_playback_updates(&mut self) {
        while let Some(update) = self.playback.try_recv() {
            match update {
                PlaybackUpdate::Position(position) => {
                    self.is_playing = true;
                    self.pattern_index = position.pattern_index;
                    self.sequence_position = position.sequence_index;
                    self.playhead_row = Some(position.position.row);
                }
                PlaybackUpdate::Stopped => {
                    self.is_playing = false;
                    self.playhead_row = None;
                    self.sequence_position = None;
                    self.notify_info("Playback stopped");
                }
                PlaybackUpdate::MidiConnected { port_index } => {
                    self.midi_status = format!("MIDI Connected {port_index}");
                    self.notify_success(format!("MIDI output connected: {port_index}"));
                }
                PlaybackUpdate::MidiDisconnected => {
                    self.midi_status = "MIDI Disconnected".to_string();
                    self.notify_info("MIDI output disconnected");
                }
                PlaybackUpdate::MidiError(error) => {
                    self.midi_status = format!("MIDI Error: {error}");
                    self.is_playing = false;
                    self.playhead_row = None;
                    self.sequence_position = None;
                    self.notify_error(format!("MIDI error: {error}"));
                }
                PlaybackUpdate::MidiLogError(error) => {
                    self.midi_status = format!("MIDI Log Error: {error}");
                    self.notify_error(format!("MIDI log error: {error}"));
                }
            }
        }
    }

    fn mutate_song(&mut self, mutate: impl FnOnce(&mut Song, Cursor)) {
        let before = self.song.clone();
        mutate(&mut self.song, self.cursor);
        if self.song != before {
            self.undo_stack.push(before);
            if self.undo_stack.len() > UNDO_LIMIT {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
            self.refresh_dirty();
            self.clamp_sequence_cursor();
        }
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.song, previous);
            self.redo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.notify_info("Undo");
        } else {
            self.notify_warning("Nothing to undo");
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.song, next);
            self.undo_stack.push(current);
            self.refresh_dirty();
            self.clamp_cursor();
            self.clamp_sequence_cursor();
            self.notify_info("Redo");
        } else {
            self.notify_warning("Nothing to redo");
        }
    }

    fn advance_after_edit(&mut self) {
        self.cursor.row = self
            .cursor
            .row
            .saturating_add(self.edit_step)
            .min(self.current_row_count().saturating_sub(1));
    }

    fn increment_octave(&mut self) {
        self.octave = self.octave.saturating_add(1).min(9);
    }

    fn decrement_octave(&mut self) {
        self.octave = self.octave.saturating_sub(1);
    }

    fn start_save_as_command(&mut self) {
        let path = self.project_path.as_ref().map_or_else(
            || "untitled.salieri".to_string(),
            |path| path.display().to_string(),
        );
        self.command_buffer = format!("saveas {path}");
        self.mode = AppMode::Command;
        self.notify_info("Save As: edit path and press Enter");
    }

    fn refresh_dirty(&mut self) {
        self.dirty = self.song != self.clean_song;
    }

    fn save(&mut self) -> Result<()> {
        let path = self
            .project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.salieri"));
        self.save_as(path)
    }

    fn save_as(&mut self, path: PathBuf) -> Result<()> {
        save_project(&path, &self.song)?;
        self.project_path = Some(path);
        self.clean_song = self.song.clone();
        self.refresh_dirty();
        self.notify_success("Project saved");
        Ok(())
    }

    fn clamp_cursor(&mut self) {
        self.clamp_pattern_index();
        self.cursor
            .clamp(self.current_row_count(), self.song.tracks.len());
    }

    fn clamp_pattern_index(&mut self) {
        self.pattern_index = self
            .pattern_index
            .min(self.song.patterns.len().saturating_sub(1));
    }

    fn clamp_sequence_cursor(&mut self) {
        if self.song.sequence.is_empty() {
            self.sequence_cursor = 0;
        } else {
            self.sequence_cursor = self
                .sequence_cursor
                .min(self.song.sequence.len().saturating_sub(1));
        }
    }

    fn tui_sequence_position(&self) -> Option<usize> {
        self.sequence_position.or_else(|| {
            (!self.song.sequence.is_empty()).then_some(
                self.sequence_cursor
                    .min(self.song.sequence.len().saturating_sub(1)),
            )
        })
    }

    fn tui_active_view(&self) -> TuiView {
        match self.mode {
            AppMode::Sequence => TuiView::Sequence,
            AppMode::Tracks => TuiView::Tracks,
            AppMode::Patterns => TuiView::Patterns,
            AppMode::Sampler => TuiView::Sampler,
            AppMode::Normal
            | AppMode::Edit
            | AppMode::Command
            | AppMode::Help
            | AppMode::Dialog
            | AppMode::MidiSettings => TuiView::Pattern,
        }
    }

    fn keep_cursor_visible(&mut self, visible_rows: usize) {
        self.keep_row_visible(self.cursor.row, visible_rows);
    }

    fn keep_active_row_visible(&mut self, visible_rows: usize) {
        let row = if self.is_playing && self.follow_playhead {
            self.playhead_row.unwrap_or(self.cursor.row)
        } else {
            self.cursor.row
        };
        self.keep_row_visible(row, visible_rows);
    }

    fn keep_row_visible(&mut self, row: usize, visible_rows: usize) {
        let visible_rows = visible_rows.max(1);
        if row < self.row_offset {
            self.row_offset = row;
        } else if row >= self.row_offset.saturating_add(visible_rows) {
            self.row_offset = row.saturating_sub(visible_rows - 1);
        }

        let max_offset = self.current_row_count().saturating_sub(visible_rows);
        self.row_offset = self.row_offset.min(max_offset);
    }

    fn current_row_count(&self) -> usize {
        self.song
            .pattern(self.pattern_index)
            .map_or(0, |pattern| pattern.row_count())
    }

    fn command_line(&self) -> Option<&str> {
        if self.mode == AppMode::Command {
            Some(self.command_buffer.as_str())
        } else {
            None
        }
    }

    fn quit_confirmation(&self) -> bool {
        self.mode == AppMode::Dialog && matches!(self.dialog, Some(Dialog::QuitDirty))
    }

    fn delete_confirmation_message(&self) -> Option<&str> {
        if self.mode != AppMode::Dialog {
            return None;
        }

        match &self.dialog {
            Some(Dialog::DeleteTrack { message, .. }) => Some(message.as_str()),
            Some(Dialog::DeletePattern { message, .. }) => Some(message.as_str()),
            Some(Dialog::QuitDirty) | None => None,
        }
    }

    fn notify(&mut self, kind: NotificationKind, message: impl Into<String>) {
        self.notification = Some(Notification {
            kind,
            message: message.into(),
            expires_at: Instant::now() + NOTIFICATION_TTL,
        });
    }

    fn notify_info(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Info, message);
    }

    fn notify_success(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Success, message);
    }

    fn notify_warning(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Warning, message);
    }

    fn notify_error(&mut self, message: impl Into<String>) {
        self.notify(NotificationKind::Error, message);
    }

    fn expire_notification(&mut self) {
        if self
            .notification
            .as_ref()
            .is_some_and(|notification| Instant::now() >= notification.expires_at)
        {
            self.notification = None;
        }
    }

    fn tui_notification(&self) -> Option<NotificationView<'_>> {
        self.notification
            .as_ref()
            .map(|notification| NotificationView {
                kind: notification.kind,
                message: notification.message.as_str(),
            })
    }

    fn tui_midi_ports(&self) -> Vec<MidiPortView<'_>> {
        self.midi_ports
            .iter()
            .map(|port| MidiPortView {
                index: port.index,
                name: port.name.as_str(),
            })
            .collect()
    }

    fn tui_midi_settings<'a>(
        &'a self,
        ports: &'a [MidiPortView<'a>],
    ) -> Option<MidiSettingsState<'a>> {
        (self.mode == AppMode::MidiSettings).then_some(MidiSettingsState {
            ports,
            selected_port: self.midi_port_cursor.min(ports.len().saturating_sub(1)),
            status: self.midi_status.as_str(),
        })
    }

    fn tui_sampler_view(&self) -> Option<SamplerViewState<'_>> {
        self.sample_view.as_ref().map(|sample| {
            let sample_path = sample.source_path.to_string_lossy();
            let sample_id = self
                .song
                .samples
                .iter()
                .find(|reference| reference.path == sample_path.as_ref())
                .map(|reference| reference.id);
            let assigned_tracks = sample_id.map_or_else(Vec::new, |sample_id| {
                self.song
                    .sample_assignments
                    .iter()
                    .filter(|assignment| assignment.sample == sample_id)
                    .filter_map(|assignment| {
                        self.song
                            .tracks
                            .iter()
                            .find(|track| track.id == assignment.track)
                    })
                    .collect::<Vec<_>>()
            });
            SamplerViewState {
                name: sample.sample.name.as_str(),
                source_path: sample.source_path.to_str().unwrap_or("<non-utf8 path>"),
                overview: &sample.overview,
                assigned_track: assigned_tracks.first().map(|track| track.name.as_str()),
                assigned_track_count: assigned_tracks.len(),
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
    Normal,
    Edit,
    Command,
    Help,
    Dialog,
    MidiSettings,
    Sequence,
    Tracks,
    Patterns,
    Sampler,
}

impl AppMode {
    const fn label(self) -> &'static str {
        match self {
            AppMode::Normal => "NORMAL",
            AppMode::Edit => "EDIT",
            AppMode::Command => "COMMAND",
            AppMode::Help => "HELP",
            AppMode::Dialog => "DIALOG",
            AppMode::MidiSettings => "MIDI",
            AppMode::Sequence => "SEQUENCE",
            AppMode::Tracks => "TRACKS",
            AppMode::Patterns => "PATTERNS",
            AppMode::Sampler => "SAMPLER",
        }
    }
}

fn parse_optional_numbered_name(values: &[&str], default_index: usize) -> Option<(usize, String)> {
    let first = values.first()?;
    if let Ok(number) = first.parse::<usize>() {
        let name = values.get(1..)?.join(" ");
        Some((number.saturating_sub(1), name))
    } else {
        Some((default_index, values.join(" ")))
    }
}

fn parse_track_number(value: &str) -> Option<usize> {
    value
        .parse::<usize>()
        .ok()
        .map(|number| number.saturating_sub(1))
}

fn parse_hex_byte(value: &str) -> Option<u8> {
    u8::from_str_radix(value.trim_start_matches("0x"), 16).ok()
}

fn keyboard_note(key: char, octave: u8) -> Option<u8> {
    let (semitone, octave_offset) = match key.to_ascii_lowercase() {
        'z' => (0, 0),
        's' => (1, 0),
        'x' => (2, 0),
        'd' => (3, 0),
        'c' => (4, 0),
        'v' => (5, 0),
        'g' => (6, 0),
        'b' => (7, 0),
        'h' => (8, 0),
        'n' => (9, 0),
        'j' => (10, 0),
        'm' => (11, 0),
        'q' => (0, 1),
        '2' => (1, 1),
        'w' => (2, 1),
        '3' => (3, 1),
        'e' => (4, 1),
        'r' => (5, 1),
        '5' => (6, 1),
        't' => (7, 1),
        '6' => (8, 1),
        'y' => (9, 1),
        '7' => (10, 1),
        'u' => (11, 1),
        _ => return None,
    };

    let midi_octave = i16::from(octave) + octave_offset + 1;
    let pitch = midi_octave * 12 + semitone;
    u8::try_from(pitch).ok().filter(|pitch| *pitch <= 127)
}

fn find_midi_output_port<'a>(
    ports: &'a [MidiOutputPort],
    output_name: &str,
) -> Option<(usize, &'a MidiOutputPort)> {
    let needle = output_name.trim().to_lowercase();
    let normalized_needle = normalize_midi_port_name(output_name);
    if needle.is_empty() {
        return None;
    }

    ports
        .iter()
        .enumerate()
        .find(|(_, port)| port.name.eq_ignore_ascii_case(output_name.trim()))
        .or_else(|| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.name.to_lowercase().contains(&needle))
        })
        .or_else(|| {
            ports.iter().enumerate().find(|(_, port)| {
                let normalized_name = normalize_midi_port_name(&port.name);
                normalized_name == normalized_needle
                    || normalized_name.contains(&normalized_needle)
                    || normalized_needle.contains(&normalized_name)
            })
        })
}

fn resolve_midi_output_port<'a>(
    ports: &'a [MidiOutputPort],
    output_name_or_index: &str,
) -> Option<(usize, &'a MidiOutputPort)> {
    let value = output_name_or_index.trim();
    value
        .parse::<usize>()
        .ok()
        .and_then(|index| {
            ports
                .iter()
                .enumerate()
                .find(|(_, port)| port.index == index)
        })
        .or_else(|| find_midi_output_port(ports, value))
}

fn normalize_midi_port_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_help_version_and_midi_listing() {
        assert_eq!(
            CliArgs::parse(["--help".to_string()]),
            CliArgs {
                command: CliCommand::Help,
                project_path: None,
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs::default(),
            }
        );
        assert_eq!(
            CliArgs::parse(["--version".to_string()]).command,
            CliCommand::Version
        );
        assert_eq!(
            CliArgs::parse(["--list-midi-outputs".to_string()]).command,
            CliCommand::ListMidiOutputs
        );
    }

    #[test]
    fn cli_parses_optional_project_path() {
        assert_eq!(
            CliArgs::parse(["song.salieri".to_string()]),
            CliArgs {
                command: CliCommand::Run,
                project_path: Some(PathBuf::from("song.salieri")),
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs::default(),
            }
        );
    }

    #[test]
    fn cli_parses_config_and_log_level_options() {
        assert_eq!(
            CliArgs::parse([
                "--config".to_string(),
                "custom.toml".to_string(),
                "--log-level=debug".to_string(),
                "--midi-log".to_string(),
                "midi.log".to_string(),
                "song.salieri".to_string()
            ]),
            CliArgs {
                command: CliCommand::Run,
                project_path: Some(PathBuf::from("song.salieri")),
                config_path: Some(PathBuf::from("custom.toml")),
                log_level: Some("debug".to_string()),
                midi_log_path: Some(PathBuf::from("midi.log")),
                midi_test: MidiTestArgs::default(),
            }
        );
    }

    #[test]
    fn cli_parses_midi_test_options() {
        assert_eq!(
            CliArgs::parse([
                "--midi-test-output=0".to_string(),
                "--midi-test-channel".to_string(),
                "2".to_string(),
                "--midi-test-note".to_string(),
                "64".to_string(),
                "--midi-test-duration-ms".to_string(),
                "1500".to_string(),
            ]),
            CliArgs {
                command: CliCommand::MidiTest,
                project_path: None,
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs {
                    output: Some("0".to_string()),
                    channel: 2,
                    note: 64,
                    duration_ms: 1500,
                },
            }
        );
    }

    #[test]
    fn cli_parses_euclidean_transform_options() {
        assert_eq!(
            CliArgs::parse([
                "transform".to_string(),
                "euclidean".to_string(),
                "input.salieri".to_string(),
                "output.salieri".to_string(),
                "--pattern=2".to_string(),
                "--track".to_string(),
                "3".to_string(),
                "--steps".to_string(),
                "12".to_string(),
                "--pulses=5".to_string(),
                "--rotation=1".to_string(),
                "--pitch".to_string(),
                "40".to_string(),
                "--velocity=96".to_string(),
            ]),
            CliArgs {
                command: CliCommand::TransformEuclidean(TransformEuclideanArgs {
                    input_path: Some(PathBuf::from("input.salieri")),
                    output_path: Some(PathBuf::from("output.salieri")),
                    pattern: 2,
                    track: 3,
                    steps: 12,
                    pulses: 5,
                    rotation: 1,
                    pitch: 40,
                    velocity: 96,
                }),
                project_path: None,
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs::default(),
            }
        );
    }

    #[test]
    fn cli_parses_sample_inspect_options() {
        assert_eq!(
            CliArgs::parse([
                "sample".to_string(),
                "inspect".to_string(),
                "kick.wav".to_string(),
                "--format=json".to_string(),
                "--width".to_string(),
                "8".to_string(),
            ]),
            CliArgs {
                command: CliCommand::SampleInspect(SampleInspectArgs {
                    path: Some(PathBuf::from("kick.wav")),
                    format: SampleInspectFormat::Json,
                    buckets: 8,
                }),
                project_path: None,
                config_path: None,
                log_level: None,
                midi_log_path: None,
                midi_test: MidiTestArgs::default(),
            }
        );
    }

    #[test]
    fn sample_inspect_loads_tiny_wav_and_formats_outputs() {
        let path =
            std::env::temp_dir().join(format!("salieri-sample-inspect-{}.wav", std::process::id()));
        std::fs::write(
            &path,
            wav_pcm16_bytes(44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]),
        )
        .expect("write wav");

        let inspection = inspect_sample(&SampleInspectArgs {
            path: Some(path.clone()),
            format: SampleInspectFormat::Text,
            buckets: 2,
        })
        .expect("inspect sample");
        let _ = std::fs::remove_file(&path);

        assert_eq!(inspection.sample.sample_rate, 44_100);
        assert_eq!(inspection.sample.channels, 1);
        assert_eq!(inspection.sample.frames, 4);
        assert_eq!(inspection.overview.buckets.len(), 2);

        let text = format_sample_inspection_text(&inspection);
        assert!(text.contains("sample_rate: 44100"));
        assert!(text.contains("channels: 1"));
        assert!(text.contains("waveform_buckets: 2"));

        let json = format_sample_inspection_json(&inspection).expect("json");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["sample"]["sample_rate"], 44_100);
        assert_eq!(value["waveform"]["bucket_count"], 2);
    }

    #[test]
    fn sample_inspect_reports_invalid_wav_with_context() {
        let path = std::env::temp_dir().join(format!(
            "salieri-sample-inspect-invalid-{}.wav",
            std::process::id()
        ));
        std::fs::write(&path, b"not a wave").expect("write invalid wav");

        let error = inspect_sample(&SampleInspectArgs {
            path: Some(path.clone()),
            format: SampleInspectFormat::Text,
            buckets: 4,
        })
        .expect_err("invalid wav");
        let _ = std::fs::remove_file(&path);

        assert!(format!("{error:#}").contains("failed to load sample"));
    }

    #[test]
    fn euclidean_transform_command_round_trips_project_files() {
        let base =
            std::env::temp_dir().join(format!("salieri-transform-cli-{}", std::process::id()));
        let input_path = base.with_extension("input.salieri");
        let output_path = base.with_extension("output.salieri");
        let song = Song::empty();
        save_project(&input_path, &song).expect("save input");

        run_transform_euclidean(&TransformEuclideanArgs {
            input_path: Some(input_path.clone()),
            output_path: Some(output_path.clone()),
            pattern: 1,
            track: 1,
            steps: 4,
            pulses: 2,
            rotation: 0,
            pitch: 36,
            velocity: 100,
        })
        .expect("transform");

        let transformed = load_project(&output_path).expect("load output");
        let pattern = transformed.current_pattern().expect("pattern");
        let active_rows = (0..8)
            .filter(|row| pattern.cell(*row, 0).expect("cell").note.is_some())
            .collect::<Vec<_>>();

        let _ = std::fs::remove_file(&input_path);
        let _ = std::fs::remove_file(&output_path);

        assert_eq!(active_rows, vec![1, 3, 5, 7]);
    }

    #[test]
    fn app_uses_keyboard_config_defaults() {
        let app = App::new(AppConfig {
            keyboard: config::KeyboardConfig {
                default_octave: 5,
                edit_step: 4,
                vim_navigation: false,
            },
            ui: config::UiConfig {
                show_line_numbers_hex: true,
                ..config::UiConfig::default()
            },
            ..AppConfig::default()
        });

        assert_eq!(app.octave, 5);
        assert_eq!(app.edit_step, 4);
        assert!(!app.vim_navigation);
        assert!(app.show_line_numbers_hex);
    }

    #[test]
    fn vim_navigation_can_be_disabled_by_config() {
        let mut app = App::new(AppConfig {
            keyboard: config::KeyboardConfig {
                vim_navigation: false,
                ..config::KeyboardConfig::default()
            },
            ..AppConfig::default()
        });
        app.cursor.row = 4;

        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert_eq!(app.cursor.row, 3);
    }

    #[test]
    fn vim_navigation_jumps_to_pattern_bounds() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(app.cursor.row, 63);

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.cursor.row, 63);

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(app.cursor.row, 0);
    }

    #[test]
    fn playhead_follow_can_be_disabled_by_config() {
        let mut app = App::new(AppConfig {
            ui: config::UiConfig {
                follow_playhead: false,
                ..config::UiConfig::default()
            },
            ..AppConfig::default()
        });
        app.cursor.row = 0;
        app.is_playing = true;
        app.playhead_row = Some(20);

        app.keep_active_row_visible(10);

        assert_eq!(app.row_offset, 0);
    }

    #[test]
    fn finds_midi_output_by_exact_or_partial_name() {
        let ports = vec![
            MidiOutputPort {
                index: 0,
                name: "External Synth".to_string(),
            },
            MidiOutputPort {
                index: 1,
                name: "IAC Driver Bus 1".to_string(),
            },
        ];

        assert_eq!(
            find_midi_output_port(&ports, "IAC Driver")
                .map(|(position, port)| (position, port.index)),
            Some((1, 1))
        );
        assert_eq!(
            find_midi_output_port(&ports, "iac driver bus 1")
                .map(|(position, port)| (position, port.index)),
            Some((1, 1))
        );
        assert_eq!(
            find_midi_output_port(&ports, "IAC Driver (Bus 1)")
                .map(|(position, port)| (position, port.index)),
            Some((1, 1))
        );
        assert_eq!(
            resolve_midi_output_port(&ports, "1")
                .map(|(position, port)| (position, port.name.as_str())),
            Some((1, "IAC Driver Bus 1"))
        );
        assert!(find_midi_output_port(&ports, "Missing").is_none());
    }

    #[test]
    fn midi_settings_keys_select_connect_and_close() {
        let mut app = App {
            midi_ports: vec![
                MidiOutputPort {
                    index: 0,
                    name: "First".to_string(),
                },
                MidiOutputPort {
                    index: 2,
                    name: "Second".to_string(),
                },
            ],
            mode: AppMode::MidiSettings,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.midi_port_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.midi_status, "MIDI Connecting 2");

        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn midi_settings_connect_without_ports_reports_warning() {
        let mut app = App {
            midi_ports: Vec::new(),
            mode: AppMode::MidiSettings,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::MidiSettings);
        assert_eq!(app.midi_status, "MIDI No Outputs");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("No MIDI output selected")
        );
        assert!(!app.dirty);
    }

    #[test]
    fn f4_opens_midi_settings_without_mutating_song() {
        let mut app = App::default();
        let song = app.song.clone();

        app.handle_key(KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::MidiSettings);
        assert_eq!(app.song, song);
        assert!(!app.dirty);
    }

    #[test]
    fn f5_refreshes_midi_settings_without_mutating_song() {
        let mut app = App {
            mode: AppMode::MidiSettings,
            ..App::default()
        };
        let song = app.song.clone();

        app.handle_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::MidiSettings);
        assert_eq!(app.song, song);
        assert!(!app.dirty);
    }

    #[test]
    fn scrolls_down_to_keep_cursor_visible() {
        let mut app = App {
            cursor: Cursor {
                row: 20,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.keep_cursor_visible(10);

        assert_eq!(app.row_offset, 11);
    }

    #[test]
    fn scrolls_up_to_keep_cursor_visible() {
        let mut app = App {
            cursor: Cursor {
                row: 5,
                ..Cursor::new()
            },
            row_offset: 20,
            ..App::default()
        };

        app.keep_cursor_visible(10);

        assert_eq!(app.row_offset, 5);
    }

    #[test]
    fn scroll_offset_is_clamped_near_pattern_end() {
        let mut app = App {
            cursor: Cursor {
                row: 63,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.keep_cursor_visible(20);

        assert_eq!(app.row_offset, 44);
    }

    #[test]
    fn scrolls_to_keep_playhead_visible_while_playing() {
        let mut app = App {
            cursor: Cursor {
                row: 0,
                ..Cursor::new()
            },
            is_playing: true,
            playhead_row: Some(20),
            ..App::default()
        };

        app.keep_active_row_visible(10);

        assert_eq!(app.row_offset, 11);
    }

    #[test]
    fn tab_and_backtab_move_between_tracks() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.cursor.track, 1);

        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(app.cursor.track, 0);

        app.mode = AppMode::Edit;
        for _ in 0..10 {
            app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        }
        assert_eq!(app.cursor.track, 3);

        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        assert_eq!(app.cursor.track, 2);
    }

    #[test]
    fn edit_mode_inserts_note_and_advances_cursor() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        let cell = pattern.cell(0, 0).expect("cell");
        assert_eq!(app.mode, AppMode::Edit);
        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
        assert_eq!(cell.velocity, Some(DEFAULT_NOTE_VELOCITY));
        assert_eq!(app.cursor.row, 1);
        assert!(app.dirty);
    }

    #[test]
    fn edit_mode_inserts_note_off_and_note_cut() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        let off = pattern.cell(0, 0).expect("note off cell");
        let cut = pattern.cell(1, 0).expect("note cut cell");
        assert_eq!(off.note, Some(NoteEvent::NoteOff));
        assert_eq!(off.velocity, None);
        assert_eq!(cut.note, Some(NoteEvent::NoteCut));
        assert_eq!(cut.velocity, None);
        assert_eq!(app.cursor.row, 2);
    }

    #[test]
    fn velocity_entry_uses_two_hex_digits() {
        let mut app = App {
            mode: AppMode::Edit,
            cursor: Cursor {
                field: CellField::Velocity,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE));
        assert_eq!(app.cursor.row, 0);
        assert_eq!(app.cursor.digit, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        let cell = pattern.cell(0, 0).expect("cell");
        assert_eq!(cell.velocity, Some(0x4f));
        assert_eq!(app.cursor.row, 1);
        assert_eq!(app.cursor.digit, 0);
    }

    #[test]
    fn clipboard_copies_cuts_and_pastes_current_cell() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.cursor.row = 0;

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        app.cursor.row = 4;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));

        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(4, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 60 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(4, 0)
                .expect("cell"),
            &PatternCell::default()
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(4, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 60 })
        );
    }

    #[test]
    fn selection_region_can_be_copied_cut_pasted_and_deleted() {
        let mut app = App::default();
        {
            let pattern = app.song.current_pattern_mut().expect("pattern");
            pattern
                .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
                .expect("set note");
            pattern
                .set_note(0, 1, NoteEvent::Note { pitch: 62 }, 0x7f)
                .expect("set note");
            pattern
                .set_note(1, 0, NoteEvent::Note { pitch: 64 }, 0x7f)
                .expect("set note");
            pattern
                .set_note(1, 1, NoteEvent::Note { pitch: 65 }, 0x7f)
                .expect("set note");
        }

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(
            app.selection_rect(),
            Some(SelectionRect {
                row_start: 0,
                row_end: 1,
                track_start: 0,
                track_end: 1,
            })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        app.cursor.row = 4;
        app.cursor.track = 2;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(5, 3)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 65 })
        );

        app.cursor.row = 0;
        app.cursor.track = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(app.selection_rect(), None);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(1, 1)
                .expect("cell"),
            &PatternCell::default()
        );

        app.cursor.row = 8;
        app.cursor.track = 0;
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(9, 1)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 65 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(8, 0)
                .expect("cell"),
            &PatternCell::default()
        );
    }

    #[test]
    fn insert_and_ctrl_delete_edit_pattern_rows() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.cursor.row = 0;

        app.handle_key(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));

        let pattern = app.song.current_pattern().expect("pattern");
        assert_eq!(pattern.row_count(), 65);
        assert_eq!(pattern.cell(0, 0), Some(&PatternCell::default()));
        assert_eq!(
            pattern.cell(1, 0).expect("cell").note,
            Some(NoteEvent::Note { pitch: 60 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 65);
    }

    #[test]
    fn undo_and_redo_restore_song_snapshots() {
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell"),
            &salieri_core::PatternCell::default()
        );
        assert!(!app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 60 })
        );
        assert!(app.dirty);
    }

    #[test]
    fn keyboard_note_maps_tracker_keys_to_midi_pitches() {
        assert_eq!(keyboard_note('z', 4), Some(60));
        assert_eq!(keyboard_note('s', 4), Some(61));
        assert_eq!(keyboard_note('q', 4), Some(72));
        assert_eq!(keyboard_note('u', 4), Some(83));
    }

    #[test]
    fn ctrl_s_saves_project_and_clears_dirty_state() {
        let path =
            std::env::temp_dir().join(format!("salieri-app-save-{}.salieri", std::process::id()));
        let mut app = App {
            mode: AppMode::Edit,
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Project saved")
        );
    }

    #[test]
    fn ctrl_shift_s_opens_save_as_prompt_with_current_path() {
        let path = PathBuf::from("current-song.salieri");
        let mut app = App {
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(
            KeyCode::Char('S'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));

        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, format!("saveas {}", path.display()));
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Save As: edit path and press Enter")
        );
    }

    #[test]
    fn save_as_prompt_can_save_to_selected_path() {
        let path = std::env::temp_dir().join(format!(
            "salieri-shortcut-save-as-{}.salieri",
            std::process::id()
        ));
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(
            KeyCode::Char('S'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        app.command_buffer = format!("saveas {}", path.display());
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert_eq!(app.project_path, Some(path));
        assert!(!app.dirty);
    }

    #[test]
    fn save_as_prompt_reports_errors_and_keeps_dirty_state() {
        let missing_dir = std::env::temp_dir().join(format!(
            "salieri-missing-save-as-dir-{}",
            std::process::id()
        ));
        let path = missing_dir.join("song.salieri");
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(
            KeyCode::Char('S'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        app.command_buffer = format!("saveas {}", path.display());
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let notification = app.notification.as_ref().expect("notification");
        assert_eq!(notification.kind, NotificationKind::Error);
        assert!(notification.message.starts_with("Save failed:"));
        assert_eq!(app.project_path, None);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_sets_bpm_and_lpb() {
        let mut app = App::default();

        type_command(&mut app, "bpm 140");
        type_command(&mut app, "lpb 8");

        assert_eq!(app.song.transport.bpm, 140);
        assert_eq!(app.song.transport.lines_per_beat, 8);
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.lines_per_beat, 4);
        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.bpm, 120);
        assert!(!app.dirty);
    }

    #[test]
    fn control_arrows_adjust_bpm_and_lpb() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.bpm, 121);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("BPM 121")
        );

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.bpm, 120);
        assert!(!app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.lines_per_beat, 5);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("LPB 5")
        );

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL));
        assert_eq!(app.song.transport.lines_per_beat, 4);
        assert!(!app.dirty);

        app.song.transport.bpm = MIN_BPM;
        app.song.transport.lines_per_beat = MAX_LPB;
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL));
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));

        assert_eq!(app.song.transport.bpm, MIN_BPM);
        assert_eq!(app.song.transport.lines_per_beat, MAX_LPB);
    }

    #[test]
    fn command_mode_sets_pattern_loop() {
        let mut app = App::default();

        assert!(app.loop_pattern);
        type_command(&mut app, "loop off");
        assert!(!app.loop_pattern);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern loop OFF")
        );
        type_command(&mut app, "loop on");
        assert!(app.loop_pattern);
        type_command(&mut app, "loop");
        assert!(!app.loop_pattern);
    }

    #[test]
    fn command_mode_sets_and_clears_current_effect_command() {
        let mut app = App::default();

        type_command(&mut app, "fx D 20");

        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell")
                .command,
            Some(TrackerCommand::delay(0x20))
        );
        assert!(app.dirty);

        type_command(&mut app, "fx clear");

        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell")
                .command,
            None
        );
        assert!(!app.dirty);
    }

    #[test]
    fn command_mode_reports_unknown_commands() {
        let mut app = App::default();

        type_command(&mut app, "doesnotexist");

        let notification = app.notification.as_ref().expect("notification");
        assert_eq!(notification.kind, NotificationKind::Warning);
        assert_eq!(notification.message, "Unknown command: doesnotexist");
    }

    #[test]
    fn command_mode_write_saves_project() {
        let path = std::env::temp_dir().join(format!(
            "salieri-command-write-{}.salieri",
            std::process::id()
        ));
        let mut app = App {
            mode: AppMode::Edit,
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        type_command(&mut app, "write");

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert!(!app.dirty);
        assert!(!app.should_quit);
    }

    #[test]
    fn command_mode_write_accepts_project_path() {
        let path = std::env::temp_dir().join(format!(
            "salieri-command-write-as-{}.salieri",
            std::process::id()
        ));
        let mut app = App {
            mode: AppMode::Edit,
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        type_command(&mut app, &format!("write {}", path.display()));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert_eq!(app.project_path, Some(path));
        assert!(!app.dirty);
    }

    #[test]
    fn command_mode_quit_marks_app_for_exit() {
        let mut app = App::default();

        type_command(&mut app, "quit");

        assert!(app.should_quit);
    }

    #[test]
    fn dirty_quit_opens_confirmation_dialog() {
        let mut app = App::default();

        app.set_bpm(140);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Dialog);
        assert!(!app.should_quit);

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn dirty_quit_can_discard_changes() {
        let mut app = App::default();

        app.set_bpm(140);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

        assert!(app.should_quit);
    }

    #[test]
    fn dirty_quit_can_save_before_exit() {
        let path =
            std::env::temp_dir().join(format!("salieri-quit-save-{}.salieri", std::process::id()));
        let mut app = App {
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.set_bpm(140);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved.transport.bpm, 140);
        assert!(!app.dirty);
        assert!(app.should_quit);
    }

    #[test]
    fn force_quit_command_bypasses_dirty_confirmation() {
        let mut app = App::default();

        app.set_bpm(140);
        type_command(&mut app, "q!");

        assert_ne!(app.mode, AppMode::Dialog);
        assert!(app.should_quit);
    }

    #[test]
    fn space_toggles_playback_and_f8_stops() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(0));

        app.handle_key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));

        assert!(!app.is_playing);
        assert_eq!(app.playhead_row, None);
    }

    #[test]
    fn shift_space_starts_playback_from_pattern_start() {
        let mut app = App {
            cursor: Cursor {
                row: 12,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::SHIFT));

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, None);
    }

    #[test]
    fn uppercase_l_toggles_pattern_loop_without_breaking_vim_right() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));
        assert!(!app.loop_pattern);

        app.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert_eq!(app.cursor.field, CellField::Velocity);
        assert!(!app.loop_pattern);
    }

    #[test]
    fn enter_starts_playback_from_cursor_row() {
        let mut app = App {
            cursor: Cursor {
                row: 12,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(12));
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn command_mode_requests_midi_connection_and_panic_stops_playback() {
        let mut app = App::default();

        type_command(&mut app, "midi connect 3");
        assert_eq!(app.midi_status, "MIDI Connecting 3");

        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(app.is_playing);

        type_command(&mut app, "midi panic");
        assert!(!app.is_playing);
        assert_eq!(app.playhead_row, None);
    }

    #[test]
    fn command_mode_can_start_sequence_playback() {
        let mut app = App::default();

        type_command(&mut app, "play sequence");

        assert!(app.is_playing);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, Some(0));

        type_command(&mut app, "stop");

        assert!(!app.is_playing);
        assert_eq!(app.sequence_position, None);
    }

    #[test]
    fn command_mode_can_start_sequence_from_position() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add 2");

        type_command(&mut app, "play sequence 1");

        assert!(app.is_playing);
        assert_eq!(app.pattern_index, 1);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, Some(1));
    }

    #[test]
    fn shift_enter_starts_sequence_from_selected_position() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add 2");
        app.sequence_cursor = 1;

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));

        assert!(app.is_playing);
        assert_eq!(app.pattern_index, 1);
        assert_eq!(app.playhead_row, Some(0));
        assert_eq!(app.sequence_position, Some(1));
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Playing sequence from 1")
        );
    }

    #[test]
    fn command_mode_wq_saves_and_quits() {
        let path =
            std::env::temp_dir().join(format!("salieri-command-wq-{}.salieri", std::process::id()));
        let mut app = App {
            mode: AppMode::Edit,
            project_path: Some(path.clone()),
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        type_command(&mut app, "wq");

        let saved = load_project(&path).expect("saved project loads");
        let _ = std::fs::remove_file(&path);
        assert_eq!(saved, app.song);
        assert!(!app.dirty);
        assert!(app.should_quit);
    }

    #[test]
    fn command_mode_creates_duplicates_selects_and_deletes_patterns() {
        let mut app = App::default();

        type_command(&mut app, "pattern new");
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        type_command(&mut app, "pattern 1");
        assert_eq!(app.pattern_index, 0);

        type_command(&mut app, "pattern duplicate");
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 2);

        enter_command(&mut app, "pattern delete");
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeletePattern {
                pattern_index: 2,
                ..
            })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 1);
    }

    #[test]
    fn bracket_keys_select_previous_and_next_pattern() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");

        assert_eq!(app.pattern_index, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 0);

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 0);

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 2);
    }

    #[test]
    fn uppercase_pattern_shortcuts_create_duplicate_and_delete() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 2);

        app.handle_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT));
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeletePattern {
                pattern_index: 2,
                ..
            })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);
        assert!(app.dirty);
    }

    #[test]
    fn patterns_view_guides_pattern_management_and_presets() {
        let mut app = App {
            cursor: Cursor {
                row: 63,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Patterns);
        assert_eq!(app.tui_active_view(), TuiView::Patterns);

        app.handle_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT));
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 0);
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.pattern_index, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT));
        assert_eq!(app.song.patterns.len(), 3);
        assert_eq!(app.pattern_index, 2);

        app.cursor.row = 63;
        app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.song.patterns[2].row_count(), 16);
        assert_eq!(app.cursor.row, 15);

        app.handle_key(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE));
        assert_eq!(app.song.patterns[2].row_count(), 256);

        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "pattern rename ");
        app.command_buffer.push_str("Breakdown");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.song.patterns[2].name, "Breakdown");

        app.handle_key(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeletePattern {
                pattern_index: 2,
                ..
            })
        ));
    }

    #[test]
    fn sampler_view_opens_without_sample_and_loads_wav_from_command() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::F(11), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Sampler);
        assert_eq!(app.tui_active_view(), TuiView::Sampler);
        assert!(app.tui_sampler_view().is_none());

        let path =
            std::env::temp_dir().join(format!("salieri-sampler-view-{}.wav", std::process::id()));
        std::fs::write(
            &path,
            wav_pcm16_bytes(44_100, 1, &[0, i16::MAX, i16::MIN, 16_384]),
        )
        .expect("write wav");

        enter_command(&mut app, &format!("sample view {}", path.display()));
        let _ = std::fs::remove_file(&path);

        assert_eq!(app.mode, AppMode::Sampler);
        let sampler = app.tui_sampler_view().expect("sampler view");
        assert_eq!(sampler.name, path.file_name().unwrap().to_string_lossy());
        assert_eq!(sampler.overview.sample_rate, 44_100);
        assert_eq!(sampler.overview.channels, 1);
        assert_eq!(sampler.overview.frames, 4);
    }

    #[test]
    fn sampler_commands_assign_list_and_unassign_loaded_sample() {
        let mut app = App::default();
        let path =
            std::env::temp_dir().join(format!("salieri-sampler-assign-{}.wav", std::process::id()));
        std::fs::write(&path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX])).expect("write wav");

        enter_command(&mut app, &format!("sample view {}", path.display()));
        enter_command(&mut app, "sample assign 2");

        let track_id = app.song.tracks[1].id;
        let assignment = app
            .song
            .sample_assignment_for_track(track_id)
            .expect("assignment");
        let sample = app.song.sample_for_id(assignment.sample).expect("sample");
        assert_eq!(sample.name, path.file_name().unwrap().to_string_lossy());
        assert_eq!(sample.path, path.to_string_lossy());
        assert!(app.dirty);

        let sampler = app.tui_sampler_view().expect("sampler view");
        assert_eq!(
            sampler.assigned_track,
            Some(app.song.tracks[1].name.as_str())
        );
        assert_eq!(sampler.assigned_track_count, 1);

        enter_command(&mut app, "sample assignments");
        assert!(app
            .notification
            .as_ref()
            .expect("notification")
            .message
            .contains("Bass"));

        enter_command(&mut app, "sample unassign 2");
        let _ = std::fs::remove_file(&path);

        assert!(app.song.sample_assignment_for_track(track_id).is_none());
    }

    #[test]
    fn sampler_commands_replace_unload_and_cleanup_references() {
        let mut app = App::default();
        let first_path =
            std::env::temp_dir().join(format!("salieri-sampler-first-{}.wav", std::process::id()));
        let second_path =
            std::env::temp_dir().join(format!("salieri-sampler-second-{}.wav", std::process::id()));
        std::fs::write(&first_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MAX]))
            .expect("write first wav");
        std::fs::write(&second_path, wav_pcm16_bytes(44_100, 1, &[0, i16::MIN]))
            .expect("write second wav");

        enter_command(&mut app, &format!("sample view {}", first_path.display()));
        enter_command(&mut app, "sample assign 2");
        let track_id = app.song.tracks[1].id;
        let first_sample = app
            .song
            .sample_assignment_for_track(track_id)
            .expect("first assignment")
            .sample;

        enter_command(&mut app, &format!("sample view {}", second_path.display()));
        enter_command(&mut app, "sample replace 2");
        let second_sample = app
            .song
            .sample_assignment_for_track(track_id)
            .expect("second assignment")
            .sample;

        assert_ne!(first_sample, second_sample);
        assert!(app.song.sample_for_id(first_sample).is_none());
        assert_eq!(
            app.song
                .sample_for_id(second_sample)
                .expect("second sample")
                .path,
            second_path.to_string_lossy()
        );

        enter_command(&mut app, "sample unload");
        assert!(app.song.sample_for_id(second_sample).is_some());
        assert!(app.tui_sampler_view().is_some());
        assert!(app
            .notification
            .as_ref()
            .expect("notification")
            .message
            .contains("Unassign or replace"));

        enter_command(&mut app, "sample unassign 2");
        enter_command(&mut app, "sample cleanup");

        assert!(app.song.sample_for_id(second_sample).is_none());
        let _ = std::fs::remove_file(&first_path);
        let _ = std::fs::remove_file(&second_path);
    }

    #[test]
    fn sample_browser_command_queues_external_request_when_configured() {
        let mut app = App::new(AppConfig {
            sample_browser: SampleBrowserConfig {
                chooser_command: Some("true".to_string()),
                start_dir: Some(PathBuf::from("Samples")),
            },
            ..AppConfig::default()
        });

        enter_command(&mut app, "sample browse Drums");

        assert_eq!(app.mode, AppMode::Sampler);
        let (config, request) = app.take_sample_browser_request().expect("browser request");
        assert_eq!(config.chooser_command, Some("true".to_string()));
        assert_eq!(request.start_dir, Some(PathBuf::from("Drums")));
    }

    #[test]
    fn sample_browser_command_warns_without_configuration() {
        let mut app = App::default();

        enter_command(&mut app, "sample browse");

        assert_eq!(app.mode, AppMode::Sampler);
        assert!(app.take_sample_browser_request().is_none());
        assert_eq!(
            app.notification
                .as_ref()
                .map(|value| value.message.as_str()),
            Some("Sample browser not configured")
        );
    }

    #[test]
    fn external_sample_browser_reads_selected_path_from_chooser_file() {
        let selected = run_external_sample_browser(
            &SampleBrowserConfig {
                chooser_command: Some(
                    "printf '%s\n' \"$SALIERI_SAMPLE_START_DIR/pick.wav\" > \"$SALIERI_CHOOSER_FILE\""
                        .to_string(),
                ),
                start_dir: Some(PathBuf::from("Samples")),
            },
            &SampleBrowserRequest { start_dir: None },
        )
        .expect("run browser");

        assert_eq!(selected, Some(PathBuf::from("Samples/pick.wav")));
    }

    #[test]
    fn command_mode_renames_current_pattern() {
        let mut app = App::default();

        type_command(&mut app, "pattern rename Intro Verse");

        assert_eq!(app.song.patterns[0].name, "Intro Verse");
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern renamed")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.patterns[0].name, "Pattern 01");
    }

    #[test]
    fn f3_prefills_current_pattern_rename_command() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "pattern rename ");

        for value in "Intro Verse".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.song.patterns[0].name, "Intro Verse");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern renamed")
        );
    }

    #[test]
    fn command_mode_reports_invalid_pattern_rename() {
        let mut app = App::default();

        type_command(&mut app, "pattern rename     ");

        assert_eq!(app.song.patterns[0].name, "Pattern 01");
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern rename failed: name cannot be empty")
        );
    }

    #[test]
    fn command_mode_resizes_current_pattern_and_clamps_cursor() {
        let mut app = App {
            cursor: Cursor {
                row: 63,
                ..Cursor::new()
            },
            row_offset: 44,
            ..App::default()
        };

        type_command(&mut app, "pattern length 16");

        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 16);
        assert_eq!(app.cursor.row, 15);
        assert_eq!(app.row_offset, 15);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern length set to 16")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
    }

    #[test]
    fn f6_prefills_current_pattern_length_command() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "pattern length ");

        for value in "32".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 32);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern length set to 32")
        );
    }

    #[test]
    fn command_mode_reports_invalid_pattern_length() {
        let mut app = App::default();

        type_command(&mut app, "pattern length 0");

        assert_eq!(app.song.current_pattern().expect("pattern").row_count(), 64);
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern length failed: invalid pattern length: 0")
        );
    }

    #[test]
    fn command_mode_adds_and_removes_sequence_positions() {
        let mut app = App::default();

        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add");
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence added pattern 02")
        );

        type_command(&mut app, "sequence remove 0");
        assert_eq!(app.song.sequence, vec![salieri_core::PatternId(2)]);
        assert_eq!(app.song.patterns.len(), 2);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence removed position 00")
        );
    }

    #[test]
    fn command_mode_reports_sequence_add_pattern_out_of_range() {
        let mut app = App::default();

        type_command(&mut app, "sequence add 99");

        assert_eq!(app.song.sequence, vec![salieri_core::PatternId(1)]);
        assert!(!app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern out of range")
        );
    }

    #[test]
    fn command_mode_duplicates_sets_and_moves_sequence_positions() {
        let mut app = App::default();

        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");
        type_command(&mut app, "sequence add 2");
        type_command(&mut app, "sequence add 3");

        type_command(&mut app, "sequence duplicate 1");
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2),
                salieri_core::PatternId(3)
            ]
        );
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence duplicated position 01")
        );

        type_command(&mut app, "sequence set 0 3");
        assert_eq!(app.song.sequence[0], salieri_core::PatternId(3));
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence position 00 set to pattern 03")
        );

        type_command(&mut app, "sequence move 3 1");
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(3),
                salieri_core::PatternId(3),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2)
            ]
        );
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence moved position 03 to 01")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.sequence[1], salieri_core::PatternId(2));
    }

    #[test]
    fn keyboard_sequence_shortcuts_edit_selected_position() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");
        app.pattern_index = 1;

        app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(app.sequence_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char(','), KeyModifiers::NONE));
        assert_eq!(app.sequence_cursor, 0);
        app.handle_key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
        assert_eq!(app.sequence_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2)
            ]
        );
        assert_eq!(app.sequence_cursor, 2);

        app.pattern_index = 2;
        app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert_eq!(app.song.sequence[2], salieri_core::PatternId(3));

        app.handle_key(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::SHIFT));
        assert_eq!(app.sequence_cursor, 1);
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(3),
                salieri_core::PatternId(2)
            ]
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::SHIFT));
        assert_eq!(app.sequence_cursor, 2);
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(3)
            ]
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(app.sequence_cursor, 1);
        assert!(app.dirty);
    }

    #[test]
    fn sequence_view_navigation_edits_and_playback() {
        let mut app = App::default();
        type_command(&mut app, "pattern new");
        type_command(&mut app, "pattern new");
        app.pattern_index = 1;

        app.handle_key(KeyEvent::new(KeyCode::F(7), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Sequence);
        assert_eq!(app.tui_active_view(), TuiView::Sequence);

        app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![salieri_core::PatternId(1), salieri_core::PatternId(2)]
        );
        assert_eq!(app.sequence_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.sequence_cursor, 0);
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.sequence_cursor, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT));
        assert_eq!(
            app.song.sequence,
            vec![
                salieri_core::PatternId(1),
                salieri_core::PatternId(2),
                salieri_core::PatternId(2)
            ]
        );
        assert_eq!(app.sequence_cursor, 2);

        app.pattern_index = 2;
        app.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert_eq!(app.song.sequence[2], salieri_core::PatternId(3));

        app.handle_key(KeyEvent::new(KeyCode::Char('<'), KeyModifiers::SHIFT));
        assert_eq!(app.sequence_cursor, 1);
        app.handle_key(KeyEvent::new(KeyCode::Char('>'), KeyModifiers::SHIFT));
        assert_eq!(app.sequence_cursor, 2);

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_playing);
        assert_eq!(app.sequence_position, Some(2));

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.tui_active_view(), TuiView::Pattern);
    }

    #[test]
    fn command_mode_reports_sequence_position_errors() {
        let mut app = App::default();

        type_command(&mut app, "sequence remove 99");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence remove failed: sequence out of bounds: position 99")
        );
        assert!(!app.dirty);

        type_command(&mut app, "sequence duplicate 99");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence duplicate failed: sequence out of bounds: position 99")
        );

        type_command(&mut app, "sequence set 99 1");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence set failed: sequence out of bounds: position 99")
        );

        type_command(&mut app, "sequence set 0 99");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Pattern out of range")
        );

        type_command(&mut app, "sequence move 99 0");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Sequence move failed: sequence out of bounds: position 99")
        );
    }

    #[test]
    fn help_mode_opens_and_closes_without_mutating_state() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Help);
        assert_eq!(app.cursor.row, 0);
        assert!(!app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);

        app.handle_key(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));
        assert_eq!(app.mode, AppMode::Help);
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Help);
    }

    #[test]
    fn ctrl_t_creates_track_and_undo_restores_previous_shape() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.cursor.track, 4);
        assert!(app.dirty);
        assert!(app
            .song
            .current_pattern()
            .expect("pattern")
            .rows
            .iter()
            .all(|row| row.cells.len() == 5));

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.cursor.track, 3);
        assert!(!app.dirty);
    }

    #[test]
    fn command_mode_duplicates_track_and_undo_restores_previous_shape() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x64)
            .expect("set note");

        type_command(&mut app, "track duplicate");

        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.song.tracks[4].name, "Bass Copy");
        assert_eq!(app.cursor.track, 4);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 4)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.cursor.track, 3);
    }

    #[test]
    fn uppercase_d_duplicates_current_track() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));

        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.song.tracks[4].name, "Bass Copy");
        assert_eq!(app.cursor.track, 4);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_moves_track_and_undo_restores_order() {
        let mut app = App::default();
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
            .expect("set bass note");
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 2, NoteEvent::Note { pitch: 64 }, 0x70)
            .expect("set lead note");

        type_command(&mut app, "track move 2 3");

        assert_eq!(app.song.tracks[1].name, "Lead");
        assert_eq!(app.song.tracks[2].name, "Bass");
        assert_eq!(app.cursor.track, 2);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("lead cell")
                .note,
            Some(NoteEvent::Note { pitch: 64 })
        );
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 2)
                .expect("bass cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks[1].name, "Bass");
        assert_eq!(app.song.tracks[2].name, "Lead");
    }

    #[test]
    fn brace_shortcuts_move_current_track_left_and_right() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };
        app.song
            .current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 48 }, 0x60)
            .expect("set bass note");

        app.handle_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::SHIFT));

        assert_eq!(app.song.tracks[0].name, "Bass");
        assert_eq!(app.cursor.track, 0);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('}'), KeyModifiers::SHIFT));

        assert_eq!(app.song.tracks[1].name, "Bass");
        assert_eq!(app.cursor.track, 1);
        assert_eq!(
            app.song
                .current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 48 })
        );
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_deletes_numbered_track_after_confirmation() {
        let mut app = App::default();

        enter_command(&mut app, "track delete 2");

        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeleteTrack { track_index: 1, .. })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.song.tracks.len(), 3);
        assert_eq!(app.song.tracks[1].name, "Lead");
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.song.tracks[1].name, "Bass");
    }

    #[test]
    fn delete_in_normal_mode_removes_current_track_and_cells() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeleteTrack { track_index: 1, .. })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

        assert_eq!(app.song.tracks.len(), 3);
        assert_eq!(app.song.tracks[1].name, "Lead");
        assert_eq!(app.cursor.track, 1);
        assert!(app
            .song
            .current_pattern()
            .expect("pattern")
            .rows
            .iter()
            .all(|row| row.cells.len() == 3));
    }

    #[test]
    fn delete_track_dialog_can_be_cancelled() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.song.tracks.len(), 4);
        assert_eq!(app.song.tracks[1].name, "Bass");
    }

    #[test]
    fn tracks_view_guides_track_management() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Tracks);
        assert_eq!(app.tui_active_view(), TuiView::Tracks);

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.cursor.track, 1);

        app.handle_key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.cursor.track, 4);
        assert_eq!(app.song.tracks[4].name, "Bass Copy");

        app.handle_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::SHIFT));
        assert_eq!(app.cursor.track, 3);

        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert!(app.song.tracks[3].muted);
        assert!(app.song.tracks[3].solo);

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "track channel 4 ");
        app.command_buffer.push('9');
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.song.tracks[3].midi_channel, 9);

        app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "track rename 4 ");
        app.command_buffer.push_str("Aux Bass");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.song.tracks[3].name, "Aux Bass");

        app.handle_key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(matches!(
            app.dialog,
            Some(Dialog::DeleteTrack { track_index: 3, .. })
        ));

        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(app.song.tracks.len(), 5);
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn cannot_delete_last_track_from_app() {
        let mut app = App::default();

        while app.song.tracks.len() > 1 {
            app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
            app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        assert_eq!(app.song.tracks.len(), 1);
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Cannot delete the last track")
        );
    }

    #[test]
    fn mute_and_solo_commands_toggle_current_track() {
        let mut app = App {
            cursor: Cursor {
                track: 2,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

        assert!(app.song.tracks[2].muted);
        assert!(app.song.tracks[2].solo);
        assert!(app.dirty);
    }

    #[test]
    fn command_mode_mutes_and_solos_numbered_track() {
        let mut app = App::default();

        type_command(&mut app, "track mute 2");
        type_command(&mut app, "track solo 2");

        assert!(app.song.tracks[1].muted);
        assert!(app.song.tracks[1].solo);
        assert!(app.dirty);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert!(app.song.tracks[1].muted);
        assert!(!app.song.tracks[1].solo);

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert!(!app.song.tracks[1].muted);
    }

    #[test]
    fn command_mode_changes_current_or_named_track_midi_channel() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        type_command(&mut app, "track channel 12");
        assert_eq!(app.song.tracks[1].midi_channel, 12);
        assert!(app.dirty);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track channel set to 12")
        );

        type_command(&mut app, "track channel 3 15");
        assert_eq!(app.song.tracks[2].midi_channel, 15);

        type_command(&mut app, "track channel 3 0");
        assert_eq!(app.song.tracks[2].midi_channel, 15);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track channel failed: invalid MIDI channel: 0")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.tracks[2].midi_channel, 2);
    }

    #[test]
    fn command_mode_renames_current_or_named_track() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        type_command(&mut app, "track rename Acid Bass");
        assert_eq!(app.song.tracks[1].name, "Acid Bass");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track renamed")
        );

        type_command(&mut app, "track rename 3 Main Lead");
        assert_eq!(app.song.tracks[2].name, "Main Lead");

        type_command(&mut app, "track rename 3    ");
        assert_eq!(app.song.tracks[2].name, "Main Lead");
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track rename failed: name cannot be empty")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
        assert_eq!(app.song.tracks[2].name, "Lead");
    }

    #[test]
    fn r_prefills_current_track_rename_command() {
        let mut app = App {
            cursor: Cursor {
                track: 1,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "track rename 2 ");

        for value in "Sub Bass".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.song.tracks[1].name, "Sub Bass");
    }

    #[test]
    fn c_prefills_current_track_channel_command() {
        let mut app = App {
            cursor: Cursor {
                track: 2,
                ..Cursor::new()
            },
            ..App::default()
        };

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));

        assert_eq!(app.mode, AppMode::Command);
        assert_eq!(app.command_buffer, "track channel 3 ");

        for value in "12".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(app.song.tracks[2].midi_channel, 12);
        assert_eq!(
            app.notification.as_ref().map(|n| n.message.as_str()),
            Some("Track channel set to 12")
        );
    }

    #[test]
    fn f1_and_f2_change_octave_in_normal_mode() {
        let mut app = App::default();

        app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        assert_eq!(app.octave, 5);
        assert_eq!(app.mode, AppMode::Normal);

        app.handle_key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE));
        assert_eq!(app.octave, 4);
    }

    fn type_command(app: &mut App, command: &str) {
        enter_command(app, command);
        assert_eq!(app.mode, AppMode::Normal);
    }

    fn enter_command(app: &mut App, command: &str) {
        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE));
        assert_eq!(app.mode, AppMode::Command);
        for value in command.chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }

    fn wav_pcm16_bytes(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data_size = samples.len() * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * u32::from(channels) * 2;
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        let block_align = channels * 2;
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data_size as u32).to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }
}
