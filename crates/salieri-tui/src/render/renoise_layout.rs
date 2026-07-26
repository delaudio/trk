use ratatui::layout::{Constraint, Direction, Layout, Rect};

const LEFT_WIDTH: u16 = 15;
const RIGHT_WIDTH: u16 = 38;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PatternWorkspaceLayout {
    pub analyzer: Rect,
    pub util: Rect,
    pub pattern: Rect,
    pub inspector: Rect,
    pub effects: Rect,
    pub mixer: Rect,
    pub vu: Rect,
    pub device_chain: Rect,
}

pub(super) fn pattern_workspace_layout(area: Rect) -> PatternWorkspaceLayout {
    let compact_height = area.height < 34;
    let top_height = if compact_height { 4 } else { 6 };
    let bottom_height = if compact_height { 7 } else { 10 };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_height),
            Constraint::Min(8),
            Constraint::Length(bottom_height),
        ])
        .split(area);
    let middle = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(LEFT_WIDTH),
            Constraint::Min(52),
            Constraint::Length(RIGHT_WIDTH),
        ])
        .split(rows[1]);
    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(28),
            Constraint::Percentage(20),
            Constraint::Percentage(28),
        ])
        .split(rows[2]);

    PatternWorkspaceLayout {
        analyzer: rows[0],
        util: middle[0],
        pattern: middle[1],
        inspector: middle[2],
        effects: bottom[0],
        mixer: bottom[1],
        vu: bottom[2],
        device_chain: bottom[3],
    }
}
