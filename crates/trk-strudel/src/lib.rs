mod evaluate;
mod parse;

pub use evaluate::{apply_to_pattern, evaluate, Evaluation, PatternWrite};
pub use parse::{Expr, Program, Scale, SourceKind, StrudelError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluateOptions {
    pub rows: usize,
    pub cycle: usize,
    pub start_track: usize,
    pub available_tracks: usize,
    pub velocity: u8,
}

impl EvaluateOptions {
    #[must_use]
    pub fn for_pattern(rows: usize, start_track: usize, available_tracks: usize) -> Self {
        Self {
            rows,
            cycle: 0,
            start_track,
            available_tracks,
            velocity: 100,
        }
    }
}
