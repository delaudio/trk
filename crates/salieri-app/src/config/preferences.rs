use std::fmt;

use salieri_tui::{ManagedPanelId, TrackerLayoutPreset, TrackerLayoutState};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub show_line_numbers_hex: bool,
    pub row_number_format: RowNumberFormat,
    pub row_number_base: RowNumberBase,
    pub pattern_divider_interval: usize,
    pub pattern_highlight_interval: usize,
    pub show_pattern_top_info: bool,
    pub follow_playhead: bool,
    pub display_mode: DisplayMode,
    pub layout: LayoutConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_line_numbers_hex: false,
            row_number_format: RowNumberFormat::Decimal,
            row_number_base: RowNumberBase::Zero,
            pattern_divider_interval: 4,
            pattern_highlight_interval: 16,
            show_pattern_top_info: true,
            follow_playhead: true,
            display_mode: DisplayMode::Adaptive,
            layout: LayoutConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowNumberFormat {
    #[default]
    Decimal,
    Hex,
}

impl RowNumberFormat {
    pub const fn uses_hex(self, legacy_hex: bool) -> bool {
        match self {
            Self::Decimal => legacy_hex,
            Self::Hex => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowNumberBase {
    #[default]
    Zero,
    One,
}

impl RowNumberBase {
    pub const fn offset(self) -> usize {
        match self {
            Self::Zero => 0,
            Self::One => 1,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    pub default: LayoutPreset,
    pub show_tracks: bool,
    pub show_sequence: bool,
    pub show_inspector: bool,
    pub show_track_desk: bool,
    pub left_width: u16,
    pub inspector_width: u16,
    pub track_desk_height: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        let preset = TrackerLayoutState::from_preset(TrackerLayoutPreset::Balanced);
        Self {
            default: LayoutPreset::Balanced,
            show_tracks: preset.tracks_visible,
            show_sequence: preset.sequence_visible,
            show_inspector: preset.inspector_visible,
            show_track_desk: preset.track_desk_visible,
            left_width: preset.left_width,
            inspector_width: preset.inspector_width,
            track_desk_height: preset.track_desk_height,
        }
    }
}

impl LayoutConfig {
    pub fn tracker_layout(self) -> TrackerLayoutState {
        let mut layout = TrackerLayoutState::from_preset(self.default.into());
        layout.tracks_visible = self.show_tracks;
        layout.sequence_visible = self.show_sequence;
        layout.inspector_visible = self.show_inspector;
        layout.track_desk_visible = self.show_track_desk;
        layout.left_width = self.left_width;
        layout.inspector_width = self.inspector_width;
        layout.track_desk_height = self.track_desk_height;
        layout.active_panel = ManagedPanelId::Pattern;
        layout
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutPreset {
    Compact,
    #[default]
    Balanced,
    Studio,
}

impl From<LayoutPreset> for TrackerLayoutPreset {
    fn from(value: LayoutPreset) -> Self {
        match value {
            LayoutPreset::Compact => Self::Compact,
            LayoutPreset::Balanced => Self::Balanced,
            LayoutPreset::Studio => Self::Studio,
        }
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
    pub playback_headroom_db: u8,
    pub limiter_mode: LimiterMode,
    pub resampling_quality: ResamplingQuality,
    pub send_mode: SendMode,
}

impl Default for AudioPreferences {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            playback_headroom_db: 0,
            limiter_mode: LimiterMode::Off,
            resampling_quality: ResamplingQuality::Balanced,
            send_mode: SendMode::Disabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimiterMode {
    #[default]
    Off,
    Soft,
    Brickwall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResamplingQuality {
    Draft,
    #[default]
    Balanced,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendMode {
    #[default]
    Disabled,
    PreFader,
    PostFader,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorkspaceConfig {
    pub project_library: Option<std::path::PathBuf>,
    pub sample_library: Option<std::path::PathBuf>,
    pub recent_project_limit: usize,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            project_library: None,
            sample_library: None,
            recent_project_limit: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HistoryConfig {
    pub undo_limit: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self { undo_limit: 100 }
    }
}
