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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiSettingsAction {
    Connect,
    Disconnect,
    Panic,
    Refresh,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationAction {
    Save,
    DontSave,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportAction {
    Play,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerEnvelopeField {
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerAction {
    SelectEnvelope(SamplerEnvelopeField),
    DecrementEnvelope,
    IncrementEnvelope,
    ZoomOut,
    ZoomIn,
    PanLeft,
    PanRight,
    Browse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTarget {
    PatternRows,
    Tracks,
    Sequence,
    Clips,
    Patterns,
    SampleBrowser,
    ProjectBrowser,
    SamplerWaveform,
    DspDevices { target: DspRackChain },
    DspParameters,
    DspPalette,
    CommandPalette,
    HelpContent,
    MidiPorts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DspRackChain {
    Track,
    Master,
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
    HelpTab {
        index: usize,
    },
    MidiPortRow {
        index: usize,
    },
    MidiSettingsAction {
        action: MidiSettingsAction,
    },
    ConfirmationAction {
        action: ConfirmationAction,
    },
    TransportAction {
        action: TransportAction,
    },
    SamplerAction {
        action: SamplerAction,
    },
    DspRackTarget {
        target: DspRackChain,
    },
    DspDeviceRow {
        target: DspRackChain,
        index: usize,
    },
    DspParameterRow {
        index: usize,
    },
    DspPaletteEntry {
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
        let rendered_width = usize::from(row_gutter_width)
            .saturating_add(visible_tracks.len().saturating_mul(usize::from(cell_width)));
        self.register(
            region::PATTERN_GRID,
            Rect::new(
                content_area.x,
                content_area.y.saturating_add(header_height),
                u16::try_from(rendered_width)
                    .unwrap_or(u16::MAX)
                    .min(content_area.width),
                u16::try_from(visible_rows.len()).unwrap_or(u16::MAX),
            ),
        );
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

    #[must_use]
    pub fn scroll_target_at(&self, column: u16, row: u16) -> Option<ScrollTarget> {
        let hit = self.hit_test(column, row)?;
        match (hit.id, hit.payload) {
            (region::PATTERN_CELL | region::PATTERN_GRID, _) => Some(ScrollTarget::PatternRows),
            (region::COMPOSITE_TRACK_ROW, _) => Some(ScrollTarget::Tracks),
            (region::COMPOSITE_SEQUENCE_ROW | region::SEQUENCE_EDITOR_ROW, _) => {
                Some(ScrollTarget::Sequence)
            }
            (region::CLIP_GRID, _) => Some(ScrollTarget::Clips),
            (region::PATTERN_MANAGER_ROW, _) => Some(ScrollTarget::Patterns),
            (region::SAMPLE_BROWSER_ENTRY, _) => Some(ScrollTarget::SampleBrowser),
            (region::PROJECT_BROWSER_ENTRY, _) => Some(ScrollTarget::ProjectBrowser),
            (region::SAMPLER_WAVEFORM, _) => Some(ScrollTarget::SamplerWaveform),
            (region::DSP_DEVICE_ROW, InteractionPayload::DspDeviceRow { target, .. }) => {
                Some(ScrollTarget::DspDevices { target })
            }
            (region::DSP_PARAMETER_ROW, _) => Some(ScrollTarget::DspParameters),
            (region::DSP_PALETTE_ENTRY, _) => Some(ScrollTarget::DspPalette),
            (region::COMMAND_PALETTE_RESULTS | region::COMMAND_PALETTE_ENTRY, _) => {
                Some(ScrollTarget::CommandPalette)
            }
            (region::HELP_CONTENT, _) => Some(ScrollTarget::HelpContent),
            (region::MIDI_SETTINGS_PORT, _) => Some(ScrollTarget::MidiPorts),
            _ => None,
        }
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    (area.x..area.x.saturating_add(area.width)).contains(&column)
        && (area.y..area.y.saturating_add(area.height)).contains(&row)
}

pub mod region {
    use super::InteractionRegionId;

    pub const APP_HEADER: InteractionRegionId = InteractionRegionId::new("app.header");
    pub const TRANSPORT_ACTION: InteractionRegionId = InteractionRegionId::new("transport.action");
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
    pub const PATTERN_GRID: InteractionRegionId = InteractionRegionId::new("pattern.grid");
    pub const CLIP_GRID: InteractionRegionId = InteractionRegionId::new("clips.grid");
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
    pub const HELP_TAB: InteractionRegionId = InteractionRegionId::new("help.tab");
    pub const HELP_CONTENT: InteractionRegionId = InteractionRegionId::new("help.content");
    pub const HELP_CLOSE: InteractionRegionId = InteractionRegionId::new("help.close");
    pub const MIDI_SETTINGS_PORT: InteractionRegionId =
        InteractionRegionId::new("midi-settings.port");
    pub const MIDI_SETTINGS_ACTION: InteractionRegionId =
        InteractionRegionId::new("midi-settings.action");
    pub const CONFIRMATION_ACTION: InteractionRegionId =
        InteractionRegionId::new("confirmation.action");
    pub const DSP_RACK_TARGET: InteractionRegionId = InteractionRegionId::new("dsp-rack.target");
    pub const DSP_CHAIN: InteractionRegionId = InteractionRegionId::new("dsp-rack.chain");
    pub const DSP_DEVICE_ROW: InteractionRegionId = InteractionRegionId::new("dsp-rack.device-row");
    pub const DSP_PARAMETER_ROW: InteractionRegionId =
        InteractionRegionId::new("dsp-rack.parameter-row");
    pub const DSP_PALETTE_ENTRY: InteractionRegionId =
        InteractionRegionId::new("dsp-rack.palette-entry");
    pub const SAMPLER_ACTION: InteractionRegionId = InteractionRegionId::new("sampler.action");
    pub const SAMPLER_WAVEFORM: InteractionRegionId = InteractionRegionId::new("sampler.waveform");

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
        assert_eq!(map.scroll_target_at(6, 2), Some(ScrollTarget::PatternRows));
        assert_eq!(
            map.hit_test(5, 2).map(|region| region.id),
            Some(region::PATTERN_GRID)
        );
        assert_eq!(map.scroll_target_at(5, 2), Some(ScrollTarget::PatternRows));
    }

    #[test]
    fn scroll_target_uses_the_topmost_semantic_region() {
        let mut map = InteractionMap::new();
        map.register(region::PATTERN_GRID, Rect::new(0, 0, 80, 20));
        map.register(region::COMPOSITE_TRACK_ROW, Rect::new(0, 0, 20, 20));
        map.register(region::OVERLAY_HELP, Rect::new(10, 4, 60, 12));
        map.register(region::HELP_CONTENT, Rect::new(12, 6, 56, 8));

        assert_eq!(map.scroll_target_at(5, 5), Some(ScrollTarget::Tracks));
        assert_eq!(map.scroll_target_at(11, 5), None);
        assert_eq!(map.scroll_target_at(12, 6), Some(ScrollTarget::HelpContent));
    }

    #[test]
    fn non_scrollable_chrome_has_no_scroll_target() {
        let mut map = InteractionMap::new();
        map.register(region::APP_HEADER, Rect::new(0, 0, 80, 3));
        map.register(region::PANEL_INSPECTOR, Rect::new(60, 3, 20, 20));

        assert_eq!(map.scroll_target_at(5, 1), None);
        assert_eq!(map.scroll_target_at(65, 8), None);
    }

    #[test]
    fn broad_view_and_panel_regions_are_not_scroll_targets() {
        let ids = [
            region::VIEW_PATTERN,
            region::VIEW_TRACKS,
            region::VIEW_SEQUENCE,
            region::VIEW_CLIPS,
            region::VIEW_PATTERNS,
            region::VIEW_SAMPLE_BROWSER,
            region::VIEW_PROJECT_BROWSER,
            region::PANEL_PATTERN,
            region::PANEL_TRACKS,
            region::PANEL_SEQUENCE,
            region::DSP_CHAIN,
        ];

        for id in ids {
            let mut map = InteractionMap::new();
            map.register(id, Rect::new(0, 0, 80, 24));
            assert_eq!(
                map.scroll_target_at(20, 10),
                None,
                "{} should not scroll from its chrome or padding",
                id.as_str()
            );
        }
    }
}
