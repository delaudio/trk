use ratatui::{
    layout::Rect,
    prelude::{Frame, Line, Span, Style},
    widgets::{Block, Borders, Paragraph},
};
use trk_core::{NoteEvent, Pattern, Song};

use super::{theme, TuiState, TuiView};

pub(super) fn render_piano_roll(
    frame: &mut Frame<'_>,
    area: Rect,
    song: &Song,
    state: TuiState<'_>,
) {
    let TuiView::PianoRoll {
        pitch,
        rows,
        ghosts,
    } = state.active_view
    else {
        return;
    };
    let Some(pattern) = song.pattern(state.pattern_index) else {
        return;
    };
    let title = format!(
        " Piano Roll · Track {:02} · {} rows · ghosts {} ",
        state.cursor.track + 1,
        rows,
        if ghosts { "on" } else { "off" }
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width < 8 || inner.height == 0 {
        return;
    }

    let visible_rows = usize::from(rows).min(pattern.row_count()).max(1);
    let max_start = pattern.row_count().saturating_sub(visible_rows);
    let row_start = state
        .cursor
        .row
        .saturating_sub(visible_rows / 2)
        .min(max_start);
    let pitch_rows = usize::from(inner.height).min(128);
    let pitch_bottom = usize::from(pitch)
        .saturating_sub(pitch_rows / 2)
        .min(128_usize.saturating_sub(pitch_rows));
    let pitch_top = pitch_bottom.saturating_add(pitch_rows.saturating_sub(1));
    let cell_width = usize::from(inner.width.saturating_sub(5))
        .checked_div(visible_rows)
        .unwrap_or(1)
        .max(1);
    let mut lines = Vec::with_capacity(pitch_rows);
    for line_index in 0..pitch_rows {
        let line_pitch = pitch_top.saturating_sub(line_index) as u8;
        let pitch_class = line_pitch % 12;
        let key_style = if matches!(pitch_class, 1 | 3 | 6 | 8 | 10) {
            Style::default().fg(theme::MUTED).bg(theme::BORDER_DIM)
        } else if pitch_class == 0 {
            theme::label()
        } else {
            theme::base()
        };
        let mut spans = vec![Span::styled(
            format!("{:>4} ", note_name(line_pitch)),
            key_style,
        )];
        for visible_row in 0..visible_rows {
            let row = row_start + visible_row;
            let active = note_segment_velocity(pattern, state.cursor.track, row, line_pitch);
            let ghost = ghosts
                && active.is_none()
                && (0..song.tracks.len()).any(|track| {
                    track != state.cursor.track
                        && note_segment_velocity(pattern, track, row, line_pitch).is_some()
                });
            let cursor = row == state.cursor.row && line_pitch == pitch;
            let playhead = state.playhead_row == Some(row);
            let (glyph, style) = if cursor {
                ("◆", theme::active())
            } else if let Some(velocity) = active {
                let intensity = 96_u8.saturating_add(velocity.min(127));
                (
                    "━",
                    Style::default().fg(ratatui::prelude::Color::Rgb(intensity, 96, 255)),
                )
            } else if ghost {
                ("·", theme::muted())
            } else if playhead {
                ("│", theme::playing())
            } else {
                (" ", Style::default())
            };
            spans.push(Span::styled(
                format!("{glyph:<width$}", width = cell_width),
                style,
            ));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn note_segment_velocity(pattern: &Pattern, track: usize, row: usize, pitch: u8) -> Option<u8> {
    (0..=row).rev().find_map(|start| {
        let cell = pattern.cell(start, track)?;
        (matches!(cell.note, Some(NoteEvent::Note { pitch: candidate }) if candidate == pitch)
            && pattern
                .note_gate_rows(start, track)
                .is_some_and(|gate| row < start.saturating_add(gate)))
        .then_some(cell.velocity.unwrap_or(127))
    })
}

fn note_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
    ];
    format!(
        "{}{}",
        NAMES[usize::from(pitch % 12)],
        i16::from(pitch / 12) - 1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_test_support::render_test_state;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn piano_roll_renders_bounded_active_ghost_and_cursor_cells() {
        let mut song = Song::empty();
        song.patterns[0]
            .set_note(4, 0, NoteEvent::Note { pitch: 60 }, 100)
            .expect("active note");
        song.patterns[0].set_gate(4, 0, Some(3)).expect("gate");
        song.patterns[0]
            .set_note(5, 1, NoteEvent::Note { pitch: 64 }, 64)
            .expect("ghost note");
        for (width, height) in [(7, 2), (20, 4), (100, 28)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
            terminal
                .draw(|frame| {
                    render_piano_roll(
                        frame,
                        frame.area(),
                        &song,
                        TuiState {
                            cursor: trk_core::Cursor {
                                row: 4,
                                track: 0,
                                ..trk_core::Cursor::new()
                            },
                            active_view: TuiView::PianoRoll {
                                pitch: 60,
                                rows: 16,
                                ghosts: true,
                            },
                            ..render_test_state()
                        },
                    );
                })
                .expect("render");
            if width == 100 {
                let symbols = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                assert!(symbols.contains("Piano Roll"));
                assert!(symbols.contains('◆'));
                assert!(symbols.contains('·'));
            }
        }
    }
}
