use crate::keymap::KeymapMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Edit,
    Command,
    CommandPalette,
    Help,
    Dialog,
    MidiSettings,
    Sequence,
    Tracks,
    Patterns,
    Sampler,
    SampleBrowser,
    ProjectBrowser,
}

impl AppMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Edit => "EDIT",
            Self::Command => "COMMAND",
            Self::CommandPalette => "PALETTE",
            Self::Help => "HELP",
            Self::Dialog => "DIALOG",
            Self::MidiSettings => "MIDI",
            Self::Sequence => "SEQUENCE",
            Self::Tracks => "TRACKS",
            Self::Patterns => "PATTERNS",
            Self::Sampler => "SAMPLER",
            Self::SampleBrowser => "SAMPLES",
            Self::ProjectBrowser => "PROJECTS",
        }
    }

    pub const fn keymap_mode(self) -> KeymapMode {
        match self {
            Self::Normal => KeymapMode::Normal,
            Self::Edit => KeymapMode::Edit,
            Self::Command => KeymapMode::Command,
            Self::CommandPalette => KeymapMode::CommandPalette,
            Self::Help => KeymapMode::Help,
            Self::Dialog => KeymapMode::Dialog,
            Self::MidiSettings => KeymapMode::MidiSettings,
            Self::Sequence => KeymapMode::Sequence,
            Self::Tracks => KeymapMode::Tracks,
            Self::Patterns => KeymapMode::Patterns,
            Self::Sampler => KeymapMode::Sampler,
            Self::SampleBrowser => KeymapMode::SampleBrowser,
            Self::ProjectBrowser => KeymapMode::ProjectBrowser,
        }
    }
}
