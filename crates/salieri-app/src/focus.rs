use crate::{app_mode::AppMode, command::FocusTarget};
use salieri_tui::TuiView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    Tracker,
    Sequence,
    Tracks,
    Patterns,
    Sampler,
    SampleBrowser,
    ProjectBrowser,
    MidiSettings,
}

impl FocusPanel {
    #[must_use]
    pub const fn app_mode(self) -> AppMode {
        match self {
            Self::Tracker => AppMode::Normal,
            Self::Sequence => AppMode::Sequence,
            Self::Tracks => AppMode::Tracks,
            Self::Patterns => AppMode::Patterns,
            Self::Sampler => AppMode::Sampler,
            Self::SampleBrowser => AppMode::SampleBrowser,
            Self::ProjectBrowser => AppMode::ProjectBrowser,
            Self::MidiSettings => AppMode::MidiSettings,
        }
    }

    #[must_use]
    pub const fn tui_view(self) -> TuiView {
        match self {
            Self::Tracker | Self::MidiSettings => TuiView::Pattern,
            Self::Sequence => TuiView::Sequence,
            Self::Tracks => TuiView::Tracks,
            Self::Patterns => TuiView::Patterns,
            Self::Sampler => TuiView::Sampler,
            Self::SampleBrowser => TuiView::SampleBrowser,
            Self::ProjectBrowser => TuiView::ProjectBrowser,
        }
    }

    #[must_use]
    pub const fn from_target(target: FocusTarget) -> Self {
        match target {
            FocusTarget::Tracker => Self::Tracker,
            FocusTarget::Patterns => Self::Patterns,
            FocusTarget::Sequence => Self::Sequence,
            FocusTarget::Tracks => Self::Tracks,
            FocusTarget::Sampler => Self::Sampler,
            FocusTarget::SampleBrowser => Self::SampleBrowser,
            FocusTarget::ProjectBrowser => Self::ProjectBrowser,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusCapture {
    Command,
    CommandPalette,
    Help,
    Dialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusManager {
    focused: FocusPanel,
    previous: Option<FocusPanel>,
    capture: Option<FocusCapture>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focused: FocusPanel::Tracker,
            previous: None,
            capture: None,
        }
    }
}

impl FocusManager {
    #[must_use]
    pub const fn focused(self) -> FocusPanel {
        self.focused
    }

    #[cfg(test)]
    #[must_use]
    pub const fn previous(self) -> Option<FocusPanel> {
        self.previous
    }

    #[cfg(test)]
    #[must_use]
    pub const fn capture(self) -> Option<FocusCapture> {
        self.capture
    }

    pub fn focus(&mut self, panel: FocusPanel) {
        if self.focused != panel {
            self.previous = Some(self.focused);
            self.focused = panel;
        }
        self.capture = None;
    }

    pub fn capture_input(&mut self, capture: FocusCapture) {
        self.capture = Some(capture);
    }

    pub fn release_capture(&mut self) -> FocusPanel {
        self.capture = None;
        self.focused
    }

    pub fn restore_previous(&mut self) -> FocusPanel {
        if let Some(previous) = self.previous.take() {
            let focused = self.focused;
            self.focused = previous;
            self.previous = Some(focused);
        }
        self.capture = None;
        self.focused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_tracks_previous_panel_and_capture() {
        let mut focus = FocusManager::default();

        focus.focus(FocusPanel::Sampler);
        focus.capture_input(FocusCapture::Help);

        assert_eq!(focus.focused(), FocusPanel::Sampler);
        assert_eq!(focus.previous(), Some(FocusPanel::Tracker));
        assert_eq!(focus.capture(), Some(FocusCapture::Help));

        assert_eq!(focus.release_capture(), FocusPanel::Sampler);
        assert_eq!(focus.capture(), None);
    }

    #[test]
    fn previous_focus_restores_deterministically() {
        let mut focus = FocusManager::default();
        focus.focus(FocusPanel::Sequence);
        focus.focus(FocusPanel::Tracks);

        assert_eq!(focus.restore_previous(), FocusPanel::Sequence);
        assert_eq!(focus.previous(), Some(FocusPanel::Tracks));
    }
}
