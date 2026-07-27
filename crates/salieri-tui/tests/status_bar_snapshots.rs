use salieri_core::Song;
use salieri_tui::{NotificationKind, NotificationView, TuiState, TuiView};

#[allow(dead_code)]
mod support;
use support::{assert_snapshot, render_snapshot, test_state};

#[test]
fn snapshots_all_main_view_status_bars_at_narrow_width() {
    assert_snapshot("status-bars-narrow", render_status_matrix(72));
}

#[test]
fn snapshots_all_main_view_status_bars_at_medium_width() {
    assert_snapshot("status-bars-medium", render_status_matrix(100));
}

fn render_status_matrix(width: u16) -> String {
    let views = [
        ("Pattern", TuiView::Pattern, "NORMAL"),
        ("Sequence", TuiView::Sequence, "SEQUENCE"),
        ("Clips", TuiView::Clips, "CLIPS"),
        ("Tracks", TuiView::Tracks, "TRACKS"),
        ("Patterns", TuiView::Patterns, "PATTERNS"),
        ("Sampler", TuiView::Sampler, "SAMPLER"),
        ("DSP Rack", TuiView::DspRack, "DSP RACK"),
        ("Sample Browser", TuiView::SampleBrowser, "SAMPLE BROWSER"),
        (
            "Project Browser",
            TuiView::ProjectBrowser,
            "PROJECT BROWSER",
        ),
        ("AI Chat", TuiView::AiChat, "AI CHAT"),
    ];
    let mut output = String::new();
    for (label, active_view, mode_label) in views {
        let rendered = render_snapshot(
            Song::empty(),
            TuiState {
                active_view,
                mode_label,
                ..test_state()
            },
            width,
            24,
        );
        let status = rendered.lines().last().unwrap_or_default().trim_end();
        output.push_str(&format!("{label:<16}|{status}\n"));
    }

    let command = render_snapshot(
        Song::empty(),
        TuiState {
            command_line: Some("write demo.salieri"),
            notification: Some(NotificationView {
                kind: NotificationKind::Warning,
                message: "hidden while command input is active",
            }),
            ..test_state()
        },
        width,
        24,
    );
    output.push_str(&format!(
        "{:<16}|{}\n",
        "Command input",
        command.lines().last().unwrap_or_default().trim_end()
    ));

    let notification = render_snapshot(
        Song::empty(),
        TuiState {
            notification: Some(NotificationView {
                kind: NotificationKind::Success,
                message: "Project saved",
            }),
            ..test_state()
        },
        width,
        24,
    );
    output.push_str(&format!(
        "{:<16}|{}\n",
        "Notification",
        notification.lines().last().unwrap_or_default().trim_end()
    ));
    output
}
