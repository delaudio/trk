use std::ops::Range;

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
    pub payload: InteractionPayload,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InteractionPayload {
    #[default]
    None,
    PatternCell {
        row: usize,
        track: usize,
    },
    SampleBrowserEntry {
        index: usize,
    },
    ProjectBrowserEntry {
        index: usize,
    },
    CompositeTrackRow {
        track: usize,
    },
    CompositeSequenceRow {
        position: usize,
    },
    PatternManagerRow {
        index: usize,
    },
    SequenceEditorRow {
        position: usize,
    },
    CommandPaletteEntry {
        index: usize,
    },
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
        self.register_with_payload(id, area, InteractionPayload::None);
    }

    pub fn register_with_payload(
        &mut self,
        id: InteractionRegionId,
        area: Rect,
        payload: InteractionPayload,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.regions.push(InteractionRegion { id, area, payload });
    }

    pub(crate) fn register_pattern_cells(
        &mut self,
        content_area: Rect,
        header_height: u16,
        row_gutter_width: u16,
        cell_width: u16,
        visible_rows: Range<usize>,
        visible_tracks: Range<usize>,
    ) {
        let content_right = content_area.x.saturating_add(content_area.width);
        let content_bottom = content_area.y.saturating_add(content_area.height);
        let first_cell_x = content_area.x.saturating_add(row_gutter_width);
        let first_cell_y = content_area.y.saturating_add(header_height);

        for (visible_row, row) in visible_rows.enumerate() {
            let y = first_cell_y.saturating_add(visible_row as u16);
            if y >= content_bottom {
                break;
            }
            for (visible_track, track) in visible_tracks.clone().enumerate() {
                let x =
                    first_cell_x.saturating_add((visible_track as u16).saturating_mul(cell_width));
                if x >= content_right {
                    break;
                }
                let width = cell_width.min(content_right.saturating_sub(x));
                self.register_with_payload(
                    region::PATTERN_CELL,
                    Rect::new(x, y, width, 1),
                    InteractionPayload::PatternCell { row, track },
                );
            }
        }
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
    pub const PATTERN_CELL: InteractionRegionId = InteractionRegionId::new("pattern.cell");
    pub const SAMPLE_BROWSER_ENTRY: InteractionRegionId =
        InteractionRegionId::new("sample-browser.entry");
    pub const PROJECT_BROWSER_ENTRY: InteractionRegionId =
        InteractionRegionId::new("project-browser.entry");
    pub const COMPOSITE_TRACK_ROW: InteractionRegionId =
        InteractionRegionId::new("composite-tracks.row");
    pub const COMPOSITE_SEQUENCE_ROW: InteractionRegionId =
        InteractionRegionId::new("composite-sequence.row");
    pub const PATTERN_MANAGER_ROW: InteractionRegionId =
        InteractionRegionId::new("pattern-manager.row");
    pub const SEQUENCE_EDITOR_ROW: InteractionRegionId =
        InteractionRegionId::new("sequence-editor.row");
    pub const COMMAND_PALETTE_RESULTS: InteractionRegionId =
        InteractionRegionId::new("command-palette.results");
    pub const COMMAND_PALETTE_ENTRY: InteractionRegionId =
        InteractionRegionId::new("command-palette.entry");

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

    #[test]
    fn pattern_cells_carry_absolute_row_and_track_payloads() {
        let mut map = InteractionMap::new();
        map.register_pattern_cells(Rect::new(1, 1, 17, 5), 1, 5, 6, 4..6, 2..4);

        assert_eq!(
            map.hit_test(6, 2).map(|region| region.payload),
            Some(InteractionPayload::PatternCell { row: 4, track: 2 })
        );
        assert_eq!(
            map.hit_test(12, 3).map(|region| region.payload),
            Some(InteractionPayload::PatternCell { row: 5, track: 3 })
        );
        assert!(map.hit_test(5, 2).is_none());
    }
}
