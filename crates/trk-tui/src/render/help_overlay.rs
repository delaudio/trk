use ratatui::{
    layout::{Alignment, Rect},
    prelude::{Color, Frame, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

use crate::{interaction_region, InteractionMap, InteractionPayload, ViewportAxis};

use super::HelpTab;

pub(super) fn render_help_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    mode_label: &str,
    edit_step: usize,
    scroll: usize,
    tab: HelpTab,
    interactions: &mut InteractionMap,
) -> Rect {
    let overlay = large_overlay_rect(area);
    interactions.register(interaction_region::OVERLAY_HELP, overlay);
    let inner = Rect::new(
        overlay.x.saturating_add(1),
        overlay.y.saturating_add(1),
        overlay.width.saturating_sub(2),
        overlay.height.saturating_sub(2),
    );
    let controls_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let hint_area = Rect::new(
        inner.x,
        inner.y.saturating_add(1),
        inner.width,
        u16::from(inner.height > 1),
    );
    let content_area = Rect::new(
        inner.x,
        inner.y.saturating_add(2),
        inner.width,
        inner.height.saturating_sub(2),
    );
    interactions.register(interaction_region::HELP_CONTENT, content_area);

    let lines = help_content_lines(mode_label, edit_step, tab);
    let visible_rows = content_area.height as usize;
    let line_count = lines.len();
    let mut viewport = ViewportAxis::with_offset(lines.len(), visible_rows, scroll);
    viewport.clamp();
    let max_scroll = viewport.max_offset();
    let scroll = viewport.offset();
    let title = if max_scroll == 0 {
        format!(" Help: {} ", tab.label())
    } else {
        format!(" Help: {} {}/{} ", tab.label(), scroll + 1, max_scroll + 1)
    };

    frame.render_widget(Clear, overlay);
    frame.render_widget(Block::default().title(title).borders(Borders::ALL), overlay);
    render_help_controls(frame, controls_area, tab, interactions);
    frame.render_widget(
        Paragraph::new(" Tab/Right next page   Shift+Tab/Left previous page   Up/Down scroll")
            .style(Style::default().fg(Color::DarkGray)),
        hint_area,
    );
    let paragraph = Paragraph::new(lines)
        .scroll((scroll as u16, 0))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, content_area);
    if line_count > visible_rows {
        let mut scrollbar_state = viewport.scrollbar_state();
        frame.render_stateful_widget(
            Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
            content_area,
            &mut scrollbar_state,
        );
    }
    overlay
}

fn render_help_controls(
    frame: &mut Frame<'_>,
    area: Rect,
    active: HelpTab,
    interactions: &mut InteractionMap,
) {
    const CLOSE_LABEL: &str = "[ Close ]";
    let close_width = (CLOSE_LABEL.len() as u16).min(area.width);
    let close_area = Rect::new(
        area.x
            .saturating_add(area.width.saturating_sub(close_width)),
        area.y,
        close_width,
        area.height,
    );
    let tab_width = area
        .width
        .saturating_sub(close_width)
        .saturating_sub(u16::from(area.width > close_width));
    let tabs_area = Rect::new(area.x, area.y, tab_width, area.height);
    let mut spans = Vec::new();
    let mut cursor_x = tabs_area.x;
    let tabs_right = tabs_area.x.saturating_add(tabs_area.width);
    for (index, tab) in HelpTab::ALL.iter().copied().enumerate() {
        if index > 0 {
            const SEPARATOR: &str = " | ";
            spans.push(Span::raw(SEPARATOR));
            cursor_x = cursor_x.saturating_add(SEPARATOR.len() as u16);
        }
        let label = format!(" {} ", tab.label());
        let visible_width = (label.len() as u16).min(tabs_right.saturating_sub(cursor_x));
        interactions.register_with_payload(
            interaction_region::HELP_TAB,
            Rect::new(cursor_x, area.y, visible_width, area.height),
            InteractionPayload::HelpTab { index },
        );
        let style = if tab == active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(label, style));
        cursor_x = cursor_x.saturating_add(visible_width);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), tabs_area);
    interactions.register(interaction_region::HELP_CLOSE, close_area);
    frame.render_widget(
        Paragraph::new(CLOSE_LABEL)
            .alignment(Alignment::Right)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        close_area,
    );
}

fn help_content_lines(mode_label: &str, edit_step: usize, tab: HelpTab) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match tab {
        HelpTab::Basics => lines.extend(help_basics_lines(mode_label)),
        HelpTab::Editing => lines.extend(help_editing_lines(mode_label, edit_step)),
        HelpTab::Sampler => lines.extend(help_sampler_lines(mode_label)),
        HelpTab::Midi => lines.extend(help_midi_lines(mode_label)),
        HelpTab::Commands => lines.extend(help_command_lines(mode_label)),
    }

    lines
}

