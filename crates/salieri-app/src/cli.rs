use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliArgs {
    pub(crate) command: CliCommand,
    pub(crate) project_path: Option<PathBuf>,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) log_level: Option<String>,
    pub(crate) midi_log_path: Option<PathBuf>,
    pub(crate) midi_test: MidiTestArgs,
}

impl CliArgs {
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Self {
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
                "--list-midi-inputs" => {
                    return Self {
                        command: CliCommand::ListMidiInputs,
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
                "export" => {
                    return Self {
                        command: parse_export_command(args),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "report" => {
                    return Self {
                        command: parse_report_command(args),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "analyze" | "analysis" => {
                    return Self {
                        command: CliCommand::Analyze(parse_analysis_args(args)),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "compare" => {
                    return Self {
                        command: CliCommand::Compare(parse_compare_args(args)),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "graph" | "composition-graph" => {
                    return Self {
                        command: parse_graph_command(args),
                        project_path: None,
                        config_path,
                        log_level,
                        midi_log_path,
                        midi_test,
                    }
                }
                "import" => {
                    return Self {
                        command: parse_import_command(args),
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
pub(crate) struct MidiTestArgs {
    pub(crate) output: Option<String>,
    pub(crate) channel: u8,
    pub(crate) note: u8,
    pub(crate) duration_ms: u64,
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
pub(crate) enum CliCommand {
    Run,
    Help,
    Version,
    ListMidiOutputs,
    ListMidiInputs,
    MidiTest,
    TransformEuclidean(TransformEuclideanArgs),
    SampleInspect(SampleInspectArgs),
    ExportPlan(RenderPlanArgs),
    ExportAudio(AudioExportArgs),
    ExportStems(RenderStemsArgs),
    ExportStrudel(StrudelExportArgs),
    ReportProject(ReportArgs),
    ReportCritique(ReportArgs),
    Analyze(AnalysisArgs),
    Compare(CompareArgs),
    GraphValidate(GraphValidateArgs),
    GraphCompile(GraphCompileArgs),
    ImportXrns(ImportXrnsArgs),
}

pub(crate) fn print_help() {
    println!(
        "Salieri Tracker\n\nUsage:\n  salieri [OPTIONS] [FILE]\n  salieri --list-midi-outputs\n  salieri --list-midi-inputs\n  salieri --midi-test-output NAME_OR_INDEX [OPTIONS]\n  salieri transform euclidean INPUT OUTPUT [OPTIONS]\n  salieri sample inspect FILE [OPTIONS]\n  salieri import xrns INPUT OUTPUT [OPTIONS]\n  salieri export plan INPUT [OUTPUT.json] [OPTIONS]\n  salieri export audio INPUT OUTPUT.wav [OPTIONS]\n  salieri export stems INPUT OUT_DIR [OPTIONS]\n  salieri export strudel INPUT [OUTPUT.js] [OPTIONS]\n  salieri report project INPUT [OUTPUT.md]\n  salieri report critique INPUT [OUTPUT.md]\n  salieri analyze INPUT [OUTPUT] [--format text|json]\n  salieri compare LEFT RIGHT [OUTPUT] [--format text|json]\n  salieri graph validate GRAPH.json\n  salieri graph compile GRAPH.json INPUT.salieri OUTPUT.salieri\n  salieri --help\n  salieri --version\n\nOptions:\n  --config PATH                 Load config from PATH\n  --log-level LEVEL             Set tracing filter, e.g. debug or salieri=debug\n  --midi-log PATH               Write sent MIDI messages to PATH\n  --list-midi-outputs           List available MIDI output ports\n  --list-midi-inputs            List available MIDI input ports\n  --midi-test-output VALUE      Send one test note to a MIDI output name or index\n  --midi-test-channel CHANNEL   Test channel, 1-16 (default 1)\n  --midi-test-note NOTE         Test MIDI note, 0-127 (default 60)\n  --midi-test-duration-ms MS    Test note length (default 1000)\n\nTransform options:\n  --pattern N                   1-based pattern index (default 1)\n  --track N                     1-based track index (default 1)\n  --steps N                     Euclidean step count (default 16)\n  --pulses N                    Euclidean pulse count (default 4)\n  --rotation N                  Euclidean rotation (default 0)\n  --pitch NOTE                  MIDI note, 0-127 (default 36)\n  --velocity VALUE              Velocity, 0-127 (default 100)\n\nSample inspect options:\n  --format text|json            Output format (default text)\n  --buckets N, --width N        Waveform bucket count (default 64)\n\nAnalysis options:\n  --format text|json            Output format for analyze/compare (default text)\n\nImport options:\n  salieri import xrns INPUT OUTPUT imports an XRNS subset and writes a .salieri project\n  --sample-dir DIR              Extract supported XRNS WAV payloads into DIR\n  --sample-path-prefix PREFIX   Store extracted sample paths with PREFIX in the project\n  --convert-samples-to-wav      Convert FLAC/OGG/AIF payloads to WAV with ffmpeg\n\nRender/export options:\n  --pattern N                   Export 1-based pattern index (default 1)\n  --patterns LIST               Comma-separated 1-based patterns for Strudel\n  --sequence                    Export the full sequence instead of one pattern\n  --tracks LIST                 Comma-separated 1-based tracks for plans/stems\n  --sample-rate HZ              Output sample rate (default 48000)\n  --channels N                  Output channels (default 2)\n\n  --help                        Show this help\n  --version                     Show version"
    );
}

pub(crate) fn parse_transform_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("euclidean") => CliCommand::TransformEuclidean(parse_transform_euclidean_args(args)),
        _ => CliCommand::Help,
    }
}

pub(crate) fn parse_sample_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("inspect") => CliCommand::SampleInspect(parse_sample_inspect_args(args)),
        _ => CliCommand::Help,
    }
}

pub(crate) fn parse_export_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("plan") | Some("render-plan") => CliCommand::ExportPlan(parse_render_plan_args(args)),
        Some("audio") => CliCommand::ExportAudio(parse_audio_export_args(args)),
        Some("stems") => CliCommand::ExportStems(parse_render_stems_args(args)),
        Some("strudel") => CliCommand::ExportStrudel(parse_strudel_export_args(args)),
        _ => CliCommand::Help,
    }
}

pub(crate) fn parse_import_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("xrns") => CliCommand::ImportXrns(parse_import_xrns_args(args)),
        _ => CliCommand::Help,
    }
}

pub(crate) fn parse_report_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("project") | Some("summary") => CliCommand::ReportProject(parse_report_args(args)),
        Some("critique") | Some("review") => CliCommand::ReportCritique(parse_report_args(args)),
        _ => CliCommand::Help,
    }
}

pub(crate) fn parse_graph_command(args: impl IntoIterator<Item = String>) -> CliCommand {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("validate") | Some("check") => {
            CliCommand::GraphValidate(parse_graph_validate_args(args))
        }
        Some("compile") => CliCommand::GraphCompile(parse_graph_compile_args(args)),
        _ => CliCommand::Help,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransformEuclideanArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) pattern: usize,
    pub(crate) track: usize,
    pub(crate) steps: usize,
    pub(crate) pulses: usize,
    pub(crate) rotation: usize,
    pub(crate) pitch: u8,
    pub(crate) velocity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SampleInspectFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SampleInspectArgs {
    pub(crate) path: Option<PathBuf>,
    pub(crate) format: SampleInspectFormat,
    pub(crate) buckets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AudioExportArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) pattern: usize,
    pub(crate) sequence: bool,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderPlanArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) pattern: usize,
    pub(crate) sequence: bool,
    pub(crate) tracks: Vec<usize>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderStemsArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_dir: Option<PathBuf>,
    pub(crate) pattern: usize,
    pub(crate) sequence: bool,
    pub(crate) tracks: Vec<usize>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StrudelExportArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) pattern: usize,
    pub(crate) patterns: Vec<usize>,
    pub(crate) sequence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReportArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalysisOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalysisArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) format: AnalysisOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompareArgs {
    pub(crate) left_path: Option<PathBuf>,
    pub(crate) right_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) format: AnalysisOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GraphValidateArgs {
    pub(crate) graph_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GraphCompileArgs {
    pub(crate) graph_path: Option<PathBuf>,
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
}

impl Default for AnalysisArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            format: AnalysisOutputFormat::Text,
        }
    }
}

