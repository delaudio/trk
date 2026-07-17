use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub keyboard: KeyboardConfig,
    pub ui: UiConfig,
    pub midi: MidiConfig,
    pub sample_browser: SampleBrowserConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub show_line_numbers_hex: bool,
    pub follow_playhead: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_line_numbers_hex: false,
            follow_playhead: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct MidiConfig {
    pub default_output: String,
    pub default_input: String,
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct SampleBrowserConfig {
    pub chooser_command: Option<String>,
    pub start_dir: Option<PathBuf>,
}

pub fn load_config(path: Option<&Path>) -> Result<AppConfig> {
    let Some(path) = path.map(Path::to_path_buf).or_else(default_config_path) else {
        return Ok(AppConfig::default());
    };

    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("failed to parse config {}", path.display()))
}

fn default_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("salieri").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_uses_defaults() {
        let path = std::env::temp_dir().join(format!(
            "salieri-missing-config-{}.toml",
            std::process::id()
        ));

        let config = load_config(Some(&path)).expect("default config");

        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn loads_partial_config_over_defaults() {
        let path = std::env::temp_dir().join(format!(
            "salieri-partial-config-{}.toml",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"
[keyboard]
vim_navigation = false
edit_step = 4
default_octave = 5

[ui]
follow_playhead = false

[midi]
default_output = "IAC Driver"
default_input = "IAC Driver"
log_file = "salieri-midi.log"

[sample_browser]
chooser_command = 'YAZI_CONFIG_HOME="$HOME/.config/yazi-readonly" yazi --chooser-file "$SALIERI_CHOOSER_FILE" "$SALIERI_SAMPLE_START_DIR"'
start_dir = "~/Samples"
"#,
        )
        .expect("write config");

        let config = load_config(Some(&path)).expect("load config");
        let _ = fs::remove_file(&path);

        assert!(!config.keyboard.vim_navigation);
        assert_eq!(config.keyboard.edit_step, 4);
        assert_eq!(config.keyboard.default_octave, 5);
        assert!(!config.ui.follow_playhead);
        assert!(!config.ui.show_line_numbers_hex);
        assert_eq!(config.midi.default_output, "IAC Driver");
        assert_eq!(config.midi.default_input, "IAC Driver");
        assert_eq!(
            config.midi.log_file,
            Some(PathBuf::from("salieri-midi.log"))
        );
        assert_eq!(
            config.sample_browser.chooser_command,
            Some(
                r#"YAZI_CONFIG_HOME="$HOME/.config/yazi-readonly" yazi --chooser-file "$SALIERI_CHOOSER_FILE" "$SALIERI_SAMPLE_START_DIR""#
                    .to_string()
            )
        );
        assert_eq!(
            config.sample_browser.start_dir,
            Some(PathBuf::from("~/Samples"))
        );
    }
}
