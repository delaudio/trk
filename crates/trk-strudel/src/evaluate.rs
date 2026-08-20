use std::collections::BTreeMap;

use trk_core::{InstrumentId, NoteEvent, Pattern, PatternCell};

use crate::{parse::pitch_class, EvaluateOptions, Expr, Program, Scale, SourceKind, StrudelError};

#[derive(Debug, Clone, PartialEq)]
pub struct PatternWrite {
    pub row: usize,
    pub track_offset: usize,
    pub cell: PatternCell,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub writes: Vec<PatternWrite>,
    pub track_count: usize,
}

pub fn evaluate(program: &Program, options: EvaluateOptions) -> Result<Evaluation, StrudelError> {
    if options.rows == 0 {
        return Err(StrudelError::evaluation("row count must be positive"));
    }
    let track_count = layer_width(&program.expression);
    if track_count > options.available_tracks {
        return Err(StrudelError::evaluation(format!(
            "pattern needs {track_count} tracks but only {} are available",
            options.available_tracks
        )));
    }
    let mut events = Vec::new();
    let mut expansion_budget = options
        .rows
        .saturating_mul(options.available_tracks)
        .saturating_mul(16)
        .clamp(64, 1_000_000);
    render(
        &program.expression,
        Span::new(0.0, options.rows as f64),
        0,
        program,
        options,
        &mut expansion_budget,
        &mut events,
    )?;
    let mut cells = BTreeMap::new();
    for event in events {
        if event.row >= options.rows {
            continue;
        }
        let key = (event.row, event.track_offset);
        if cells.insert(key, event.cell).is_some() {
            return Err(StrudelError::evaluation(format!(
                "multiple events resolve to row {} on layer {}; increase pattern rows",
                event.row,
                event.track_offset + 1
            )));
        }
    }
    Ok(Evaluation {
        writes: cells
            .into_iter()
            .map(|((row, track_offset), cell)| PatternWrite {
                row,
                track_offset,
                cell,
            })
            .collect(),
        track_count,
    })
}

pub fn apply_to_pattern(
    pattern: &mut Pattern,
    program: &Program,
    options: EvaluateOptions,
) -> Result<Evaluation, StrudelError> {
    if options.rows != pattern.row_count() {
        return Err(StrudelError::evaluation(
            "evaluation rows must match the target pattern",
        ));
    }
    let evaluation = evaluate(program, options)?;
    let mut staged = pattern.clone();
    for row in 0..options.rows {
        for offset in 0..evaluation.track_count {
            let track = options.start_track + offset;
            let Some(cell) = staged.cell_mut(row, track) else {
                return Err(StrudelError::evaluation("target cell is out of bounds"));
            };
            *cell = PatternCell::default();
        }
    }
    for write in &evaluation.writes {
        let track = options.start_track + write.track_offset;
        let Some(cell) = staged.cell_mut(write.row, track) else {
            return Err(StrudelError::evaluation("target cell is out of bounds"));
        };
        *cell = write.cell.clone();
    }
    *pattern = staged;
    Ok(evaluation)
}

#[derive(Debug, Clone, Copy)]
struct Span {
    start: f64,
    length: f64,
}

impl Span {
    fn new(start: f64, length: f64) -> Self {
        Self { start, length }
    }

    fn part(self, index: usize, count: usize) -> Self {
        let length = self.length / count as f64;
        Self::new(self.start + length * index as f64, length)
    }
}