fn large_overlay_rect(area: Rect) -> Rect {
    let horizontal_margin = if area.width >= 120 { 6 } else { 2 };
    let vertical_margin = if area.height >= 32 { 3 } else { 1 };
    let width = area.width.saturating_sub(horizontal_margin * 2).max(20);
    let height = area.height.saturating_sub(vertical_margin * 2).max(8);
    Rect {
        x: area.x + horizontal_margin.min(area.width.saturating_sub(1)),
        y: area.y + vertical_margin.min(area.height.saturating_sub(1)),
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn help_basics_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Global",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(
            "  Ctrl+P Command Palette   ?/H Help   :h/:help Help   q Quit   Space Play/Stop",
        ),
        Line::from("  Shift+Space Start"),
        Line::from("  Enter Play Row   Shift+Enter Play Sequence From Cursor   L Loop   F8 Stop"),
        Line::from("  F7 Sequence View   F9 Track View   F10 Pattern View   Ctrl+J Sampler View"),
        Line::from("  :t Tracker   :p Patterns   :se Sequence   :tr Tracks   :sa Sampler   :sb Browser"),
        Line::from("  Esc returns from focused views"),
        Line::from("  :play pattern from start   :play sequence arrangement"),
        Line::from("  Ctrl+S Save   Ctrl+Shift+S Save As   Ctrl+Z Undo   Ctrl+Y Redo   Ctrl+Arrows BPM/LPB"),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Arrows or h/j/k/l move   Tab/Shift+Tab track   PageUp/PageDown jump"),
        Line::from("  Home/End pattern bounds   gg first row   G last row"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_editing_lines(mode_label: &str, edit_step: usize) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Editing",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  i Edit   Esc Normal   Del/Backspace clear cell   Ctrl+C/X/V cell clipboard"),
        Line::from("  V select region   Esc cancel selection   Delete clears selection"),
        Line::from("  Insert row   Ctrl+Delete delete row   F1/- octave down"),
        Line::from("  F2/+/= octave up   VEL/INST/VOL/PAN/DLY/FX accept two hex digits"),
        Line::from(format!(
            "  Step jump advances manual entry by {edit_step} row(s); 0 keeps the cursor in place"
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Patterns And Sequence",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(
            "  N new pattern   P duplicate pattern   X delete pattern   F3 rename   F6 length",
        ),
        Line::from("  :pattern fill/copy/paste/invert/expand/shrink/duplicate-selection"),
        Line::from("  Pattern view: 1/2/3/4/5 set length 16/32/64/128/256"),
        Line::from("  A add current pattern to sequence   ,/. move sequence cursor"),
        Line::from("  Y duplicate sequence position   R remove   T set to current pattern"),
        Line::from("  </> move selected sequence position up/down"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_sampler_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Sampler And Instruments",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Ctrl+J opens Sampler view   Esc returns to Pattern view"),
        Line::from("  In Sampler view: +/- zoom waveform   Left/Right pan   Home/End bounds"),
        Line::from("  Tab/Shift+Tab selects A/D/S/R   [/]/{/} adjusts selected envelope field"),
        Line::from("  :sample view PATH loads a WAV and shows metadata plus waveform"),
        Line::from("  :sample browse [DIR] opens the in-app sample browser"),
        Line::from(
            "  In Sample Browser: A assigns the selected sample; right-click assigns with mouse",
        ),
        Line::from("  :sample choose [DIR] opens the configured external chooser"),
        Line::from(""),
        Line::from(Span::styled(
            "Track Assignment",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  :sample assign [TRACK] assigns the loaded sample to a track"),
        Line::from("  :sample replace [TRACK] swaps the track sample and prunes the old reference"),
        Line::from("  :sample unassign [TRACK] clears the track sample and instrument assignment"),
        Line::from("  TRACK is 1-based; omitted TRACK means the current track"),
        Line::from("  :sample assignments lists track=sample mappings"),
        Line::from(""),
        Line::from(Span::styled(
            "Instrument Column",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Assigning a sample creates a sample-backed instrument for that track"),
        Line::from("  Cells can override the track default with INST, e.g. :cell instrument 01"),
        Line::from("  An empty INST field uses the track instrument or sample assignment"),
        Line::from("  :preset instrument save|show|load PATH for portable instrument files"),
        Line::from(""),
        Line::from(Span::styled(
            "Playback Window",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  :sample start FRAME|clear   :sample end FRAME|clear"),
        Line::from("  :sample loop [backward|pingpong] START END   :sample loop off"),
        Line::from("  :sample mode one-shot|forward-loop|backward-loop|pingpong-loop|reverse"),
        Line::from("  :sample envelope ATTACK DECAY SUSTAIN RELEASE"),
        Line::from("  :sample settings shows mode, frame window, loop and envelope"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_midi_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "MIDI",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  F4 or :midi outputs opens MIDI settings and lists output ports"),
        Line::from("  In MIDI settings: arrows select, Enter connects, F5/r refresh, p panic"),
        Line::from("  CLI fallback: trk --list-midi-outputs, then :midi connect 0"),
        Line::from("  Input: trk --list-midi-inputs, then :midi-input connect 0"),
        Line::from("  :midi-input record on captures note-on events into the current pattern"),
        Line::from("  :midi-input clock/transport/notes/cc in|out on|off routes each lane"),
        Line::from("  :midi-input channel in|out all|1,10 filters by MIDI channel"),
        Line::from("  :midi-input middle-c NOTE and sync-delay MS calibrate incoming timing"),
        Line::from("  Press Space or run :play pattern to send notes to the connected output"),
        Line::from("  :midi disconnect closes the output   :midi panic sends All Notes Off"),
        Line::from("  Use :track channel 2 10 to set track 02 to MIDI channel 10"),
        Line::from("  Config: [midi] default_output/default_input auto-connect by name"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}

fn help_command_lines(mode_label: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "Tracks And Commands",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Ctrl+T create track   D duplicate track   {/} move track left/right"),
        Line::from("  t live DSP calibration   arrows adjust   r reset   t/Esc close"),
        Line::from("  e edit project in $EDITOR and reload valid changes on exit"),
        Line::from("  r rename track   c channel"),
        Line::from("  Del delete track   M mute   S solo"),
        Line::from("  :write [path]   :saveas path   :quit   :q!   :wq   :bpm 140   :lpb 4"),
        Line::from(
            "  Panel focus: :t tracker   :p patterns   :se sequence   :tr tracks   :sa sampler",
        ),
        Line::from("  :layout compact|balanced|studio   :layout toggle inspector"),
        Line::from(
            "  :layout fields full|note|instrument|fx|note-instrument|note-fx|instrument-fx",
        ),
        Line::from("  Dirty quit asks: [Y]es save, [N]o quit, [C]ancel"),
        Line::from("  :track new   :track duplicate 2   :track delete 2   :track move 2 3"),
        Line::from("  :track mute 2   :track solo 2   :track rename Acid Bass"),
        Line::from("  Tracker FX columns edit per-cell commands, not audio device chains:"),
        Line::from("  :fx D 20 delay   :fx R 04 retrigger   :fx2 R 02 second FX column"),
        Line::from("  :fx clear   :fx2 clear"),
        Line::from("  :cell instrument 01   :cell volume 40   :cell pan 7F   :cell delay 20"),
        Line::from("  :cell effect R 04   :cell FIELD clear"),
        Line::from("  :mixer send delay|reverb   :mixer send SEND gain [TRACK] GAIN"),
        Line::from("  Native DSP chains process audio on tracks/master:"),
        Line::from(
            "  :dsp track 2 filter lowpass 2000 0.25 0 0.5   :dsp master reverb 0.5 20 2.5 0.25",
        ),
        Line::from("  :dsp track 2 clear"),
        Line::from("  Row locks change one parameter on the current tracker row:"),
        Line::from(
            "  :plock dsp track filter-cutoff 1200   :plock dsp track filter-cutoff reset|clear",
        ),
        Line::from("  :ai propose PROMPT   :ai show   :ai accept   :ai reject"),
        Line::from("  :play pattern   :play sequence [position]   :stop"),
        Line::from("  :tasks   :task cancel ID"),
        Line::from("  :pattern new   :pattern duplicate   :pattern delete   :pattern length 128"),
        Line::from("  :pattern fill|copy|paste|invert|expand|shrink|duplicate-selection"),
        Line::from("  :pattern rename Intro   :pattern 1   [ previous pattern   ] next pattern"),
        Line::from("  :sequence add   :sequence remove 0   :sequence duplicate 0"),
        Line::from("  :sequence set 0 2   :sequence move 1 0"),
        Line::from(""),
        Line::from(format!("Mode: {mode_label}   Close: Esc, q, or ?")),
    ]
}
