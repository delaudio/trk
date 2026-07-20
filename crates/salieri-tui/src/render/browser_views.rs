use ratatui::{
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

use crate::ViewportAxis;

use super::{
    render_sampler_view, truncate, ProjectBrowserEntryKind, ProjectBrowserViewState,
    SampleBrowserEntryKind, SampleBrowserViewState,
};

pub(super) fn render_sample_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: Option<SampleBrowserViewState<'_>>,
) {
    let Some(browser) = browser else {
        let empty = Paragraph::new("Sample browser unavailable").block(
            Block::default()
                .title(" Sample Browser ")
                .borders(Borders::ALL),
        );
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
    .block(Block::default().title(" Directory ").borders(Borders::ALL));
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
        for (index, entry) in browser
            .entries
            .iter()
            .enumerate()
            .skip(viewport.offset())
            .take(visible_rows)
        {
            let marker = if index == selected { ">" } else { " " };
            let icon = match entry.kind {
                SampleBrowserEntryKind::Directory => "[D]",
                SampleBrowserEntryKind::SupportedSample => "[W]",
                SampleBrowserEntryKind::UnsupportedFile => "[ ]",
            };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                match entry.kind {
                    SampleBrowserEntryKind::Directory => Style::default().fg(Color::Cyan),
                    SampleBrowserEntryKind::SupportedSample => Style::default().fg(Color::White),
                    SampleBrowserEntryKind::UnsupportedFile => Style::default().fg(Color::DarkGray),
                }
            };
            lines.push(Line::from(Span::styled(
                format!("{marker} {icon} {}", truncate(entry.name, 38)),
                style,
            )));
        }
    }

    let list = Paragraph::new(lines)
        .block(Block::default().title(" Samples ").borders(Borders::ALL))
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
            .block(Block::default().title(" Preview ").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(preview, columns[1]);
    }
}

pub(super) fn render_project_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    browser: Option<ProjectBrowserViewState<'_>>,
) {
    let Some(browser) = browser else {
        let empty = Paragraph::new("Project browser unavailable").block(
            Block::default()
                .title(" Project Browser ")
                .borders(Borders::ALL),
        );
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
    .block(Block::default().title(" Directory ").borders(Borders::ALL));
    frame.render_widget(path, left[0]);

    let visible_rows = left[1].height.saturating_sub(2) as usize;
    let selected = browser
        .selected
        .min(browser.entries.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let mut lines = Vec::new();

    if browser.entries.is_empty() {
        lines.push(Line::from("No projects"));
    } else {
        for (index, entry) in browser
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
        {
            let marker = if index == selected { ">" } else { " " };
            let icon = match entry.kind {
                ProjectBrowserEntryKind::Directory => "[D]",
                ProjectBrowserEntryKind::RecentProject => "[R]",
                ProjectBrowserEntryKind::Project => "[S]",
                ProjectBrowserEntryKind::MissingProject => "[!]",
                ProjectBrowserEntryKind::InvalidProject => "[X]",
            };
            let style = if index == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                match entry.kind {
                    ProjectBrowserEntryKind::Directory => Style::default().fg(Color::Cyan),
                    ProjectBrowserEntryKind::RecentProject => Style::default().fg(Color::Green),
                    ProjectBrowserEntryKind::Project => Style::default().fg(Color::White),
                    ProjectBrowserEntryKind::MissingProject
                    | ProjectBrowserEntryKind::InvalidProject => Style::default().fg(Color::Red),
                }
            };
            lines.push(Line::from(Span::styled(
                format!("{marker} {icon} {}", truncate(entry.name, 42)),
                style,
            )));
        }
    }

    let list = Paragraph::new(lines)
        .block(Block::default().title(" Projects ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(list, left[1]);

    let mut detail_lines = Vec::new();
    if let Some(entry) = browser.entries.get(selected) {
        detail_lines.push(Line::from(vec![
            Span::styled("Name ", Style::default().fg(Color::Yellow)),
            Span::raw(entry.name.to_string()),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Type ", Style::default().fg(Color::Yellow)),
            Span::raw(match entry.kind {
                ProjectBrowserEntryKind::Directory => "directory",
                ProjectBrowserEntryKind::RecentProject => "recent project",
                ProjectBrowserEntryKind::Project => "project",
                ProjectBrowserEntryKind::MissingProject => "missing project",
                ProjectBrowserEntryKind::InvalidProject => "invalid project",
            }),
        ]));
        detail_lines.push(Line::from(vec![
            Span::styled("Path ", Style::default().fg(Color::Yellow)),
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
            Style::default().fg(Color::Yellow),
        )));
    }

    let details = Paragraph::new(detail_lines)
        .block(Block::default().title(" Details ").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, columns[1]);
}
