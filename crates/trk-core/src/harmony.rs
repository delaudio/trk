use std::fmt;

use crate::{NoteEvent, Pattern, Song};

const PITCH_CLASS_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    Major,
    Minor,
    Dorian,
    Mixolydian,
    Hirajoshi,
    Pentatonic,
}

impl ScaleMode {
    pub const ALL: [Self; 6] = [
        Self::Major,
        Self::Minor,
        Self::Dorian,
        Self::Mixolydian,
        Self::Hirajoshi,
        Self::Pentatonic,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Dorian => "dorian",
            Self::Mixolydian => "mixolydian",
            Self::Hirajoshi => "hirajoshi",
            Self::Pentatonic => "pentatonic",
        }
    }

    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Major => "maj",
            Self::Minor => "min",
            Self::Dorian => "dor",
            Self::Mixolydian => "mixo",
            Self::Hirajoshi => "hira",
            Self::Pentatonic => "penta",
        }
    }

    #[must_use]
    pub const fn intervals(self) -> &'static [u8] {
        match self {
            Self::Major => &[0, 2, 4, 5, 7, 9, 11],
            Self::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Self::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Self::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Self::Hirajoshi => &[0, 2, 3, 7, 8],
            Self::Pentatonic => &[0, 2, 4, 7, 9],
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "major" | "maj" | "ionian" => Some(Self::Major),
            "minor" | "min" | "natural-minor" | "aeolian" => Some(Self::Minor),
            "dorian" | "dor" => Some(Self::Dorian),
            "mixolydian" | "mixo" => Some(Self::Mixolydian),
            "hirajoshi" | "hira" => Some(Self::Hirajoshi),
            "pentatonic" | "penta" | "major-pentatonic" => Some(Self::Pentatonic),
            _ => None,
        }
    }
}

impl fmt::Display for ScaleMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarmonicScale {
    root: u8,
    mode: ScaleMode,
}

impl HarmonicScale {
    #[must_use]
    pub const fn new(root: u8, mode: ScaleMode) -> Option<Self> {
        if root < 12 {
            Some(Self { root, mode })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn root(self) -> u8 {
        self.root
    }

    #[must_use]
    pub const fn mode(self) -> ScaleMode {
        self.mode
    }

    #[must_use]
    pub const fn intervals(self) -> &'static [u8] {
        self.mode.intervals()
    }

    #[must_use]
    pub fn degree_pitch(self, octave: u8, degree: usize) -> Option<u8> {
        let intervals = self.intervals();
        let scale_octave = degree / intervals.len();
        let interval = usize::from(intervals[degree % intervals.len()]);
        let base = usize::from(octave)
            .checked_add(1)?
            .checked_mul(12)?
            .checked_add(usize::from(self.root))?;
        let pitch = base
            .checked_add(scale_octave.checked_mul(12)?)?
            .checked_add(interval)?;
        u8::try_from(pitch).ok().filter(|pitch| *pitch <= 127)
    }

    #[must_use]
    pub fn label(self) -> String {
        format!("{} {}", pitch_class_name(self.root), self.mode.name())
    }

    #[must_use]
    pub fn short_label(self) -> String {
        format!("{}:{}", pitch_class_name(self.root), self.mode.short_name())
    }
}

impl Default for HarmonicScale {
    fn default() -> Self {
        Self {
            root: 0,
            mode: ScaleMode::Major,
        }
    }
}

#[must_use]
pub const fn pitch_class_name(pitch_class: u8) -> &'static str {
    PITCH_CLASS_NAMES[(pitch_class % 12) as usize]
}

