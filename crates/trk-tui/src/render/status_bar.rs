use ratatui::{
    layout::Rect,
    prelude::{Frame, Line, Modifier, Span},
    widgets::Paragraph,
};

use super::{theme, NotificationKind, TuiState, TuiView};
use crate::PatternFieldLayout;

pub(super) fn render_status(frame: &mut Frame<'_>, area: Rect, state: TuiState<'_>) {
    if let Some(command_line) = state.command_line {
        frame.render_widget(Paragraph::new(format!(" :{command_line}")), area);
        return;
    }

    if let Some(notification) = state.notification {
        let label = match notification.kind {
            NotificationKind::Info => "INFO",
            NotificationKind::Success => "OK",
            NotificationKind::Warning => "WARN",
            NotificationKind::Error => "ERR",
        };
        let style = match notification.kind {
            NotificationKind::Info => theme::label(),
            NotificationKind::Success => theme::playing(),
            NotificationKind::Warning => theme::warning(),
            NotificationKind::Error => theme::error(),
        };
        let status = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {label} "), style.add_modifier(Modifier::BOLD)),
            Span::styled(notification.message.to_string(), theme::base()),
        ]));
        frame.render_widget(status, area);
        return;
    }

    frame.render_widget(
        Paragraph::new(compose_shortcut_status(state, area.width)).style(theme::base()),
        area,
    );
}

fn compose_shortcut_status(state: TuiState<'_>, available_width: u16) -> String {
    let (leading, actions) = status_segments(state);
    let mut text = format!(" {leading}");
    let available_width = usize::from(available_width);
    if Line::from(text.as_str()).width() > available_width {
        return String::new();
    }

    for action in actions.iter().take(3) {
        let candidate = format!("{text} | {action}");
        if Line::from(candidate.as_str()).width() > available_width {
            return finish_status_line(text, available_width);
        }
        text = candidate;
    }

    if let Some(context) = contextual_status_segment(state) {
        let candidate = format!("{text} | {context}");
        if Line::from(candidate.as_str()).width() > available_width {
            return finish_status_line(text, available_width);
        }
        text = candidate;
    }

    for action in actions.iter().skip(3) {
        let candidate = format!("{text} | {action}");
        if Line::from(candidate.as_str()).width() > available_width {
            break;
        }
        text = candidate;
    }

    finish_status_line(text, available_width)
}

fn finish_status_line(mut text: String, available_width: usize) -> String {
    if Line::from(text.as_str()).width() < available_width {
        text.push(' ');
    }
    text
}

fn contextual_status_segment(state: TuiState<'_>) -> Option<String> {
    (state.active_view == TuiView::Pattern
        && state.tracker_layout.pattern_fields != PatternFieldLayout::Full)
        .then(|| format!("Fields {}", state.tracker_layout.pattern_fields.label()))
}