fn render(
    expression: &Expr,
    span: Span,
    track_offset: usize,
    program: &Program,
    options: EvaluateOptions,
    expansion_budget: &mut usize,
    output: &mut Vec<PatternWrite>,
) -> Result<(), StrudelError> {
    let Some(remaining) = expansion_budget.checked_sub(1) else {
        return Err(StrudelError::evaluation(
            "expression expansion exceeds the target grid budget",
        ));
    };
    *expansion_budget = remaining;
    match expression {
        Expr::Rest => {}
        Expr::Atom(token) => {
            let row = quantize_start(span.start);
            let pitch = token_pitch(token, program.source, program.scale.as_ref())?;
            let remaining_rows = options.rows.saturating_sub(row).clamp(1, 127) as u8;
            let gate = quantize_gate(span.length).min(remaining_rows);
            output.push(PatternWrite {
                row,
                track_offset,
                cell: PatternCell {
                    note: Some(NoteEvent::Note { pitch }),
                    velocity: Some(options.velocity.min(127)),
                    instrument: token_instrument(token)?,
                    gate: Some(gate),
                    ..PatternCell::default()
                },
            });
        }
        Expr::Sequence(values) => {
            for (index, value) in values.iter().enumerate() {
                render(
                    value,
                    span.part(index, values.len()),
                    track_offset,
                    program,
                    options,
                    expansion_budget,
                    output,
                )?;
            }
        }
        Expr::Layer(values) => {
            let mut offset = track_offset;
            for value in values {
                render(
                    value,
                    span,
                    offset,
                    program,
                    options,
                    expansion_budget,
                    output,
                )?;
                offset += layer_width(value);
            }
        }
        Expr::Alternation(values) => {
            let value = &values[options.cycle % values.len()];
            render(
                value,
                span,
                track_offset,
                program,
                options,
                expansion_budget,
                output,
            )?;
        }
        Expr::Fast(value, factor) => {
            for index in 0..*factor {
                render(
                    value,
                    span.part(index, *factor),
                    track_offset,
                    program,
                    options,
                    expansion_budget,
                    output,
                )?;
            }
        }
        Expr::Slow(value, factor) => render(
            value,
            Span::new(span.start, span.length * *factor as f64),
            track_offset,
            program,
            options,
            expansion_budget,
            output,
        )?,
        Expr::Euclid {
            expression,
            pulses,
            steps,
            rotation,
        } => {
            for (index, active) in euclidean_pattern(*steps, *pulses, *rotation)
                .into_iter()
                .enumerate()
            {
                if active {
                    render(
                        expression,
                        span.part(index, *steps),
                        track_offset,
                        program,
                        options,
                        expansion_budget,
                        output,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn quantize_start(value: f64) -> usize {
    let nearest = value.round();
    let normalized = if (value - nearest).abs() <= 1e-9 {
        nearest
    } else {
        value.floor()
    };
    normalized.max(0.0) as usize
}

fn quantize_gate(value: f64) -> u8 {
    let nearest = value.round();
    let normalized = if (value - nearest).abs() <= 1e-9 {
        nearest
    } else {
        value.ceil()
    };
    normalized.clamp(1.0, 127.0) as u8
}

fn layer_width(expression: &Expr) -> usize {
    match expression {
        Expr::Layer(values) => values.iter().map(layer_width).sum(),
        Expr::Sequence(values) => values.iter().map(layer_width).max().unwrap_or(1),
        Expr::Alternation(values) => values.iter().map(layer_width).max().unwrap_or(1),
        Expr::Fast(value, _) | Expr::Slow(value, _) => layer_width(value),
        Expr::Euclid { expression, .. } => layer_width(expression),
        Expr::Atom(_) | Expr::Rest => 1,
    }
}

fn token_pitch(token: &str, source: SourceKind, scale: Option<&Scale>) -> Result<u8, StrudelError> {
    let token = token.split('@').next().unwrap_or(token);
    if source == SourceKind::Samples {
        return match token.to_ascii_lowercase().as_str() {
            "bd" | "kick" => Ok(36),
            "sd" | "sn" | "snare" => Ok(38),
            "cp" | "clap" => Ok(39),
            "hh" | "ch" => Ok(42),
            "oh" => Ok(46),
            _ => Err(StrudelError::evaluation(format!(
                "unknown sample token '{token}'"
            ))),
        };
    }
    if let Ok(degree) = token.parse::<i16>() {
        let scale = scale.ok_or_else(|| {
            StrudelError::evaluation(format!("scale degree '{token}' requires .scale()"))
        })?;
        return scale_pitch(degree, scale);
    }
    note_pitch(token)
}

fn token_instrument(token: &str) -> Result<Option<InstrumentId>, StrudelError> {
    let Some((_, value)) = token.rsplit_once('@') else {
        return Ok(None);
    };
    let value = value
        .parse::<u32>()
        .map_err(|_| StrudelError::evaluation("instrument suffix must be @NUMBER"))?;
    Ok(Some(InstrumentId(value)))
}

fn note_pitch(token: &str) -> Result<u8, StrudelError> {
    let octave_start = token
        .char_indices()
        .find(|(_, value)| value.is_ascii_digit() || *value == '-')
        .map(|(index, _)| index)
        .ok_or_else(|| StrudelError::evaluation(format!("note '{token}' needs an octave")))?;
    let class = pitch_class(&token[..octave_start])
        .ok_or_else(|| StrudelError::evaluation(format!("unknown note '{token}'")))?;
    let octave = token[octave_start..]
        .parse::<i16>()
        .map_err(|_| StrudelError::evaluation(format!("invalid octave in '{token}'")))?;
    let pitch = (octave + 1) * 12 + i16::from(class);
    u8::try_from(pitch)
        .ok()
        .filter(|value| *value <= 127)
        .ok_or_else(|| StrudelError::evaluation(format!("note '{token}' is outside MIDI range")))
}

fn scale_pitch(degree: i16, scale: &Scale) -> Result<u8, StrudelError> {
    let width = scale.intervals.len() as i16;
    let octave = degree.div_euclid(width);
    let index = degree.rem_euclid(width) as usize;
    let pitch = 60 + i16::from(scale.root) + octave * 12 + i16::from(scale.intervals[index]);
    u8::try_from(pitch)
        .ok()
        .filter(|value| *value <= 127)
        .ok_or_else(|| StrudelError::evaluation("scale degree is outside MIDI range"))
}

fn euclidean_pattern(steps: usize, pulses: usize, rotation: usize) -> Vec<bool> {
    let mut pattern = vec![false; steps];
    for step in 0..steps {
        let previous = step.saturating_mul(pulses) / steps;
        let current = (step + 1).saturating_mul(pulses) / steps;
        pattern[(step + rotation) % steps] = current > previous;
    }
    pattern
}

#[cfg(test)]
mod tests {
    use trk_core::{Pattern, PatternId};

    use super::*;

    fn rows(source: &str, row_count: usize, cycle: usize) -> Vec<(usize, usize, u8, u8)> {
        let program = Program::parse(source).expect("parse");
        evaluate(
            &program,
            EvaluateOptions {
                rows: row_count,
                cycle,
                start_track: 0,
                available_tracks: 8,
                velocity: 96,
            },
        )
        .expect("evaluate")
        .writes
        .into_iter()
        .map(|write| {
            let Some(NoteEvent::Note { pitch }) = write.cell.note else {
                panic!("expected note")
            };
            (
                write.row,
                write.track_offset,
                pitch,
                write.cell.gate.unwrap(),
            )
        })
        .collect()
    }

    #[test]
    fn evaluates_subdivision_fast_slow_rest_and_layers() {
        assert_eq!(
            rows("[c4 d4] ~ e4*2", 12, 0),
            vec![(0, 0, 60, 2), (2, 0, 62, 2), (8, 0, 64, 2), (10, 0, 64, 2)]
        );
        assert_eq!(rows("[c4/2, e4]", 8, 0), vec![(0, 0, 60, 8), (0, 1, 64, 8)]);
        assert_eq!(rows("~ c4/2", 8, 0), vec![(4, 0, 60, 4)]);
    }

    #[test]
    fn evaluates_layers_alternation_euclid_and_scale() {
        assert_eq!(rows("[c4, e4]", 8, 0), vec![(0, 0, 60, 8), (0, 1, 64, 8)]);
        assert_eq!(rows("<c4 d4>", 8, 1), vec![(0, 0, 62, 8)]);
        let uneven = Program::parse("<[c4, e4] g4>").expect("parse uneven alternation");
        assert!(evaluate(
            &uneven,
            EvaluateOptions {
                cycle: 1,
                ..EvaluateOptions::for_pattern(8, 0, 1)
            }
        )
        .is_err());
        assert_eq!(
            rows("note(\"0\").euclid(3,8).scale(\"d:minor\")", 8, 0),
            vec![(2, 0, 62, 1), (5, 0, 62, 1), (7, 0, 62, 1)]
        );
    }

    #[test]
    fn maps_note_names_scale_degrees_and_drum_tokens() {
        assert_eq!(
            rows("note(\"0 1 7\").scale(\"d:dorian\")", 12, 0),
            vec![(0, 0, 62, 4), (4, 0, 64, 4), (8, 0, 74, 4)]
        );
        assert_eq!(
            rows("s(\"bd sd cp hh\")", 8, 0)
                .iter()
                .map(|row| row.2)
                .collect::<Vec<_>>(),
            vec![36, 38, 39, 42]
        );
        assert!(Program::parse("note(\"0\").scale(\"h:minor\")").is_err());
        assert!(evaluate(
            &Program::parse("130").expect("parse"),
            EvaluateOptions::for_pattern(8, 0, 1)
        )
        .is_err());
    }

    #[test]
    fn applies_atomically_and_clears_only_addressed_layers() {
        let mut pattern = Pattern::empty(PatternId(1), "Pattern", 8, 3);
        pattern.cell_mut(0, 2).unwrap().note = Some(NoteEvent::Note { pitch: 72 });
        let program = Program::parse("[c4, e4@3]").expect("parse");
        let result = apply_to_pattern(
            &mut pattern,
            &program,
            EvaluateOptions::for_pattern(8, 0, 3),
        )
        .expect("apply");
        assert_eq!(result.track_count, 2);
        assert_eq!(
            pattern.cell(0, 1).unwrap().instrument,
            Some(InstrumentId(3))
        );
        assert_eq!(
            pattern.cell(0, 2).unwrap().note,
            Some(NoteEvent::Note { pitch: 72 })
        );

        let before = pattern.clone();
        let too_wide = Program::parse("[c4, e4, g4, b4]").expect("parse");
        assert!(apply_to_pattern(
            &mut pattern,
            &too_wide,
            EvaluateOptions::for_pattern(8, 0, 3),
        )
        .is_err());
        assert_eq!(pattern, before);
    }

    #[test]
    fn rejects_sub_row_collisions() {
        let program = Program::parse("c4*16").expect("parse");
        assert!(evaluate(&program, EvaluateOptions::for_pattern(8, 0, 1)).is_err());
    }

    #[test]
    fn rejects_chained_expansion_beyond_the_grid_budget() {
        let program = Program::parse("~*1024*1024").expect("parse");
        let error = evaluate(&program, EvaluateOptions::for_pattern(64, 0, 1))
            .expect_err("bounded expansion");
        assert!(error.message.contains("expansion"));
    }

    #[test]
    fn nested_fractional_boundaries_quantize_stably() {
        assert_eq!(
            rows("[[c4 d4 e4] [f4 g4 a4] [b4 c5 d5]]", 9, 0)
                .into_iter()
                .map(|event| event.0)
                .collect::<Vec<_>>(),
            (0..9).collect::<Vec<_>>()
        );
    }
}
