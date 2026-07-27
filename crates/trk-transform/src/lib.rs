use trk_core::{NoteEvent, Song};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EuclideanRhythm {
    pub steps: usize,
    pub pulses: usize,
    pub rotation: usize,
    pub track: usize,
    pub pitch: u8,
    pub velocity: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformReport {
    pub touched_cells: Vec<(usize, usize)>,
}

#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("pattern {0} does not exist")]
    MissingPattern(usize),
    #[error("track {0} does not exist")]
    MissingTrack(usize),
    #[error("steps must be greater than zero")]
    EmptySteps,
    #[error("pulses cannot exceed steps")]
    TooManyPulses,
}

pub fn euclidean_pattern(steps: usize, pulses: usize, rotation: usize) -> Vec<bool> {
    if steps == 0 {
        return Vec::new();
    }
    let pulses = pulses.min(steps);
    let mut pattern = vec![false; steps];
    for step in 0..steps {
        let previous = step.saturating_mul(pulses) / steps;
        let current = (step + 1).saturating_mul(pulses) / steps;
        pattern[(step + rotation) % steps] = current > previous;
    }
    pattern
}

pub fn apply_euclidean(
    song: &mut Song,
    pattern_index: usize,
    rhythm: EuclideanRhythm,
) -> Result<TransformReport, TransformError> {
    if rhythm.steps == 0 {
        return Err(TransformError::EmptySteps);
    }
    if rhythm.pulses > rhythm.steps {
        return Err(TransformError::TooManyPulses);
    }
    if rhythm.track >= song.tracks.len() {
        return Err(TransformError::MissingTrack(rhythm.track));
    }
    let pattern = song
        .pattern_mut(pattern_index)
        .ok_or(TransformError::MissingPattern(pattern_index))?;
    let row_count = pattern.row_count();
    let rhythm_pattern = euclidean_pattern(rhythm.steps, rhythm.pulses, rhythm.rotation);
    let mut touched_cells = Vec::new();

    for row in 0..row_count {
        let Some(cell) = pattern.cell_mut(row, rhythm.track) else {
            continue;
        };
        if rhythm_pattern[row % rhythm.steps] {
            cell.note = Some(NoteEvent::Note {
                pitch: rhythm.pitch.min(127),
            });
            cell.velocity = Some(rhythm.velocity.min(127));
        } else {
            cell.note = None;
            cell.velocity = None;
        }
        touched_cells.push((row, rhythm.track));
    }

    Ok(TransformReport { touched_cells })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclidean_pattern_is_deterministic() {
        assert_eq!(
            euclidean_pattern(8, 3, 0),
            vec![false, false, true, false, false, true, false, true]
        );
        assert_eq!(
            euclidean_pattern(8, 3, 1),
            vec![true, false, false, true, false, false, true, false]
        );
    }

    #[test]
    fn applies_euclidean_rhythm_to_pattern_track() {
        let mut song = Song::empty();
        song.resize_pattern(0, 8).expect("resize");

        let report = apply_euclidean(
            &mut song,
            0,
            EuclideanRhythm {
                steps: 4,
                pulses: 2,
                rotation: 0,
                track: 0,
                pitch: 36,
                velocity: 100,
            },
        )
        .expect("transform");

        let pattern = song.current_pattern().expect("pattern");
        let active_rows: Vec<_> = (0..8)
            .filter(|row| pattern.cell(*row, 0).expect("cell").note.is_some())
            .collect();

        assert_eq!(active_rows, vec![1, 3, 5, 7]);
        assert_eq!(report.touched_cells.len(), 8);
    }

    #[test]
    fn rejects_invalid_transform_specs() {
        let mut song = Song::empty();

        assert!(matches!(
            apply_euclidean(
                &mut song,
                0,
                EuclideanRhythm {
                    steps: 0,
                    pulses: 0,
                    rotation: 0,
                    track: 0,
                    pitch: 60,
                    velocity: 100,
                },
            ),
            Err(TransformError::EmptySteps)
        ));
        assert!(matches!(
            apply_euclidean(
                &mut song,
                0,
                EuclideanRhythm {
                    steps: 4,
                    pulses: 5,
                    rotation: 0,
                    track: 0,
                    pitch: 60,
                    velocity: 100,
                },
            ),
            Err(TransformError::TooManyPulses)
        ));
    }
}