impl Default for CompareArgs {
    fn default() -> Self {
        Self {
            left_path: None,
            right_path: None,
            output_path: None,
            format: AnalysisOutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ImportXrnsArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) sample_dir: Option<PathBuf>,
    pub(crate) sample_path_prefix: Option<String>,
    pub(crate) convert_samples_to_wav: bool,
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
pub(crate) struct SampleInspection {
    pub(crate) sample: Sample,
    pub(crate) overview: WaveformOverview,
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

impl Default for AudioExportArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
            sequence: false,
            sample_rate: AudioConfig::default().sample_rate,
            channels: AudioConfig::default().channels,
        }
    }
}

impl Default for RenderPlanArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
            sequence: false,
            tracks: Vec::new(),
            sample_rate: AudioConfig::default().sample_rate,
            channels: AudioConfig::default().channels,
        }
    }
}

impl Default for RenderStemsArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_dir: None,
            pattern: 1,
            sequence: false,
            tracks: Vec::new(),
            sample_rate: AudioConfig::default().sample_rate,
            channels: AudioConfig::default().channels,
        }
    }
}

impl Default for StrudelExportArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
            patterns: Vec::new(),
            sequence: false,
        }
    }
}

pub(crate) fn parse_transform_euclidean_args(
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

pub(crate) fn parse_sample_inspect_args(
    args: impl IntoIterator<Item = String>,
) -> SampleInspectArgs {
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

pub(crate) fn parse_audio_export_args(args: impl IntoIterator<Item = String>) -> AudioExportArgs {
    let mut parsed = AudioExportArgs::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pattern" => parse_next_usize(&mut args, &mut parsed.pattern),
            "--sequence" => parsed.sequence = true,
            "--sample-rate" => parse_next_u32(&mut args, &mut parsed.sample_rate),
            "--channels" => parse_next_u16(&mut args, &mut parsed.channels),
            _ if arg.starts_with("--pattern=") => {
                parse_usize_value(arg.trim_start_matches("--pattern="), &mut parsed.pattern);
            }
            _ if arg.starts_with("--sample-rate=") => {
                parse_u32_value(
                    arg.trim_start_matches("--sample-rate="),
                    &mut parsed.sample_rate,
                );
            }
            _ if arg.starts_with("--channels=") => {
                parse_u16_value(arg.trim_start_matches("--channels="), &mut parsed.channels);
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }

    parsed.pattern = parsed.pattern.max(1);
    parsed.sample_rate = parsed.sample_rate.max(1);
    parsed.channels = parsed.channels.max(1);
    parsed
}

pub(crate) fn parse_render_plan_args(args: impl IntoIterator<Item = String>) -> RenderPlanArgs {
    let common = parse_render_common_args(args);
    RenderPlanArgs {
        input_path: common.input_path,
        output_path: common.output_path,
        pattern: common.pattern,
        sequence: common.sequence,
        tracks: common.tracks,
        sample_rate: common.sample_rate,
        channels: common.channels,
    }
}

pub(crate) fn parse_render_stems_args(args: impl IntoIterator<Item = String>) -> RenderStemsArgs {
    let common = parse_render_common_args(args);
    RenderStemsArgs {
        input_path: common.input_path,
        output_dir: common.output_path,
        pattern: common.pattern,
        sequence: common.sequence,
        tracks: common.tracks,
        sample_rate: common.sample_rate,
        channels: common.channels,
    }
}

pub(crate) fn parse_strudel_export_args(
    args: impl IntoIterator<Item = String>,
) -> StrudelExportArgs {
    let mut parsed = StrudelExportArgs::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pattern" => parse_next_usize(&mut args, &mut parsed.pattern),
            "--patterns" => {
                if let Some(value) = args.next() {
                    parsed.patterns = parse_track_list(&value);
                }
            }
            "--sequence" => parsed.sequence = true,
            _ if arg.starts_with("--pattern=") => {
                parse_usize_value(arg.trim_start_matches("--pattern="), &mut parsed.pattern);
            }
            _ if arg.starts_with("--patterns=") => {
                parsed.patterns = parse_track_list(arg.trim_start_matches("--patterns="));
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed.pattern = parsed.pattern.max(1);
    parsed
}

pub(crate) fn parse_report_args(args: impl IntoIterator<Item = String>) -> ReportArgs {
    let mut parsed = ReportArgs::default();
    for arg in args {
        if parsed.input_path.is_none() {
            parsed.input_path = Some(PathBuf::from(arg));
        } else if parsed.output_path.is_none() {
            parsed.output_path = Some(PathBuf::from(arg));
        }
    }
    parsed
}

pub(crate) fn parse_analysis_args(args: impl IntoIterator<Item = String>) -> AnalysisArgs {
    let mut parsed = AnalysisArgs::default();
    for arg in args {
        match arg.as_str() {
            "--format" => {}
            "text" if parsed.input_path.is_some() => parsed.format = AnalysisOutputFormat::Text,
            "json" if parsed.input_path.is_some() => parsed.format = AnalysisOutputFormat::Json,
            _ if arg.starts_with("--format=") => {
                parse_analysis_format(arg.trim_start_matches("--format="), &mut parsed.format);
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed
}

pub(crate) fn parse_compare_args(args: impl IntoIterator<Item = String>) -> CompareArgs {
    let mut parsed = CompareArgs::default();
    let mut expect_format = false;
    for arg in args {
        if expect_format {
            parse_analysis_format(&arg, &mut parsed.format);
            expect_format = false;
            continue;
        }
        match arg.as_str() {
            "--format" => expect_format = true,
            _ if arg.starts_with("--format=") => {
                parse_analysis_format(arg.trim_start_matches("--format="), &mut parsed.format);
            }
            _ if parsed.left_path.is_none() => parsed.left_path = Some(PathBuf::from(arg)),
            _ if parsed.right_path.is_none() => parsed.right_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed
}

fn parse_analysis_format(value: &str, target: &mut AnalysisOutputFormat) {
    match value {
        "text" => *target = AnalysisOutputFormat::Text,
        "json" => *target = AnalysisOutputFormat::Json,
        _ => {}
    }
}

pub(crate) fn parse_graph_validate_args(
    args: impl IntoIterator<Item = String>,
) -> GraphValidateArgs {
    let mut parsed = GraphValidateArgs::default();
    for arg in args {
        if parsed.graph_path.is_none() {
            parsed.graph_path = Some(PathBuf::from(arg));
        }
    }
    parsed
}

pub(crate) fn parse_graph_compile_args(args: impl IntoIterator<Item = String>) -> GraphCompileArgs {
    let mut parsed = GraphCompileArgs::default();
    for arg in args {
        if parsed.graph_path.is_none() {
            parsed.graph_path = Some(PathBuf::from(arg));
        } else if parsed.input_path.is_none() {
            parsed.input_path = Some(PathBuf::from(arg));
        } else if parsed.output_path.is_none() {
            parsed.output_path = Some(PathBuf::from(arg));
        }
    }
    parsed
}

struct RenderCommonArgs {
    input_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    pattern: usize,
    sequence: bool,
    tracks: Vec<usize>,
    sample_rate: u32,
    channels: u16,
}

impl Default for RenderCommonArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
            sequence: false,
            tracks: Vec::new(),
            sample_rate: AudioConfig::default().sample_rate,
            channels: AudioConfig::default().channels,
        }
    }
}

fn parse_render_common_args(args: impl IntoIterator<Item = String>) -> RenderCommonArgs {
    let mut parsed = RenderCommonArgs::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pattern" => parse_next_usize(&mut args, &mut parsed.pattern),
            "--sequence" => parsed.sequence = true,
            "--tracks" => {
                if let Some(value) = args.next() {
                    parsed.tracks = parse_track_list(&value);
                }
            }
            "--sample-rate" => parse_next_u32(&mut args, &mut parsed.sample_rate),
            "--channels" => parse_next_u16(&mut args, &mut parsed.channels),
            _ if arg.starts_with("--pattern=") => {
                parse_usize_value(arg.trim_start_matches("--pattern="), &mut parsed.pattern);
            }
            _ if arg.starts_with("--tracks=") => {
                parsed.tracks = parse_track_list(arg.trim_start_matches("--tracks="));
            }
            _ if arg.starts_with("--sample-rate=") => {
                parse_u32_value(
                    arg.trim_start_matches("--sample-rate="),
                    &mut parsed.sample_rate,
                );
            }
            _ if arg.starts_with("--channels=") => {
                parse_u16_value(arg.trim_start_matches("--channels="), &mut parsed.channels);
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed.pattern = parsed.pattern.max(1);
    parsed.sample_rate = parsed.sample_rate.max(1);
    parsed.channels = parsed.channels.max(1);
    parsed
}

fn parse_track_list(value: &str) -> Vec<usize> {
    value
        .split(',')
        .filter_map(|part| part.trim().parse::<usize>().ok())
        .filter(|track| *track > 0)
        .collect()
}

pub(crate) fn parse_import_xrns_args(args: impl IntoIterator<Item = String>) -> ImportXrnsArgs {
    let mut parsed = ImportXrnsArgs::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sample-dir" => {
                if let Some(path) = args.next() {
                    parsed.sample_dir = Some(PathBuf::from(path));
                }
            }
            "--sample-path-prefix" => {
                if let Some(prefix) = args.next() {
                    parsed.sample_path_prefix = Some(prefix);
                }
            }
            "--convert-samples-to-wav" => parsed.convert_samples_to_wav = true,
            _ if arg.starts_with("--sample-dir=") => {
                parsed.sample_dir = Some(PathBuf::from(arg.trim_start_matches("--sample-dir=")));
            }
            _ if arg.starts_with("--sample-path-prefix=") => {
                parsed.sample_path_prefix =
                    Some(arg.trim_start_matches("--sample-path-prefix=").to_string());
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed
}

pub(crate) fn parse_sample_format(value: &str, target: &mut SampleInspectFormat) {
    match value {
        "text" => *target = SampleInspectFormat::Text,
        "json" => *target = SampleInspectFormat::Json,
        _ => {}
    }
}

pub(crate) fn parse_next_usize(args: &mut impl Iterator<Item = String>, target: &mut usize) {
    if let Some(value) = args.next() {
        parse_usize_value(&value, target);
    }
}

pub(crate) fn parse_usize_value(value: &str, target: &mut usize) {
    if let Ok(parsed) = value.parse::<usize>() {
        *target = parsed;
    }
}

pub(crate) fn parse_next_u8(args: &mut impl Iterator<Item = String>, target: &mut u8) {
    if let Some(value) = args.next() {
        parse_u8_value(&value, target);
    }
}

pub(crate) fn parse_u8_value(value: &str, target: &mut u8) {
    if let Ok(parsed) = value.parse::<u8>() {
        *target = parsed.min(127);
    }
}

pub(crate) fn parse_next_u32(args: &mut impl Iterator<Item = String>, target: &mut u32) {
    if let Some(value) = args.next() {
        parse_u32_value(&value, target);
    }
}

pub(crate) fn parse_u32_value(value: &str, target: &mut u32) {
    if let Ok(parsed) = value.parse::<u32>() {
        *target = parsed;
    }
}

pub(crate) fn parse_next_u16(args: &mut impl Iterator<Item = String>, target: &mut u16) {
    if let Some(value) = args.next() {
        parse_u16_value(&value, target);
    }
}

pub(crate) fn parse_u16_value(value: &str, target: &mut u16) {
    if let Ok(parsed) = value.parse::<u16>() {
        *target = parsed;
    }
}
