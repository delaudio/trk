use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedPanelId {
    Tracks,
    Sequence,
    Pattern,
    TrackDesk,
    Inspector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedLayoutDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedSize {
    Cells(u16),
    Percent(u16),
    RemainingAfterSecond(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedLayoutNode {
    Panel {
        id: ManagedPanelId,
        visible: bool,
        min_width: u16,
        min_height: u16,
    },
    Split {
        direction: ManagedLayoutDirection,
        first: Box<ManagedLayoutNode>,
        second: Box<ManagedLayoutNode>,
        first_size: ManagedSize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPanel {
    pub id: ManagedPanelId,
    pub area: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutDiagnostic {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedManagedLayout {
    pub panels: Vec<ResolvedPanel>,
    pub diagnostics: Vec<LayoutDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrackerLayoutPreset {
    Compact,
    #[default]
    Balanced,
    Studio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackerLayoutState {
    pub preset: TrackerLayoutPreset,
    pub tracks_visible: bool,
    pub sequence_visible: bool,
    pub inspector_visible: bool,
    pub track_desk_visible: bool,
    pub left_width: u16,
    pub inspector_width: u16,
    pub track_desk_height: u16,
    pub active_panel: ManagedPanelId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedTrackerLayout {
    pub tracks: Option<Rect>,
    pub sequence: Option<Rect>,
    pub pattern: Rect,
    pub track_desk: Option<Rect>,
    pub inspector: Option<Rect>,
    pub diagnostics: usize,
}

impl ManagedLayoutNode {
    pub fn panel(id: ManagedPanelId, min_width: u16, min_height: u16) -> Self {
        Self::Panel {
            id,
            visible: true,
            min_width,
            min_height,
        }
    }

    pub fn hidden_panel(id: ManagedPanelId, min_width: u16, min_height: u16) -> Self {
        Self::Panel {
            id,
            visible: false,
            min_width,
            min_height,
        }
    }

    pub fn split(
        direction: ManagedLayoutDirection,
        first: Self,
        second: Self,
        first_size: ManagedSize,
    ) -> Self {
        Self::Split {
            direction,
            first: Box::new(first),
            second: Box::new(second),
            first_size,
        }
    }
}

impl Default for TrackerLayoutState {
    fn default() -> Self {
        Self::from_preset(TrackerLayoutPreset::Balanced)
    }
}

impl TrackerLayoutState {
    pub fn from_preset(preset: TrackerLayoutPreset) -> Self {
        match preset {
            TrackerLayoutPreset::Compact => Self {
                preset,
                tracks_visible: false,
                sequence_visible: false,
                inspector_visible: false,
                track_desk_visible: false,
                left_width: 24,
                inspector_width: 34,
                track_desk_height: 8,
                active_panel: ManagedPanelId::Pattern,
            },
            TrackerLayoutPreset::Balanced => Self {
                preset,
                tracks_visible: true,
                sequence_visible: true,
                inspector_visible: false,
                track_desk_visible: true,
                left_width: 28,
                inspector_width: 36,
                track_desk_height: 10,
                active_panel: ManagedPanelId::Pattern,
            },
            TrackerLayoutPreset::Studio => Self {
                preset,
                tracks_visible: true,
                sequence_visible: true,
                inspector_visible: true,
                track_desk_visible: true,
                left_width: 30,
                inspector_width: 42,
                track_desk_height: 10,
                active_panel: ManagedPanelId::Pattern,
            },
        }
    }

    pub fn resize_panel(&mut self, panel: ManagedPanelId, delta: i16) {
        match panel {
            ManagedPanelId::Tracks | ManagedPanelId::Sequence => {
                self.left_width = resize_cells(self.left_width, delta, 18, 56);
            }
            ManagedPanelId::Inspector => {
                self.inspector_width = resize_cells(self.inspector_width, delta, 24, 64);
            }
            ManagedPanelId::TrackDesk => {
                self.track_desk_height = resize_cells(self.track_desk_height, delta, 6, 18);
            }
            ManagedPanelId::Pattern => {}
        }
    }

    pub fn set_panel_visible(&mut self, panel: ManagedPanelId, visible: bool) {
        match panel {
            ManagedPanelId::Tracks => self.tracks_visible = visible,
            ManagedPanelId::Sequence => self.sequence_visible = visible,
            ManagedPanelId::Inspector => self.inspector_visible = visible,
            ManagedPanelId::TrackDesk => self.track_desk_visible = visible,
            ManagedPanelId::Pattern => {}
        }
    }

    pub fn toggle_panel(&mut self, panel: ManagedPanelId) {
        let visible = !self.panel_visible(panel);
        self.set_panel_visible(panel, visible);
    }

    pub fn panel_visible(self, panel: ManagedPanelId) -> bool {
        match panel {
            ManagedPanelId::Tracks => self.tracks_visible,
            ManagedPanelId::Sequence => self.sequence_visible,
            ManagedPanelId::Pattern => true,
            ManagedPanelId::TrackDesk => self.track_desk_visible,
            ManagedPanelId::Inspector => self.inspector_visible,
        }
    }
}

pub fn resolve_managed_layout(root: &ManagedLayoutNode, area: Rect) -> ResolvedManagedLayout {
    let mut resolved = ResolvedManagedLayout {
        panels: Vec::new(),
        diagnostics: Vec::new(),
    };
    resolve_node(root, area, &mut resolved);
    resolved
}

pub fn resolve_tracker_layout(area: Rect, state: TrackerLayoutState) -> ResolvedTrackerLayout {
    let root = tracker_layout_tree(state);
    let resolved = resolve_managed_layout(&root, area);
    let panel = |id| {
        resolved
            .panels
            .iter()
            .find(|panel| panel.id == id)
            .map(|panel| panel.area)
    };
    ResolvedTrackerLayout {
        tracks: panel(ManagedPanelId::Tracks),
        sequence: panel(ManagedPanelId::Sequence),
        pattern: panel(ManagedPanelId::Pattern).unwrap_or(area),
        track_desk: panel(ManagedPanelId::TrackDesk),
        inspector: panel(ManagedPanelId::Inspector),
        diagnostics: resolved.diagnostics.len(),
    }
}

fn tracker_layout_tree(state: TrackerLayoutState) -> ManagedLayoutNode {
    use ManagedLayoutDirection::{Horizontal, Vertical};
    use ManagedPanelId::{Inspector, Pattern, Sequence, TrackDesk, Tracks};

    let left = ManagedLayoutNode::split(
        Vertical,
        panel(Tracks, state.tracks_visible, 18, 5),
        panel(Sequence, state.sequence_visible, 18, 5),
        ManagedSize::Percent(60),
    );
    let pattern_stack = ManagedLayoutNode::split(
        Vertical,
        ManagedLayoutNode::panel(Pattern, 32, 8),
        panel(TrackDesk, state.track_desk_visible, 24, 6),
        ManagedSize::RemainingAfterSecond(state.track_desk_height),
    );
    let center = ManagedLayoutNode::split(
        Horizontal,
        pattern_stack,
        panel(Inspector, state.inspector_visible, 24, 8),
        ManagedSize::RemainingAfterSecond(state.inspector_width),
    );
    ManagedLayoutNode::split(
        Horizontal,
        left,
        center,
        ManagedSize::Cells(state.left_width),
    )
}

fn panel(id: ManagedPanelId, visible: bool, min_width: u16, min_height: u16) -> ManagedLayoutNode {
    if visible {
        ManagedLayoutNode::panel(id, min_width, min_height)
    } else {
        ManagedLayoutNode::hidden_panel(id, min_width, min_height)
    }
}

fn resolve_node(node: &ManagedLayoutNode, area: Rect, resolved: &mut ResolvedManagedLayout) {
    match node {
        ManagedLayoutNode::Panel {
            id,
            visible,
            min_width,
            min_height,
        } => {
            if !visible {
                return;
            }
            if area.width < *min_width || area.height < *min_height {
                resolved.diagnostics.push(LayoutDiagnostic {
                    message: format!("panel {id:?} below minimum constraints"),
                });
            }
            resolved.panels.push(ResolvedPanel { id: *id, area });
        }
        ManagedLayoutNode::Split {
            direction,
            first,
            second,
            first_size,
        } => {
            let first_visible = contains_visible_panel(first);
            let second_visible = contains_visible_panel(second);
            match (first_visible, second_visible) {
                (false, false) => {}
                (true, false) => resolve_node(first, area, resolved),
                (false, true) => resolve_node(second, area, resolved),
                (true, true) => {
                    let (first_area, second_area) = split_area(*direction, *first_size, area);
                    resolve_node(first, first_area, resolved);
                    resolve_node(second, second_area, resolved);
                }
            }
        }
    }
}

fn contains_visible_panel(node: &ManagedLayoutNode) -> bool {
    match node {
        ManagedLayoutNode::Panel { visible, .. } => *visible,
        ManagedLayoutNode::Split { first, second, .. } => {
            contains_visible_panel(first) || contains_visible_panel(second)
        }
    }
}

fn split_area(
    direction: ManagedLayoutDirection,
    first_size: ManagedSize,
    area: Rect,
) -> (Rect, Rect) {
    let total = match direction {
        ManagedLayoutDirection::Horizontal => area.width,
        ManagedLayoutDirection::Vertical => area.height,
    };
    let first_len = first_length(first_size, total);
    match direction {
        ManagedLayoutDirection::Horizontal => {
            let first = Rect {
                width: first_len,
                ..area
            };
            let second = Rect {
                x: area.x.saturating_add(first_len),
                width: area.width.saturating_sub(first_len),
                ..area
            };
            (first, second)
        }
        ManagedLayoutDirection::Vertical => {
            let first = Rect {
                height: first_len,
                ..area
            };
            let second = Rect {
                y: area.y.saturating_add(first_len),
                height: area.height.saturating_sub(first_len),
                ..area
            };
            (first, second)
        }
    }
}

fn first_length(size: ManagedSize, total: u16) -> u16 {
    if total <= 1 {
        return total;
    }
    match size {
        ManagedSize::Cells(cells) => cells.min(total.saturating_sub(1)),
        ManagedSize::Percent(percent) => {
            let length = total.saturating_mul(percent.min(100)) / 100;
            length.clamp(1, total.saturating_sub(1))
        }
        ManagedSize::RemainingAfterSecond(cells) => {
            total.saturating_sub(cells.min(total.saturating_sub(1)))
        }
    }
}

fn resize_cells(value: u16, delta: i16, minimum: u16, maximum: u16) -> u16 {
    value.saturating_add_signed(delta).clamp(minimum, maximum)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: u16, height: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    #[test]
    fn nested_splits_resolve_panel_areas() {
        let root = ManagedLayoutNode::split(
            ManagedLayoutDirection::Horizontal,
            ManagedLayoutNode::panel(ManagedPanelId::Tracks, 10, 5),
            ManagedLayoutNode::split(
                ManagedLayoutDirection::Vertical,
                ManagedLayoutNode::panel(ManagedPanelId::Pattern, 20, 6),
                ManagedLayoutNode::panel(ManagedPanelId::TrackDesk, 20, 4),
                ManagedSize::RemainingAfterSecond(5),
            ),
            ManagedSize::Cells(12),
        );

        let resolved = resolve_managed_layout(&root, area(80, 24));

        assert_eq!(resolved.panels.len(), 3);
        assert_eq!(resolved.panels[0].area.width, 12);
        assert_eq!(resolved.panels[1].area.height, 19);
        assert_eq!(resolved.panels[2].area.y, 19);
    }

    #[test]
    fn hidden_panels_are_removed_and_neighbors_fill_space() {
        let root = ManagedLayoutNode::split(
            ManagedLayoutDirection::Horizontal,
            ManagedLayoutNode::hidden_panel(ManagedPanelId::Tracks, 10, 5),
            ManagedLayoutNode::panel(ManagedPanelId::Pattern, 20, 5),
            ManagedSize::Cells(20),
        );

        let resolved = resolve_managed_layout(&root, area(72, 20));

        assert_eq!(resolved.panels.len(), 1);
        assert_eq!(resolved.panels[0].id, ManagedPanelId::Pattern);
        assert_eq!(resolved.panels[0].area, area(72, 20));
    }

    #[test]
    fn tracker_layout_presets_choose_representative_panels() {
        let compact = resolve_tracker_layout(
            area(120, 30),
            TrackerLayoutState::from_preset(TrackerLayoutPreset::Compact),
        );
        let studio = resolve_tracker_layout(
            area(120, 30),
            TrackerLayoutState::from_preset(TrackerLayoutPreset::Studio),
        );

        assert!(compact.tracks.is_none());
        assert!(compact.inspector.is_none());
        assert!(studio.tracks.is_some());
        assert!(studio.inspector.is_some());
        assert!(studio.pattern.width < compact.pattern.width);
    }

    #[test]
    fn tracker_layout_reports_minimum_constraint_diagnostics() {
        let resolved = resolve_tracker_layout(
            area(40, 8),
            TrackerLayoutState::from_preset(TrackerLayoutPreset::Studio),
        );

        assert!(resolved.diagnostics > 0);
        assert!(resolved.pattern.width > 0);
    }

    #[test]
    fn panel_resize_is_clamped_to_safe_bounds() {
        let mut state = TrackerLayoutState::default();

        state.resize_panel(ManagedPanelId::Inspector, 200);
        state.resize_panel(ManagedPanelId::TrackDesk, -200);

        assert_eq!(state.inspector_width, 64);
        assert_eq!(state.track_desk_height, 6);
    }
}
