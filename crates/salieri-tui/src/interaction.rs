use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InteractionRegionId(&'static str);

impl InteractionRegionId {
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionRegion {
    pub id: InteractionRegionId,
    pub area: Rect,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionMap {
    regions: Vec<InteractionRegion>,
}

impl InteractionMap {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub fn register(&mut self, id: InteractionRegionId, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.regions.push(InteractionRegion { id, area });
    }

    #[must_use]
    pub fn regions(&self) -> &[InteractionRegion] {
        &self.regions
    }

    #[must_use]
    pub fn region(&self, id: InteractionRegionId) -> Option<&InteractionRegion> {
        self.regions.iter().rev().find(|region| region.id == id)
    }

    #[must_use]
    pub fn hit_test(&self, column: u16, row: u16) -> Option<&InteractionRegion> {
        self.regions
            .iter()
            .rev()
            .find(|region| contains(region.area, column, row))
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    (area.x..area.x.saturating_add(area.width)).contains(&column)
        && (area.y..area.y.saturating_add(area.height)).contains(&row)
}

pub mod region {
    use super::InteractionRegionId;

    pub const APP_HEADER: InteractionRegionId = InteractionRegionId::new("app.header");
    pub const APP_BODY: InteractionRegionId = InteractionRegionId::new("app.body");
    pub const APP_STATUS: InteractionRegionId = InteractionRegionId::new("app.status");

    pub const VIEW_PATTERN: InteractionRegionId = InteractionRegionId::new("view.pattern");
    pub const VIEW_SEQUENCE: InteractionRegionId = InteractionRegionId::new("view.sequence");
    pub const VIEW_CLIPS: InteractionRegionId = InteractionRegionId::new("view.clips");
    pub const VIEW_TRACKS: InteractionRegionId = InteractionRegionId::new("view.tracks");
    pub const VIEW_PATTERNS: InteractionRegionId = InteractionRegionId::new("view.patterns");
    pub const VIEW_SAMPLER: InteractionRegionId = InteractionRegionId::new("view.sampler");
    pub const VIEW_DSP_RACK: InteractionRegionId = InteractionRegionId::new("view.dsp-rack");
    pub const VIEW_SAMPLE_BROWSER: InteractionRegionId =
        InteractionRegionId::new("view.sample-browser");
    pub const VIEW_PROJECT_BROWSER: InteractionRegionId =
        InteractionRegionId::new("view.project-browser");
    pub const VIEW_AI_CHAT: InteractionRegionId = InteractionRegionId::new("view.ai-chat");

    pub const PANEL_TRACKS: InteractionRegionId = InteractionRegionId::new("panel.tracks");
    pub const PANEL_SEQUENCE: InteractionRegionId = InteractionRegionId::new("panel.sequence");
    pub const PANEL_PATTERN: InteractionRegionId = InteractionRegionId::new("panel.pattern");
    pub const PANEL_TRACK_DESK: InteractionRegionId = InteractionRegionId::new("panel.track-desk");
    pub const PANEL_INSPECTOR: InteractionRegionId = InteractionRegionId::new("panel.inspector");
    pub const PANEL_ANALYZER: InteractionRegionId = InteractionRegionId::new("panel.analyzer");
    pub const PANEL_UTIL: InteractionRegionId = InteractionRegionId::new("panel.util");
    pub const PANEL_EFFECTS: InteractionRegionId = InteractionRegionId::new("panel.effects");
    pub const PANEL_MIXER: InteractionRegionId = InteractionRegionId::new("panel.mixer");
    pub const PANEL_VU: InteractionRegionId = InteractionRegionId::new("panel.vu");
    pub const PANEL_DEVICE_CHAIN: InteractionRegionId =
        InteractionRegionId::new("panel.device-chain");

    pub const OVERLAY_HELP: InteractionRegionId = InteractionRegionId::new("overlay.help");
    pub const OVERLAY_MIDI_SETTINGS: InteractionRegionId =
        InteractionRegionId::new("overlay.midi-settings");
    pub const OVERLAY_COMMAND_PALETTE: InteractionRegionId =
        InteractionRegionId::new("overlay.command-palette");
    pub const OVERLAY_QUIT_CONFIRMATION: InteractionRegionId =
        InteractionRegionId::new("overlay.quit-confirmation");
    pub const OVERLAY_DELETE_CONFIRMATION: InteractionRegionId =
        InteractionRegionId::new("overlay.delete-confirmation");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_empty_regions() {
        let mut map = InteractionMap::new();
        map.register(region::APP_BODY, Rect::new(0, 0, 0, 10));

        assert!(map.regions().is_empty());
    }

    #[test]
    fn hit_test_prefers_the_most_recent_matching_region() {
        let mut map = InteractionMap::new();
        map.register(region::APP_BODY, Rect::new(0, 0, 80, 24));
        map.register(region::PANEL_PATTERN, Rect::new(20, 3, 60, 20));

        assert_eq!(
            map.hit_test(30, 10).map(|region| region.id),
            Some(region::PANEL_PATTERN)
        );
        assert_eq!(
            map.hit_test(5, 10).map(|region| region.id),
            Some(region::APP_BODY)
        );
        assert!(map.hit_test(80, 24).is_none());
    }
}
