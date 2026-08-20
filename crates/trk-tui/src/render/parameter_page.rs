use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Paragraph},
};
use trk_core::ParameterPage;

use crate::{interaction_region, InteractionMap, InteractionPayload};

use super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterPageViewState<'a> {
    pub page: ParameterPage,
    pub selected: usize,
    pub row: usize,
    pub track_number: usize,
    pub track_name: &'a str,
    pub slots: &'a [ParameterPageSlotView],
    pub has_snapshot: bool,
    pub reload_pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterPageSlotView {
    pub key: char,
    pub label: String,
    pub value: String,
    pub meter_percent: u8,
    pub locked: bool,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

pub(super) fn render_parameter_page(
    frame: &mut Frame<'_>,
    area: Rect,
    page: Option<ParameterPageViewState<'_>>,
    interactions: &mut InteractionMap,
) {
    let Some(page) = page else {
        frame.render_widget(
            Paragraph::new("No active track").block(theme::block(" Parameter Pages ")),
            area,
        );
        return;
    };
    let outer = Block::default()
        .title(format!(
            " Track {:02} [{}] · Row {:02} ",
            page.track_number, page.track_name, page.row
        ))
        .borders(Borders::ALL)
        .border_style(theme::label());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(inner);
    render_tabs(frame, vertical[0], page.page);
    render_slots(frame, vertical[1], page, interactions);
    render_footer(frame, vertical[2], page);
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, active: ParameterPage) {
    let spans = ParameterPage::ALL
        .into_iter()
        .enumerate()
        .flat_map(|(index, page)| {
            let style = if page == active {
                theme::active()
            } else {
                theme::muted()
            };
            [
                Span::styled(format!(" F{} {} ", index + 1, page.label()), style),
                Span::raw(" "),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_slots(
    frame: &mut Frame<'_>,
    area: Rect,
    page: ParameterPageViewState<'_>,
    interactions: &mut InteractionMap,
) {
    let columns = if area.width >= 72 { 4 } else { 2 };
    let rows = 8_usize.div_ceil(columns);
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Ratio(1, rows as u32); rows])
        .split(area);
    for (row_index, row_area) in row_areas.iter().enumerate() {
        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Ratio(1, columns as u32); columns])
            .split(*row_area);
        for (column_index, slot_area) in column_areas.iter().enumerate() {
            let index = row_index * columns + column_index;
            let Some(slot) = page.slots.get(index) else {
                continue;
            };
            render_slot(frame, *slot_area, slot, index == page.selected);
            if slot.enabled {
                interactions.register_with_payload(
                    interaction_region::PARAMETER_PAGE_SLOT,
                    *slot_area,
                    InteractionPayload::ParameterPageSlot { index },
                );
            }
        }
    }
}

fn render_slot(frame: &mut Frame<'_>, area: Rect, slot: &ParameterPageSlotView, selected: bool) {
    let lock = if slot.locked { " ●" } else { "" };
    let title_style = if !slot.enabled {
        theme::disabled()
    } else if selected {
        theme::active()
    } else if slot.locked {
        theme::playing()
    } else {
        theme::label()
    };
    let block = Block::default()
        .title(Span::styled(
            format!(" ({}) {}{} ", slot.key, slot.label, lock),
            title_style,
        ))
        .borders(Borders::ALL)
        .border_style(if selected {
            theme::active()
        } else {
            theme::muted()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let content = if slot.enabled {
        let meter_width = usize::from(inner.width.saturating_sub(2)).min(18);
        let filled = meter_width * usize::from(slot.meter_percent) / 100;
        let meter = format!(
            "[{}{}]",
            "=".repeat(filled),
            "-".repeat(meter_width.saturating_sub(filled))
        );
        vec![
            Line::styled(slot.value.clone(), theme::base()),
            Line::styled(meter, theme::playing()),
        ]
    } else {
        vec![Line::styled(
            slot.disabled_reason
                .as_deref()
                .unwrap_or("Unavailable")
                .to_string(),
            theme::disabled(),
        )]
    };
    frame.render_widget(Paragraph::new(content), inner);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, page: ParameterPageViewState<'_>) {
    let tags = page
        .slots
        .iter()
        .filter(|slot| slot.locked)
        .map(|slot| format!("[{}:{}]", slot.label, slot.value))
        .collect::<Vec<_>>();
    let snapshot = if page.reload_pending {
        "RELOAD QUEUED"
    } else if page.has_snapshot {
        "TEMP SAVED"
    } else {
        "NO TEMP"
    };
    let mut spans = vec![
        Span::styled("P-Locks ", theme::label()),
        Span::styled(
            if tags.is_empty() {
                "none".to_string()
            } else {
                tags.join(" ")
            },
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  · {snapshot}"), theme::muted()),
    ];
    if area.width >= 72 {
        spans.push(Span::styled(
            "  · +/- fine · Shift coarse · Backspace clears",
            theme::muted(),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render_with_interactions, TuiView};
    use ratatui::{backend::TestBackend, Terminal};
    use trk_core::Song;

    fn slots() -> Vec<ParameterPageSlotView> {
        (0..8)
            .map(|index| ParameterPageSlotView {
                key: trk_core::PARAMETER_ENCODER_KEYS[index],
                label: format!("Param {}", index + 1),
                value: format!("{}%", index * 10),
                meter_percent: (index * 10) as u8,
                locked: index == 1,
                enabled: index != 7,
                disabled_reason: (index == 7).then(|| "Unavailable".to_string()),
            })
            .collect()
    }

    #[test]
    fn page_renders_all_slots_lock_tags_and_exact_pointer_regions() {
        let slots = slots();
        let mut state = super::super::render_test_support::render_test_state();
        state.active_view = TuiView::ParameterPage;
        state.mode_label = "PARAM";
        state.parameter_page = Some(ParameterPageViewState {
            page: ParameterPage::Filter,
            selected: 1,
            row: 3,
            track_number: 1,
            track_name: "Bass",
            slots: &slots,
            has_snapshot: true,
            reload_pending: false,
        });
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let mut interactions = InteractionMap::new();
        terminal
            .draw(|frame| {
                interactions = render_with_interactions(frame, &Song::empty(), state);
            })
            .expect("draw");
        let rendered = super::super::render_test_support::terminal_buffer_text(&terminal);

        assert!(rendered.contains("F2 FLTR"));
        assert!(rendered.contains("(W) Param 2 ●"));
        assert!(rendered.contains("[Param 2:10%]"));
        assert!(rendered.contains("TEMP SAVED"));
        assert!(rendered.contains("Unavailable"));
        assert_eq!(
            interactions
                .regions()
                .iter()
                .filter(|region| region.id == interaction_region::PARAMETER_PAGE_SLOT)
                .count(),
            7
        );
    }

    #[test]
    fn narrow_page_reflows_without_dropping_encoder_slots() {
        let slots = slots();
        let backend = TestBackend::new(58, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                render_parameter_page(
                    frame,
                    frame.area(),
                    Some(ParameterPageViewState {
                        page: ParameterPage::Source,
                        selected: 0,
                        row: 0,
                        track_number: 1,
                        track_name: "Drums",
                        slots: &slots,
                        has_snapshot: false,
                        reload_pending: false,
                    }),
                    &mut InteractionMap::new(),
                );
            })
            .expect("draw");
        let rendered = super::super::render_test_support::terminal_buffer_text(&terminal);
        for key in trk_core::PARAMETER_ENCODER_KEYS {
            assert!(rendered.contains(&format!("({key})")), "missing {key}");
        }
    }
}
