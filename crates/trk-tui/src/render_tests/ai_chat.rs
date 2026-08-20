use ratatui::{backend::TestBackend, Terminal};
use trk_core::{Cursor, Song};

use super::*;

#[test]
fn renders_ai_chat_view_with_all_message_roles() {
    let messages = [
        AiChatMessageView {
            role: AiChatMessageRole::System,
            text: "system ready",
        },
        AiChatMessageView {
            role: AiChatMessageRole::User,
            text: "make a bassline",
        },
        AiChatMessageView {
            role: AiChatMessageRole::Assistant,
            text: "proposal touches p01/r00/t01",
        },
        AiChatMessageView {
            role: AiChatMessageRole::Error,
            text: "missing token",
        },
        AiChatMessageView {
            role: AiChatMessageRole::Progress,
            text: "Task #1 running",
        },
    ];
    let proposal_preview = vec![
        "Pending: Local deterministic pattern sketch".to_string(),
        "Touches 2 cell(s): p01/r00/t01, p01/r04/t01".to_string(),
        "Actions: a apply | r reject | p preview | Ctrl+Z undo after apply".to_string(),
    ];
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &Song::empty(),
                TuiState {
                    cursor: Cursor::new(),
                    row_offset: 0,
                    track_offset: 0,
                    pattern_index: 0,
                    active_view: TuiView::AiChat,
                    selection: None,
                    mode_label: "AI",
                    octave: 4,
                    edit_step: 1,
                    dirty: false,
                    show_line_numbers_hex: false,
                    row_number_offset: 0,
                    pattern_divider_interval: 4,
                    pattern_highlight_interval: 16,
                    show_pattern_top_info: false,
                    command_line: None,
                    notification: None,
                    show_help: false,
                    help_scroll: 0,
                    help_tab: HelpTab::Basics,
                    is_playing: false,
                    loop_pattern: true,
                    playhead_row: None,
                    midi_status: "MIDI Disconnected",
                    sequence_position: None,
                    quit_confirmation: false,
                    delete_confirmation: None,
                    midi_settings: None,
                    command_palette: None,
                    sampler_view: None,
                    dsp_rack: None,
                    sample_browser: None,
                    project_browser: None,
                    ai_chat: Some(AiChatViewState {
                        provider: "mock model=fixture",
                        status: "available",
                        composer: "draft prompt",
                        messages: &messages,
                        selected_context: "Context: pattern 01, track 01, row 00",
                        proposal_preview: Some(AiChatProposalPreviewView {
                            lines: &proposal_preview,
                        }),
                        engine_selector: None,
                    }),
                    tracker_layout: crate::TrackerLayoutState::default(),
                },
            );
        })
        .expect("draw");

    let rendered = buffer_text(terminal.backend().buffer());
    assert!(rendered.contains("AI Chat"));
    assert!(rendered.contains("system:"));
    assert!(rendered.contains("user:"));
    assert!(rendered.contains("assistant:"));
    assert!(rendered.contains("error:"));
    assert!(rendered.contains("progress:"));
    assert!(rendered.contains("Selected Proposal"));
    assert!(rendered.contains("p01/r00/t01"));
    assert!(rendered.contains("a apply"));
    assert!(rendered.contains("draft prompt"));
}

#[test]
fn renders_double_bordered_engine_selector_with_availability_and_active_badge() {
    let entries = [
        AiEngineEntryView {
            label: "Built-in",
            model: "local-deterministic",
            available: true,
            active: true,
            unavailable_reason: None,
        },
        AiEngineEntryView {
            label: "Claude CLI",
            model: "default",
            available: false,
            active: false,
            unavailable_reason: Some("missing claude executable in PATH"),
        },
    ];
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).expect("terminal");

    terminal
        .draw(|frame| {
            render(
                frame,
                &Song::empty(),
                TuiState {
                    active_view: TuiView::AiChat,
                    mode_label: "AI",
                    ai_chat: Some(AiChatViewState {
                        provider: "Built-in model=local-deterministic",
                        status: "available",
                        composer: "",
                        messages: &[],
                        selected_context: "Context: pattern 01, track 01, row 00",
                        proposal_preview: None,
                        engine_selector: Some(AiEngineSelectorViewState {
                            entries: &entries,
                            selected: 1,
                        }),
                    }),
                    ..super::render_test_support::render_test_state()
                },
            );
        })
        .expect("draw");

    let rendered = buffer_text(terminal.backend().buffer());
    assert!(rendered.contains("AI Engines"));
    assert!(rendered.contains("* Built-in"));
    assert!(rendered.contains("[OK] Available"));
    assert!(rendered.contains("missing claude executable in PATH"));
    assert!(rendered.contains('╔'));
    assert!(rendered.contains('╝'));
}

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }
    output
}
