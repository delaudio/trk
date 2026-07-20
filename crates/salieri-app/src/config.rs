use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::keymap;

mod preferences;

pub use crate::keymap::KeymapConfig;
pub use preferences::{
    AudioPreferences, DisplayMode, HistoryConfig, ThemeConfig, UiConfig, WorkspaceConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub keyboard: KeyboardConfig,
    pub keymap: KeymapConfig,
    pub ui: UiConfig,
    pub theme: ThemeConfig,
    pub audio: AudioPreferences,
    pub midi: MidiConfig,
    pub sample_browser: SampleBrowserConfig,
    pub project_browser: ProjectBrowserConfig,
    pub workspace: WorkspaceConfig,
    pub history: HistoryConfig,
    #[serde(skip)]
    pub metadata: ConfigMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeyboardConfig {
    pub vim_navigation: bool,
    pub edit_step: usize,
    pub default_octave: u8,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            vim_navigation: true,
            edit_step: 1,
            default_octave: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MidiConfig {
    pub default_output: String,
    pub default_input: String,
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SampleBrowserConfig {
    pub chooser_command: Option<String>,
    pub start_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectBrowserConfig {
    pub start_dir: Option<PathBuf>,
    pub recent_file: Option<PathBuf>,
}

impl ProjectBrowserConfig {
    pub fn recent_file(&self) -> Option<PathBuf> {
        self.recent_file.clone().or_else(|| {
            env::var_os("HOME").map(PathBuf::from).map(|home| {
                home.join(".config")
                    .join("salieri")
                    .join("recent-projects.json")
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigOverrides {
    pub midi_log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    config: AppConfig,
}

impl LoadedConfig {
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn metadata(&self) -> &ConfigMetadata {
        &self.config.metadata
    }

    pub fn into_config(self) -> AppConfig {
        self.config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigMetadata {
    pub source: ConfigSource,
    pub keymap_profile: String,
    pub theme_name: String,
    pub display_mode: DisplayMode,
}

impl Default for ConfigMetadata {
    fn default() -> Self {
        Self {
            source: ConfigSource::Defaults,
            keymap_profile: "tracker".to_string(),
            theme_name: "default".to_string(),
            display_mode: DisplayMode::Adaptive,
        }
    }
}

impl ConfigMetadata {
    fn new(source: ConfigSource, config: &AppConfig) -> Self {
        Self {
            source,
            keymap_profile: config.keymap.profile.clone(),
            theme_name: config.theme.name.clone(),
            display_mode: config.ui.display_mode,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} | keymap={} | theme={} | display={}",
            self.source, self.keymap_profile, self.theme_name, self.display_mode
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    Defaults,
    File(PathBuf),
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Defaults => formatter.write_str("built-in defaults"),
            Self::File(path) => write!(formatter, "config {}", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationErrors {
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl fmt::Display for ConfigValidationErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "configuration has {} error(s):",
            self.diagnostics.len()
        )?;
        for diagnostic in &self.diagnostics {
            writeln!(formatter, "- {}: {}", diagnostic.field, diagnostic.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigValidationErrors {}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoadError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error(transparent)]
    Validation(#[from] ConfigValidationErrors),
}

pub fn load_config(
    path: Option<&Path>,
    overrides: ConfigOverrides,
) -> Result<LoadedConfig, ConfigLoadError> {
    let resolved_path = path.map(Path::to_path_buf).or_else(default_config_path);
    let (mut config, source) = match resolved_path {
        Some(path) if path.exists() => {
            let contents = fs::read_to_string(&path).map_err(|source| ConfigLoadError::Read {
                path: path.clone(),
                source,
            })?;
            let config = toml::from_str(&contents).map_err(|source| ConfigLoadError::Parse {
                path: path.clone(),
                source,
            })?;
            (config, ConfigSource::File(path))
        }
        Some(_) | None => (AppConfig::default(), ConfigSource::Defaults),
    };

    apply_overrides(&mut config, overrides);
    validate(&config)?;
    config.metadata = ConfigMetadata::new(source, &config);
    Ok(LoadedConfig { config })
}

fn apply_overrides(config: &mut AppConfig, overrides: ConfigOverrides) {
    if let Some(path) = overrides.midi_log_file {
        config.midi.log_file = Some(path);
    }
}

fn validate(config: &AppConfig) -> Result<(), ConfigValidationErrors> {
    let mut diagnostics = Vec::new();
    check_range(
        &mut diagnostics,
        "keyboard.edit_step",
        config.keyboard.edit_step,
        1,
        64,
    );
    check_range(
        &mut diagnostics,
        "keyboard.default_octave",
        usize::from(config.keyboard.default_octave),
        0,
        9,
    );
    check_non_empty(&mut diagnostics, "keymap.profile", &config.keymap.profile);
    check_non_empty(&mut diagnostics, "theme.name", &config.theme.name);
    check_range(
        &mut diagnostics,
        "audio.sample_rate",
        config.audio.sample_rate as usize,
        8_000,
        384_000,
    );
    check_range(
        &mut diagnostics,
        "audio.channels",
        usize::from(config.audio.channels),
        1,
        8,
    );
    check_range(
        &mut diagnostics,
        "workspace.recent_project_limit",
        config.workspace.recent_project_limit,
        1,
        100,
    );
    check_range(
        &mut diagnostics,
        "history.undo_limit",
        config.history.undo_limit,
        1,
        10_000,
    );
    check_range(
        &mut diagnostics,
        "ui.layout.left_width",
        usize::from(config.ui.layout.left_width),
        18,
        56,
    );
    check_range(
        &mut diagnostics,
        "ui.layout.inspector_width",
        usize::from(config.ui.layout.inspector_width),
        24,
        64,
    );
    check_range(
        &mut diagnostics,
        "ui.layout.track_desk_height",
        usize::from(config.ui.layout.track_desk_height),
        6,
        18,
    );
    if let Some(command) = &config.sample_browser.chooser_command {
        check_non_empty(&mut diagnostics, "sample_browser.chooser_command", command);
    }
    diagnostics.extend(
        keymap::validate_config(&config.keymap)
            .into_iter()
            .map(|diagnostic| ConfigDiagnostic {
                field: diagnostic.field,
                message: diagnostic.message,
            }),
    );

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ConfigValidationErrors { diagnostics })
    }
}

fn check_non_empty(diagnostics: &mut Vec<ConfigDiagnostic>, field: &str, value: &str) {
    if value.trim().is_empty() {
        diagnostics.push(ConfigDiagnostic {
            field: field.to_string(),
            message: "must not be empty".to_string(),
        });
    }
}

fn check_range(
    diagnostics: &mut Vec<ConfigDiagnostic>,
    field: &str,
    value: usize,
    minimum: usize,
    maximum: usize,
) {
    if !(minimum..=maximum).contains(&value) {
        diagnostics.push(ConfigDiagnostic {
            field: field.to_string(),
            message: format!("must be between {minimum} and {maximum}; got {value}"),
        });
    }
}

fn default_config_path() -> Option<PathBuf> {
    default_config_path_for(env::var_os("XDG_CONFIG_HOME"), env::var_os("HOME"))
}

fn default_config_path_for(
    xdg_config_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    xdg_config_home
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(PathBuf::from).map(|path| path.join(".config")))
        .map(|root| root.join("salieri").join("config.toml"))
}

#[cfg(test)]
mod config_tests;

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

    struct TestFile(PathBuf);

    impl TestFile {
        fn new(name: &str, contents: &str) -> Self {
            let path = env::temp_dir().join(format!(
                "salieri-config-{name}-{}-{}.toml",
                std::process::id(),
                NEXT_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&path, contents).expect("write config");
            Self(path)
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn missing_config_uses_defaults() {
        let path = env::temp_dir().join("salieri-definitely-missing-config.toml");
        let loaded = load_config(Some(&path), ConfigOverrides::default()).expect("defaults");

        assert_eq!(loaded.config(), &AppConfig::default());
        assert_eq!(loaded.metadata().source, ConfigSource::Defaults);
    }

    #[test]
    fn loads_partial_config_over_defaults_and_exposes_metadata() {
        let file = TestFile::new(
            "partial",
            r#"
[keyboard]
vim_navigation = false
edit_step = 4
default_octave = 5

[keymap]
profile = "studio"
bindings = { "ctrl+p" = "play pattern" }

[ui]
follow_playhead = false
display_mode = "compact"

[theme]
name = "high-contrast"

[midi]
default_output = "IAC Driver"
default_input = "IAC Driver"
log_file = "salieri-midi.log"

[sample_browser]
chooser_command = "yazi"
start_dir = "~/Samples"

[project_browser]
start_dir = "~/Music/Salieri"
recent_file = "recent-projects.json"

[history]
undo_limit = 250
"#,
        );

        let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
        let config = loaded.config();
        assert!(!config.keyboard.vim_navigation);
        assert_eq!(config.keyboard.edit_step, 4);
        assert!(!config.ui.follow_playhead);
        assert!(!config.ui.show_line_numbers_hex);
        assert_eq!(config.audio, AudioPreferences::default());
        assert_eq!(config.midi.default_output, "IAC Driver");
        assert_eq!(config.history.undo_limit, 250);
        assert_eq!(
            config.sample_browser.start_dir,
            Some(PathBuf::from("~/Samples"))
        );
        assert_eq!(loaded.metadata().source, ConfigSource::File(file.0.clone()));
        assert_eq!(loaded.metadata().keymap_profile, "studio");
        assert_eq!(loaded.metadata().theme_name, "high-contrast");
        assert_eq!(loaded.metadata().display_mode, DisplayMode::Compact);
    }

    #[test]
    fn loads_mode_specific_keymap_sections() {
        let file = TestFile::new(
            "keymap-layers",
            r#"
[keymap]
profile = "custom"

[keymap.normal]
q = "bpm 150"

[keymap.edit]
q = "bpm 90"

[keymap.ai]
a = "help"

[keymap.clip]
c = "help"
"#,
        );

        let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("valid keymap");

        assert_eq!(loaded.config().keymap.normal["q"], "bpm 150");
        assert_eq!(loaded.config().keymap.edit["q"], "bpm 90");
        assert_eq!(loaded.config().keymap.ai["a"], "help");
        assert_eq!(loaded.config().keymap.clip["c"], "help");
    }

    #[test]
    fn keymap_conflicts_are_reported_as_config_diagnostics() {
        let file = TestFile::new(
            "keymap-conflict",
            r#"
[keymap.normal]
"ctrl+p" = "play pattern"
"control+p" = "stop"
"#,
        );

        let error = load_config(Some(&file.0), ConfigOverrides::default()).expect_err("conflict");
        let ConfigLoadError::Validation(error) = error else {
            panic!("expected keymap validation error");
        };
        assert_eq!(error.diagnostics.len(), 1);
        assert!(error.diagnostics[0].field.starts_with("keymap.normal."));
        assert!(error.diagnostics[0].message.contains("conflicts with"));
    }

    #[test]
    fn cli_overrides_win_over_user_config() {
        let file = TestFile::new("override", "[midi]\nlog_file = 'user.log'\n");
        let loaded = load_config(
            Some(&file.0),
            ConfigOverrides {
                midi_log_file: Some(PathBuf::from("cli.log")),
            },
        )
        .expect("load config");

        assert_eq!(
            loaded.config().midi.log_file,
            Some(PathBuf::from("cli.log"))
        );
    }

    #[test]
    fn validation_reports_all_actionable_errors() {
        let file = TestFile::new(
            "invalid",
            r#"
[keyboard]
edit_step = 0
default_octave = 12
[keymap]
profile = " "
[theme]
name = ""
[audio]
sample_rate = 1000
channels = 0
[workspace]
recent_project_limit = 0
[history]
undo_limit = 0
"#,
        );

        let error = load_config(Some(&file.0), ConfigOverrides::default()).expect_err("invalid");
        let ConfigLoadError::Validation(error) = error else {
            panic!("expected validation error");
        };
        assert_eq!(error.diagnostics.len(), 8);
        let rendered = error.to_string();
        assert!(rendered.contains("keyboard.edit_step"));
        assert!(rendered.contains("audio.sample_rate"));
        assert!(rendered.contains("workspace.recent_project_limit"));
        assert!(rendered.contains("history.undo_limit"));
    }

    #[test]
    fn unknown_fields_are_rejected_with_parse_context() {
        let file = TestFile::new("unknown", "[keyboard]\ntyop = true\n");
        let error = load_config(Some(&file.0), ConfigOverrides::default()).expect_err("invalid");

        assert!(matches!(error, ConfigLoadError::Parse { .. }));
        assert!(error.to_string().contains("unknown field `tyop`"));
    }

    #[test]
    fn metadata_summary_is_ready_for_help_and_status_views() {
        let file = TestFile::new("metadata", "[theme]\nname = 'night'\n");
        let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");

        assert_eq!(
            loaded.metadata().summary(),
            format!(
                "config {} | keymap=tracker | theme=night | display=adaptive",
                file.0.display()
            )
        );
    }

    #[test]
    fn config_path_prefers_xdg_then_home() {
        assert_eq!(
            default_config_path_for(Some("/xdg".into()), Some("/home/me".into())),
            Some(PathBuf::from("/xdg/salieri/config.toml"))
        );
        assert_eq!(
            default_config_path_for(None, Some("/home/me".into())),
            Some(PathBuf::from("/home/me/.config/salieri/config.toml"))
        );
    }
}
