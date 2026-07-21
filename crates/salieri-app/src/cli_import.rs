use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ImportXrnsArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) sample_dir: Option<PathBuf>,
    pub(crate) sample_path_prefix: Option<String>,
    pub(crate) convert_samples_to_wav: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ImportMidiArgs {
    pub(crate) input_path: Option<PathBuf>,
    pub(crate) output_path: Option<PathBuf>,
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

pub(crate) fn parse_import_midi_args(args: impl IntoIterator<Item = String>) -> ImportMidiArgs {
    let mut parsed = ImportMidiArgs::default();
    for arg in args {
        if parsed.input_path.is_none() {
            parsed.input_path = Some(PathBuf::from(arg));
        } else if parsed.output_path.is_none() {
            parsed.output_path = Some(PathBuf::from(arg));
        }
    }
    parsed
}
