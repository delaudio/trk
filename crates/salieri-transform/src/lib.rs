use salieri_core::{EditError, NoteEvent, Song, TrackerCommand};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HumanizeSpec {
    pub pattern_index: usize,
    pub track: Option<usize>,
    pub seed: u64,
    pub velocity_amount: u8,
    pub max_delay: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanizeReport {
    pub touched_cells: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariationSpec {
    pub source_pattern_index: usize,
    pub target_name: Option<String>,
    pub track: Option<usize>,
    pub seed: u64,
    pub thin_percent: u8,
    pub fill_percent: u8,
    pub transpose: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariationReport {
    pub new_pattern_id: u32,
    pub new_pattern_index: usize,
    pub touched_cells: Vec<(usize, usize)>,
    pub added_notes: usize,
    pub removed_notes: usize,
    pub transposed_notes: usize,
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
    #[error("percentage must be between 0 and 100")]
    InvalidPercentage,
    #[error(transparent)]
    Edit(#[from] EditError),
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

pub fn apply_humanize(
    song: &mut Song,
    spec: HumanizeSpec,
) -> Result<HumanizeReport, TransformError> {
    validate_track(song, spec.track)?;
    let pattern = song
        .pattern_mut(spec.pattern_index)
        .ok_or(TransformError::MissingPattern(spec.pattern_index))?;
    let mut rng = SeededRng::new(spec.seed);
    let mut touched_cells = Vec::new();

    for row in 0..pattern.row_count() {
        let track_range = track_range(spec.track, pattern.rows[row].cells.len());
        for track in track_range {
            let Some(cell) = pattern.cell_mut(row, track) else {
                continue;
            };
            if !matches!(cell.note, Some(NoteEvent::Note { .. })) {
                continue;
            }

            if spec.velocity_amount > 0 {
                let base = cell.velocity.unwrap_or(0x7f);
                let delta = rng.signed(spec.velocity_amount);
                cell.velocity = Some(clamp_midi_i16(i16::from(base) + delta).max(1));
            }
            if spec.max_delay > 0 {
                let delay = rng.range_inclusive(spec.max_delay);
                cell.command = (delay > 0).then(|| TrackerCommand::delay(delay));
            }
            touched_cells.push((row, track));
        }
    }

    Ok(HumanizeReport { touched_cells })
}

pub fn create_variation(
    song: &mut Song,
    spec: VariationSpec,
) -> Result<VariationReport, TransformError> {
    if spec.thin_percent > 100 || spec.fill_percent > 100 {
        return Err(TransformError::InvalidPercentage);
    }
    validate_track(song, spec.track)?;
    let source = song
        .pattern(spec.source_pattern_index)
        .ok_or(TransformError::MissingPattern(spec.source_pattern_index))?
        .clone();
    let source_pitches = source_track_pitches(&source);
    let new_pattern_id = song.duplicate_pattern(spec.source_pattern_index)?;
    let new_pattern_index = song
        .patterns
        .iter()
        .position(|pattern| pattern.id == new_pattern_id)
        .expect("duplicated pattern exists");
    if let Some(name) = spec.target_name.filter(|name| !name.trim().is_empty()) {
        song.rename_pattern(new_pattern_index, name)
            .expect("non-empty variation name is valid");
    } else {
        let name = format!("{} Variation", source.name);
        song.rename_pattern(new_pattern_index, name)
            .expect("generated variation name is valid");
    }

    let pattern = song
        .pattern_mut(new_pattern_index)
        .ok_or(TransformError::MissingPattern(new_pattern_index))?;
    let mut rng = SeededRng::new(spec.seed);
    let mut report = VariationReport {
        new_pattern_id: new_pattern_id.0,
        new_pattern_index,
        touched_cells: Vec::new(),
        added_notes: 0,
        removed_notes: 0,
        transposed_notes: 0,
    };

    for row in 0..pattern.row_count() {
        let track_range = track_range(spec.track, pattern.rows[row].cells.len());
        for track in track_range {
            let Some(cell) = pattern.cell_mut(row, track) else {
                continue;
            };
            match cell.note {
                Some(NoteEvent::Note { pitch }) => {
                    if rng.percent(spec.thin_percent) {
                        cell.note = None;
                        cell.velocity = None;
                        cell.gate = None;
                        cell.command = None;
                        report.removed_notes += 1;
                        report.touched_cells.push((row, track));
                    } else if spec.transpose != 0 {
                        let next_pitch =
                            (i16::from(pitch) + i16::from(spec.transpose)).clamp(0, 127) as u8;
                        cell.note = Some(NoteEvent::Note { pitch: next_pitch });
                        report.transposed_notes += 1;
                        report.touched_cells.push((row, track));
                    }
                }
                None => {
                    if rng.percent(spec.fill_percent) {
                        let pitch = pick_pitch(&source_pitches, track, &mut rng).unwrap_or(60);
                        cell.note = Some(NoteEvent::Note { pitch });
                        cell.velocity = Some(96);
                        report.added_notes += 1;
                        report.touched_cells.push((row, track));
                    }
                }
                Some(NoteEvent::NoteOff | NoteEvent::NoteCut) => {}
            }
        }
    }

    Ok(report)
}

fn validate_track(song: &Song, track: Option<usize>) -> Result<(), TransformError> {
    if let Some(track) = track {
        if track >= song.tracks.len() {
            return Err(TransformError::MissingTrack(track));
        }
    }
    Ok(())
}

fn track_range(track: Option<usize>, track_count: usize) -> Box<dyn Iterator<Item = usize>> {
    if let Some(track) = track {
        Box::new(std::iter::once(track))
    } else {
        Box::new(0..track_count)
    }
}

fn source_track_pitches(pattern: &salieri_core::Pattern) -> Vec<Vec<u8>> {
    let track_count = pattern.rows.first().map_or(0, |row| row.cells.len());
    let mut pitches = vec![Vec::new(); track_count];
    for row in &pattern.rows {
        for (track, cell) in row.cells.iter().enumerate() {
            if let Some(NoteEvent::Note { pitch }) = cell.note {
                pitches[track].push(pitch);
            }
        }
    }
    pitches
}

fn pick_pitch(source_pitches: &[Vec<u8>], track: usize, rng: &mut SeededRng) -> Option<u8> {
    let pitches = source_pitches.get(track)?;
    if pitches.is_empty() {
        None
    } else {
        Some(pitches[rng.index(pitches.len())])
    }
}

fn clamp_midi_i16(value: i16) -> u8 {
    value.clamp(0, 127) as u8
}

#[derive(Debug, Clone, Copy)]
struct SeededRng {
    state: u64,
}

impl SeededRng {
    const fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn range_inclusive(&mut self, max: u8) -> u8 {
        (self.next_u32() % (u32::from(max) + 1)) as u8
    }

    fn signed(&mut self, amount: u8) -> i16 {
        let span = u32::from(amount).saturating_mul(2).saturating_add(1);
        (self.next_u32() % span) as i16 - i16::from(amount)
    }

    fn percent(&mut self, percent: u8) -> bool {
        percent > 0 && self.next_u32() % 100 < u32::from(percent)
    }

    fn index(&mut self, len: usize) -> usize {
        if len == 0 {
            0
        } else {
            self.next_u32() as usize % len
        }
    }
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

    #[test]
    fn humanize_is_deterministic_for_fixed_seed() {
        let mut left = Song::empty();
        let mut right = Song::empty();
        left.resize_pattern(0, 8).expect("resize");
        right.resize_pattern(0, 8).expect("resize");
        for song in [&mut left, &mut right] {
            song.current_pattern_mut()
                .expect("pattern")
                .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
                .expect("note");
            song.current_pattern_mut()
                .expect("pattern")
                .set_note(4, 0, NoteEvent::Note { pitch: 62 }, 100)
                .expect("note");
        }

        let spec = HumanizeSpec {
            pattern_index: 0,
            track: Some(0),
            seed: 42,
            velocity_amount: 8,
            max_delay: 32,
        };
        let left_report = apply_humanize(&mut left, spec).expect("left humanize");
        let right_report = apply_humanize(&mut right, spec).expect("right humanize");

        assert_eq!(left, right);
        assert_eq!(left_report, right_report);
        assert_eq!(left_report.touched_cells, vec![(0, 0), (4, 0)]);
        assert!(left.patterns[0].rows[0].cells[0].command.is_some());
    }

    #[test]
    fn variation_duplicates_and_mutates_pattern_deterministically() {
        let mut song = Song::empty();
        song.resize_pattern(0, 8).expect("resize");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
            .expect("note");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(4, 0, NoteEvent::Note { pitch: 64 }, 100)
            .expect("note");

        let report = create_variation(
            &mut song,
            VariationSpec {
                source_pattern_index: 0,
                target_name: Some("Variation A".to_string()),
                track: Some(0),
                seed: 7,
                thin_percent: 0,
                fill_percent: 20,
                transpose: 12,
            },
        )
        .expect("variation");

        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.patterns[1].name, "Variation A");
        assert_eq!(report.new_pattern_index, 1);
        assert!(report.transposed_notes >= 2);
        assert!(!report.touched_cells.is_empty());
    }
}
