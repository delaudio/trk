use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Frame, Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

use crate::{
    interaction_region, InteractionMap, InteractionPayload, InteractionRegionId, ViewportAxis,
};

use super::{
    render_sampler_view, theme, truncate, ProjectBrowserEntryKind, ProjectBrowserEntryView,
    ProjectBrowserViewState, SampleBrowserEntryKind, SampleBrowserViewState,
};

pub(super) fn render_sample_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: Option<SampleBrowserViewState<'_>>,
    interactions: &mut InteractionMap,
) {
    let Some(browser) = browser else {
        let empty =
            Paragraph::new("Sample browser unavailable").block(theme::block(" Sample Browser "));
        frame.render_widget(empty, area);
        return;
    };

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);
    let left = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(columns[0]);

    let path = Paragraph::new(truncate(
        browser.current_dir,
        columns[0].width.saturating_sub(4) as usize,
    ))
    .block(theme::block(" Directory "));
    frame.render_widget(path, left[0]);

    let visible_rows = left[1].height.saturating_sub(2) as usize;
    let selected = browser
        .selected
        .min(browser.entries.len().saturating_sub(1));
    let mut viewport = ViewportAxis::new(browser.entries.len(), visible_rows);
    viewport.keep_visible(selected);
    let mut lines = Vec::new();

    if browser.entries.is_empty() {
        lines.push(Line::from("No files"));
    } else {
        for (visible_row, (index, entry)) in browser
            .entries
            .iter()
            .enumerate()
            .skip(viewport.offset())
            .take(visible_rows)
            .enumerate()
        {
            let marker = if index == selected { ">" } else { " " };
            let icon = match entry.kind {
                SampleBrowserEntryKind::Directory => "[D]",
                SampleBrowserEntryKind::SupportedSample => "[W]",
                SampleBrowserEntryKind::UnsupportedFile => "[ ]",
            };
            let style = if index == selected {
                theme::active()
            } else {
                match entry.kind {
                    SampleBrowserEntryKind::Directory => theme::label(),
                    SampleBrowserEntryKind::SupportedSample => theme::base(),
                    SampleBrowserEntryKind::UnsupportedFile => theme::muted(),
                }
            };
            lines.push(Line::from(Span::styled(
                format!("{marker} {icon} {}", truncate(entry.name, 38)),
                style,
            )));
            register_browser_entry(
                interactions,
                interaction_region::SAMPLE_BROWSER_ENTRY,
                left[1],
                visible_row,
                InteractionPayload::SampleBrowserEntry { index },
            );
        }
    }

    let list = Paragraph::new(lines)
        .block(theme::block(" Samples "))
        .wrap(Wrap { trim: false });
    frame.render_widget(list, left[1]);
    if browser.entries.len() > visible_rows {
        let mut scrollbar_state = viewport.scrollbar_state();
        frame.render_stateful_widget(
            Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
            left[1],
            &mut scrollbar_state,
        );
    }

    if let Some(preview) = browser.preview {
        render_sampler_view(frame, columns[1], Some(preview));
    } else {
        let message = browser.message.unwrap_or("Select a WAV file to preview it");
        let preview = Paragraph::new(message)
            .block(theme::block(" Preview "))
            .wrap(Wrap { trim: true });
        frame.render_widget(preview, columns[1]);
    }
}

