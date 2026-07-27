use super::cli::{parse_next_usize, parse_usize_value, AnalysisOutputFormat};
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MusicXmlExportArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) pattern: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ImportMusicXmlArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RoundTripValidationArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) pattern: usize,
    pub(crate) format: AnalysisOutputFormat,
}

impl Default for MusicXmlExportArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
        }
    }
}

impl Default for RoundTripValidationArgs {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            pattern: 1,
            format: AnalysisOutputFormat::Text,
        }
    }
}

pub(crate) fn parse_musicxml_export_args(
    args: impl IntoIterator<Item = String>,
) -> MusicXmlExportArgs {
    let mut parsed = MusicXmlExportArgs::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pattern" => parse_next_usize(&mut args, &mut parsed.pattern),
            _ if arg.starts_with("--pattern=") => {
                parse_usize_value(arg.trim_start_matches("--pattern="), &mut parsed.pattern);
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed.pattern = parsed.pattern.max(1);
    parsed
}

pub(crate) fn parse_import_musicxml_args(
    args: impl IntoIterator<Item = String>,
) -> ImportMusicXmlArgs {
    let mut parsed = ImportMusicXmlArgs::default();
    for arg in args {
        if parsed.input_path.is_none() {
            parsed.input_path = Some(PathBuf::from(arg));
        } else if parsed.output_path.is_none() {
            parsed.output_path = Some(PathBuf::from(arg));
        }
    }
    parsed
}

pub(crate) fn parse_round_trip_validation_args(
    args: impl IntoIterator<Item = String>,
) -> RoundTripValidationArgs {
    let mut parsed = RoundTripValidationArgs::default();
    let mut expect_format = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if expect_format {
            parse_analysis_format(&arg, &mut parsed.format);
            expect_format = false;
            continue;
        }
        match arg.as_str() {
            "--pattern" => parse_next_usize(&mut args, &mut parsed.pattern),
            "--format" => expect_format = true,
            _ if arg.starts_with("--pattern=") => {
                parse_usize_value(arg.trim_start_matches("--pattern="), &mut parsed.pattern);
            }
            _ if arg.starts_with("--format=") => {
                parse_analysis_format(arg.trim_start_matches("--format="), &mut parsed.format);
            }
            _ if parsed.input_path.is_none() => parsed.input_path = Some(PathBuf::from(arg)),
            _ if parsed.output_path.is_none() => parsed.output_path = Some(PathBuf::from(arg)),
            _ => {}
        }
    }
    parsed.pattern = parsed.pattern.max(1);
    parsed
}

fn parse_analysis_format(value: &str, target: &mut AnalysisOutputFormat) {
    match value {
        "text" => *target = AnalysisOutputFormat::Text,
        "json" => *target = AnalysisOutputFormat::Json,
        _ => {}
    }
}