#[must_use]
pub fn parse_pitch_class(value: &str) -> Option<u8> {
    match value.trim().to_ascii_lowercase().as_str() {
        "c" | "b#" => Some(0),
        "c#" | "db" => Some(1),
        "d" => Some(2),
        "d#" | "eb" => Some(3),
        "e" | "fb" => Some(4),
        "f" | "e#" => Some(5),
        "f#" | "gb" => Some(6),
        "g" => Some(7),
        "g#" | "ab" => Some(8),
        "a" => Some(9),
        "a#" | "bb" => Some(10),
        "b" | "cb" => Some(11),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    SuspendedSecond,
    SuspendedFourth,
    Sixth,
    MinorSixth,
    DominantSeventh,
    MajorSeventh,
    MinorSeventh,
    MinorMajorSeventh,
    HalfDiminishedSeventh,
    DiminishedSeventh,
    DominantNinth,
    MajorNinth,
    MinorNinth,
}

impl ChordQuality {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Major => "",
            Self::Minor => "m",
            Self::Diminished => "dim",
            Self::Augmented => "aug",
            Self::SuspendedSecond => "sus2",
            Self::SuspendedFourth => "sus4",
            Self::Sixth => "6",
            Self::MinorSixth => "m6",
            Self::DominantSeventh => "7",
            Self::MajorSeventh => "maj7",
            Self::MinorSeventh => "m7",
            Self::MinorMajorSeventh => "mMaj7",
            Self::HalfDiminishedSeventh => "m7b5",
            Self::DiminishedSeventh => "dim7",
            Self::DominantNinth => "9",
            Self::MajorNinth => "maj9",
            Self::MinorNinth => "m9",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChordName {
    pub root: u8,
    pub quality: ChordQuality,
}

impl fmt::Display for ChordName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}{}",
            pitch_class_name(self.root),
            self.quality.suffix()
        )
    }
}

const CHORD_TEMPLATES: &[(u16, ChordQuality)] = &[
    (interval_mask(&[0, 2, 4, 7, 11]), ChordQuality::MajorNinth),
    (
        interval_mask(&[0, 2, 4, 7, 10]),
        ChordQuality::DominantNinth,
    ),
    (interval_mask(&[0, 2, 3, 7, 10]), ChordQuality::MinorNinth),
    (
        interval_mask(&[0, 3, 6, 9]),
        ChordQuality::DiminishedSeventh,
    ),
    (
        interval_mask(&[0, 3, 6, 10]),
        ChordQuality::HalfDiminishedSeventh,
    ),
    (
        interval_mask(&[0, 3, 7, 11]),
        ChordQuality::MinorMajorSeventh,
    ),
    (interval_mask(&[0, 4, 7, 11]), ChordQuality::MajorSeventh),
    (interval_mask(&[0, 4, 7, 10]), ChordQuality::DominantSeventh),
    (interval_mask(&[0, 3, 7, 10]), ChordQuality::MinorSeventh),
    (interval_mask(&[0, 3, 7, 9]), ChordQuality::MinorSixth),
    (interval_mask(&[0, 4, 7, 9]), ChordQuality::Sixth),
    (interval_mask(&[0, 2, 7]), ChordQuality::SuspendedSecond),
    (interval_mask(&[0, 5, 7]), ChordQuality::SuspendedFourth),
    (interval_mask(&[0, 3, 6]), ChordQuality::Diminished),
    (interval_mask(&[0, 4, 8]), ChordQuality::Augmented),
    (interval_mask(&[0, 3, 7]), ChordQuality::Minor),
    (interval_mask(&[0, 4, 7]), ChordQuality::Major),
];

const fn interval_mask(intervals: &[u8]) -> u16 {
    let mut mask = 0_u16;
    let mut index = 0;
    while index < intervals.len() {
        mask |= 1_u16 << intervals[index];
        index += 1;
    }
    mask
}

#[must_use]
pub fn identify_chord(pitches: &[u8]) -> Option<ChordName> {
    let bass = pitches.iter().copied().min()? % 12;
    let pitch_mask = pitches
        .iter()
        .fold(0_u16, |mask, pitch| mask | (1_u16 << (pitch % 12)));
    if pitch_mask.count_ones() < 3 {
        return None;
    }

    let mut fallback = None;
    for root in 0..12_u8 {
        let normalized = normalize_pitch_mask(pitch_mask, root);
        for &(template, quality) in CHORD_TEMPLATES {
            if normalized != template {
                continue;
            }
            let chord = ChordName { root, quality };
            if root == bass {
                return Some(chord);
            }
            fallback.get_or_insert(chord);
        }
    }
    fallback
}

fn normalize_pitch_mask(pitch_mask: u16, root: u8) -> u16 {
    let mut normalized = 0_u16;
    for pitch_class in 0..12_u8 {
        if pitch_mask & (1_u16 << pitch_class) != 0 {
            let interval = (pitch_class + 12 - root) % 12;
            normalized |= 1_u16 << interval;
        }
    }
    normalized
}