pub(super) fn render_project_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: Option<ProjectBrowserViewState<'_>>,
    interactions: &mut InteractionMap,
) {
    let Some(browser) = browser else {
        let empty =
            Paragraph::new("Project browser unavailable").block(theme::block(" Project Browser "));
        frame.render_widget(empty, area);
        return;
    };

    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);
    let left = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(columns[0]);

    let path = Paragraph::new(truncate(
        browser.current_dir,
        columns[0].width.saturating_sub(4) as usize,
    ))
    .block(theme::block(" Directory "));
    frame.render_widget(path, left[0]);

    let visible_rows = left[1].height.saturating_sub(2) as usize;
    let selected = browser
        .selected
        .min(browser.entries.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let mut lines = Vec::new();

    if browser.entries.is_empty() {
        lines.push(Line::from(browser.message.unwrap_or(
            "No projects. Import demos into fixtures/local/renoise-demos/.",
        )));
    } else if is_renoise_demo_browser(browser) {
        for (visible_row, (line, entry_index)) in renoise_demo_project_rows(browser, selected)
            .into_iter()
            .take(visible_rows)
            .enumerate()
        {
            lines.push(line);
            if let Some(index) = entry_index {
                register_browser_entry(
                    interactions,
                    interaction_region::PROJECT_BROWSER_ENTRY,
                    left[1],
                    visible_row,
                    InteractionPayload::ProjectBrowserEntry { index },
                );
            }
        }
    } else {
        for (visible_row, (index, entry)) in browser
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
            .enumerate()
        {
            lines.push(project_entry_line(index, entry, selected));
            register_browser_entry(
                interactions,
                interaction_region::PROJECT_BROWSER_ENTRY,
                left[1],
                visible_row,
                InteractionPayload::ProjectBrowserEntry { index },
            );
        }
    }

    let list = Paragraph::new(lines)
        .block(theme::block(" Projects "))
        .wrap(Wrap { trim: false });
    frame.render_widget(list, left[1]);

    let mut detail_lines = Vec::new();
    if let Some(entry) = browser.entries.get(selected) {
        detail_lines.push(Line::from(vec![
            Span::styled("Name ", theme::label()),
            Span::raw(entry.name.to_string()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Type ", theme::label()),
            Span::raw(match entry.kind {
                ProjectBrowserEntryKind::Directory => "directory",
                ProjectBrowserEntryKind::RecentProject => "recent project",
                ProjectBrowserEntryKind::Project => "project",
                ProjectBrowserEntryKind::MissingProject => "missing project",
                ProjectBrowserEntryKind::InvalidProject => "invalid project",
            }),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Path ", theme::label()),
            Span::raw(entry.path.to_string()),
        ]));
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(entry.detail.to_string()));
    } else {
        detail_lines.push(Line::from(browser.message.unwrap_or("No project selected")));
    }
    if let Some(message) = browser.message {
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(Span::styled(
            message.to_string(),
            theme::warning(),
        )));
    }

    let details = Paragraph::new(detail_lines)
        .block(theme::block(" Details "))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, columns[1]);
}

fn is_renoise_demo_browser(browser: ProjectBrowserViewState<'_>) -> bool {
    browser.current_dir.contains("renoise-demos")
        || browser.entries.iter().any(|entry| {
            entry.name.starts_with("DemoSong")
                || entry.name.starts_with("Tutorial")
                || matches!(entry.name, "Samples" | "Songs" | "Instruments")
        })
}

fn renoise_demo_project_rows(
    browser: ProjectBrowserViewState<'_>,
    selected: usize,
) -> Vec<(Line<'static>, Option<usize>)> {
    let sections = ["Samples", "Songs", "Tutorial", "Instruments"];
    let mut lines = Vec::new();
    for section in sections {
        let entries = browser
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| demo_section(entry) == section)
            .collect::<Vec<_>>();
        if entries.is_empty() {
            continue;
        }
        lines.push((Line::from(theme::label_span(format!("▾ {section}"))), None));
        for (index, entry) in entries {
            lines.push((project_entry_line(index, entry, selected), Some(index)));
        }
    }
    lines
}

fn register_browser_entry(
    interactions: &mut InteractionMap,
    id: InteractionRegionId,
    list_area: Rect,
    visible_row: usize,
    payload: InteractionPayload,
) {
    let inner = Rect::new(
        list_area.x.saturating_add(1),
        list_area.y.saturating_add(1),
        list_area.width.saturating_sub(2),
        list_area.height.saturating_sub(2),
    );
    let y = inner.y.saturating_add(visible_row as u16);
    if y < inner.y.saturating_add(inner.height) {
        interactions.register_with_payload(id, Rect::new(inner.x, y, inner.width, 1), payload);
    }
}

fn demo_section(entry: &ProjectBrowserEntryView<'_>) -> &'static str {
    if entry.name == "Samples" {
        "Samples"
    } else if entry.name == "Instruments" {
        "Instruments"
    } else if entry.name.starts_with("Tutorial") {
        "Tutorial"
    } else {
        "Songs"
    }
}

fn project_entry_line(
    index: usize,
    entry: &ProjectBrowserEntryView<'_>,
    selected: usize,
) -> Line<'static> {
    let marker = if index == selected { ">" } else { " " };
    let icon = match entry.kind {
        ProjectBrowserEntryKind::Directory => "[D]",
        ProjectBrowserEntryKind::RecentProject => "[R]",
        ProjectBrowserEntryKind::Project => "[S]",
        ProjectBrowserEntryKind::MissingProject => "[!]",
        ProjectBrowserEntryKind::InvalidProject => "[X]",
    };
    let style = if index == selected {
        theme::active()
    } else {
        match entry.kind {
            ProjectBrowserEntryKind::Directory => theme::label(),
            ProjectBrowserEntryKind::RecentProject => theme::playing(),
            ProjectBrowserEntryKind::Project => theme::base(),
            ProjectBrowserEntryKind::MissingProject | ProjectBrowserEntryKind::InvalidProject => {
                theme::error()
            }
        }
    };
    Line::from(Span::styled(
        format!("{marker} {icon} {}", truncate(entry.name, 42)),
        style,
    ))
}