fn status_segments(state: TuiState<'_>) -> (String, &'static [&'static str]) {
    match state.active_view {
        TuiView::Pattern => {
            let selection = if state.selection.is_some() {
                " SEL"
            } else {
                ""
            };
            (
                format!(
                    "{}{} | Step {}",
                    state.mode_label, selection, state.edit_step
                ),
                &[
                    "Ctrl+P Palette",
                    "H Help",
                    "Space Play/Stop",
                    "b Web",
                    "v History",
                    "Enter Row",
                    "Shift+Enter Seq",
                    "L Loop",
                    "N/P/X Pattern",
                    "A/Y/R Seq",
                    ": Command",
                    "i Edit",
                    "V Select",
                    "Ctrl+S Save",
                    "q Quit",
                ],
            )
        }
        TuiView::PianoRoll { .. } => (
            state.mode_label.to_string(),
            &[
                "Esc Tracker",
                "Arrows Cursor",
                "Space Note",
                "Shift+Left/Right Gate",
                "Alt+Arrows Move",
                "1-9 Velocity",
                "[/] Zoom",
                "g Ghosts",
                ": Command",
            ],
        ),
        TuiView::ParameterPage => (
            state.mode_label.to_string(),
            &[
                "F1-F6 Pages",
                "QWER/ASDF Select",
                "+/- Adjust",
                "Shift Coarse",
                "Backspace+Key Clear",
                "Shift+1..8 Mute",
                "Esc Tracker",
            ],
        ),
        TuiView::Sequence => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Pattern",
                "A Add",
                "R Remove",
                "Y Duplicate",
                "T Set Pattern",
                "</> Move",
                "Enter Play",
                ": Command",
                "Ctrl+S Save",
                "Ctrl+Shift+S Save As",
                "q Quit",
            ],
        ),
        TuiView::Clips => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Pattern",
                "Arrows Select",
                "Enter Queue",
                "Space Launch",
                "F8 Stop",
                "A Add Scene",
                "T Set Pattern",
                "R Clear",
                ": Command",
                "q Quit",
            ],
        ),
        TuiView::Tracks => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Pattern",
                "N New",
                "D Duplicate",
                "r Rename",
                "c Channel",
                "Del Delete",
                "{/} Move",
                "M/S Mute/Solo",
                ": Command",
                "Ctrl+S Save",
                "Ctrl+Shift+S Save As",
                "q Quit",
            ],
        ),
        TuiView::Patterns => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Pattern",
                "N New",
                "P Duplicate",
                "r Rename",
                "X/Del Delete",
                "1-5 Length Presets",
                "F6 Length",
                ": Command",
                "Ctrl+S Save",
                "Ctrl+Shift+S Save As",
                "q Quit",
            ],
        ),
        TuiView::Sampler => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Pattern",
                "Tab ADSR",
                "[/]/{/} Adjust",
                "+/- Zoom",
                "Left/Right Pan",
                "b Browse",
                "F7 Sequence",
                "F9 Tracks",
                "F10 Patterns",
                ": Command",
                "Ctrl+S Save",
                "q Quit",
            ],
        ),
        TuiView::DspRack => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Pattern",
                "Tab Track/Master",
                "Up/Down Device",
                "[/]/Left/Right Param",
                "A Add",
                "P/R/C Lock",
                "Ctrl+S Save",
                "q Quit",
            ],
        ),
        TuiView::SampleBrowser => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Sampler",
                "Up/Down Select",
                "A Assign",
                "Right-click Assign",
                "Enter Load/Open",
                "Backspace Parent",
                ": Command",
                "q Quit",
            ],
        ),
        TuiView::ProjectBrowser => (
            state.mode_label.to_string(),
            &[
                "H Help",
                "Esc Tracker",
                "Up/Down Select",
                "Enter Open",
                "Backspace Parent",
                "r Refresh",
                ": Command",
                "q Quit",
            ],
        ),
        TuiView::AiChat => (
            state.mode_label.to_string(),
            &[
                "m Engines",
                "Enter Submit",
                "a Apply",
                "r Reject",
                "p Preview",
                "Ctrl+C Cancel Task",
                "Esc Tracker",
                ": Command",
                "q Quit",
            ],
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_test_support::render_test_state;

    const VIEWS: &[(TuiView, &str)] = &[
        (TuiView::Pattern, "NORMAL"),
        (TuiView::ParameterPage, "PARAM"),
        (TuiView::Sequence, "SEQUENCE"),
        (TuiView::Clips, "CLIPS"),
        (TuiView::Tracks, "TRACKS"),
        (TuiView::Patterns, "PATTERNS"),
        (TuiView::Sampler, "SAMPLER"),
        (TuiView::DspRack, "DSP RACK"),
        (TuiView::SampleBrowser, "SAMPLE BROWSER"),
        (TuiView::ProjectBrowser, "PROJECT BROWSER"),
        (TuiView::AiChat, "AI CHAT"),
    ];

    #[test]
    fn every_main_view_keeps_mode_and_three_priority_actions_at_72_columns() {
        for &(active_view, mode_label) in VIEWS {
            let state = TuiState {
                active_view,
                mode_label,
                ..render_test_state()
            };
            let text = compose_shortcut_status(state, 72);
            let (_, actions) = status_segments(state);

            assert!(text.contains(mode_label), "{active_view:?}: {text}");
            for action in &actions[..3] {
                assert!(text.contains(action), "{active_view:?}: missing {action}");
            }
            assert!(Line::from(text).width() <= 72);
        }
    }

    #[test]
    fn compact_pattern_field_context_does_not_displace_priority_actions() {
        for pattern_fields in [
            PatternFieldLayout::Note,
            PatternFieldLayout::Instrument,
            PatternFieldLayout::Fx,
            PatternFieldLayout::NoteInstrument,
            PatternFieldLayout::NoteFx,
            PatternFieldLayout::InstrumentFx,
        ] {
            let mut state = render_test_state();
            state.tracker_layout.pattern_fields = pattern_fields;
            let text = compose_shortcut_status(state, 72);

            for action in ["Ctrl+P Palette", "H Help", "Space Play/Stop"] {
                assert!(
                    text.contains(action),
                    "{pattern_fields:?}: missing {action}"
                );
            }
            assert!(Line::from(text).width() <= 72);
        }
    }

    #[test]
    fn widening_compact_pattern_status_never_removes_an_already_visible_segment() {
        let mut state = render_test_state();
        state.tracker_layout.pattern_fields = PatternFieldLayout::Note;
        let (_, actions) = status_segments(state);
        let mut visible = Vec::new();

        for width in 72..=160 {
            let text = compose_shortcut_status(state, width);
            for segment in visible.iter().copied() {
                assert!(text.contains(segment), "width {width} removed {segment}");
            }
            for segment in actions
                .iter()
                .copied()
                .chain(std::iter::once("Fields note"))
            {
                if text.contains(segment) && !visible.contains(&segment) {
                    visible.push(segment);
                }
            }
        }
    }

    #[test]
    fn a_segment_is_omitted_whole_when_its_delimiter_and_label_do_not_fit() {
        let state = render_test_state();
        let first_three = compose_shortcut_status(state, 60);
        let (_, actions) = status_segments(state);
        let next_action = actions[3];
        let next_candidate = format!("{} | {next_action}", first_three.trim_end());
        let constrained_width = Line::from(next_candidate).width() - 1;
        let constrained = compose_shortcut_status(state, constrained_width as u16);

        assert_eq!(constrained.trim_end(), first_three.trim_end());
        assert!(!constrained.contains(next_action));
        assert!(!constrained.ends_with(" |"));
        assert!(Line::from(constrained).width() <= constrained_width);
    }
}
