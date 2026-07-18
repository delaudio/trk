use std::{collections::BTreeMap, fmt};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeymapConfig {
    pub profile: String,
    pub bindings: BTreeMap<String, String>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            profile: "tracker".to_string(),
            bindings: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub show_line_numbers_hex: bool,
    pub follow_playhead: bool,
    pub display_mode: DisplayMode,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_line_numbers_hex: false,
            follow_playhead: true,
            display_mode: DisplayMode::Adaptive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    Compact,
    #[default]
    Adaptive,
    Spacious,
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Compact => "compact",
            Self::Adaptive => "adaptive",
            Self::Spacious => "spacious",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    pub name: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AudioPreferences {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for AudioPreferences {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkspaceConfig {
    pub recent_project_limit: usize,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            recent_project_limit: 12,
        }
    }
}