#[must_use]
pub fn active_pitches_at_row(song: &Song, pattern: &Pattern, row: usize) -> Vec<u8> {
    if row >= pattern.row_count() {
        return Vec::new();
    }
    let solo_active = song.tracks.iter().any(|track| track.solo);
    let mut pitches = song
        .tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| !track.muted && (!solo_active || track.solo))
        .filter_map(|(track_index, _)| active_pitch_for_track(pattern, row, track_index))
        .collect::<Vec<_>>();
    pitches.sort_unstable();
    pitches
}

fn active_pitch_for_track(pattern: &Pattern, row: usize, track: usize) -> Option<u8> {
    for event_row in (0..=row).rev() {
        let Some(note) = pattern.cell(event_row, track)?.note else {
            continue;
        };
        return match note {
            NoteEvent::Note { pitch } => pattern
                .note_gate_rows(event_row, track)
                .filter(|gate| row < event_row.saturating_add(*gate))
                .map(|_| pitch),
            NoteEvent::NoteOff | NoteEvent::NoteCut => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scales_and_quantizes_degrees_across_octaves() {
        assert_eq!(parse_pitch_class("Db"), Some(1));
        assert_eq!(ScaleMode::parse("natural-minor"), Some(ScaleMode::Minor));
        assert_eq!(
            ScaleMode::parse("major-pentatonic"),
            Some(ScaleMode::Pentatonic)
        );

        let d_minor = HarmonicScale::new(2, ScaleMode::Minor).expect("D minor");
        let pitches = (0..8)
            .map(|degree| d_minor.degree_pitch(3, degree).expect("bounded pitch"))
            .collect::<Vec<_>>();
        assert_eq!(pitches, vec![50, 52, 53, 55, 57, 58, 60, 62]);
        assert_eq!(d_minor.degree_pitch(9, 7), None);
    }

    #[test]
    fn identifies_required_chords_and_inversions() {
        assert_eq!(
            identify_chord(&[50, 53, 57, 60]).map(|c| c.to_string()),
            Some("Dm7".into())
        );
        assert_eq!(
            identify_chord(&[53, 57, 60, 64, 67]).map(|c| c.to_string()),
            Some("Fmaj9".into())
        );
        assert_eq!(
            identify_chord(&[43, 48, 50]).map(|c| c.to_string()),
            Some("Gsus4".into())
        );
        assert_eq!(
            identify_chord(&[49, 52, 55, 58]).map(|c| c.to_string()),
            Some("C#dim7".into())
        );
        assert_eq!(
            identify_chord(&[52, 55, 60]).map(|c| c.to_string()),
            Some("C".into())
        );
        assert_eq!(
            identify_chord(&[57, 60, 64, 67]).map(|c| c.to_string()),
            Some("Am7".into())
        );
        assert_eq!(
            identify_chord(&[60, 64, 67, 69]).map(|c| c.to_string()),
            Some("C6".into())
        );
        assert_eq!(identify_chord(&[60, 67]), None);
        assert_eq!(identify_chord(&[60, 61, 67]), None);
    }

    #[test]
    fn active_row_pitches_follow_gates_terminators_mute_and_solo() {
        let mut song = Song::empty();
        let pattern = song.pattern_mut(0).expect("default pattern");
        pattern
            .set_note(0, 0, NoteEvent::Note { pitch: 50 }, 100)
            .expect("D");
        pattern.set_gate(0, 0, Some(3)).expect("D gate");
        pattern
            .set_note(0, 1, NoteEvent::Note { pitch: 53 }, 100)
            .expect("F");
        pattern.set_gate(0, 1, Some(3)).expect("F gate");
        pattern
            .set_note(0, 2, NoteEvent::Note { pitch: 57 }, 100)
            .expect("A");
        pattern.set_gate(0, 2, Some(3)).expect("A gate");
        pattern
            .set_note_event(2, 1, NoteEvent::NoteOff, None)
            .expect("off");

        let pattern = song.pattern(0).expect("default pattern");
        assert_eq!(active_pitches_at_row(&song, pattern, 1), vec![50, 53, 57]);
        assert_eq!(active_pitches_at_row(&song, pattern, 2), vec![50, 57]);

        song.tracks[2].muted = true;
        let pattern = song.pattern(0).expect("default pattern");
        assert_eq!(active_pitches_at_row(&song, pattern, 1), vec![50, 53]);
        song.tracks[0].solo = true;
        let pattern = song.pattern(0).expect("default pattern");
        assert_eq!(active_pitches_at_row(&song, pattern, 1), vec![50]);
    }
}
