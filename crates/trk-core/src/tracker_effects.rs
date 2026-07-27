use thiserror::Error;

use crate::{PatternCell, PatternId, Song, TrackId, TrackerCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerCommandDomain {
    PlaybackTiming,
    Sample,
    Midi,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerCommandSupport {
    Supported,
    Deferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackerCommandSpec {
    pub code: u8,
    pub name: &'static str,
    pub semantics: &'static str,
    pub domain: TrackerCommandDomain,
    pub support: TrackerCommandSupport,
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TrackerCommandParseError {
    #[error("unsupported tracker FX command {code}")]
    Unsupported { code: char },
    #[error("tracker FX command {code} is deferred")]
    Deferred { code: char },
    #[error("tracker FX command {code} value {value:#04x} is outside {min:#04x}..={max:#04x}")]
    InvalidRange {
        code: char,
        value: u8,
        min: u8,
        max: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerCommandSlot {
    Fx1,
    Fx2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerCommandDiagnosticKind {
    Deferred,
    InvalidRange,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerCommandDiagnostic {
    pub pattern: PatternId,
    pub row: usize,
    pub track: TrackId,
    pub slot: TrackerCommandSlot,
    pub command: TrackerCommand,
    pub kind: TrackerCommandDiagnosticKind,
    pub message: String,
}

const SPECS: &[TrackerCommandSpec] = &[
    supported(
        b'D',
        "delay",
        "Offsets note and sample triggers within the current row.",
        TrackerCommandDomain::PlaybackTiming,
        0x00,
        0xff,
    ),
    supported(
        b'R',
        "retrigger",
        "Repeats the current note or sample deterministically within the current row.",
        TrackerCommandDomain::PlaybackTiming,
        0x01,
        0x10,
    ),
    deferred(
        b'V',
        "volume",
        "Sample voice gain and random-volume commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'P',
        "pan",
        "Sample voice pan and stereo-position commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'O',
        "sample position",
        "Sample start offset, reverse, and slice-position commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'C',
        "note cut/gate",
        "Gate length, note cut, and chord-output commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'U',
        "slide up",
        "Pitch slide up, tuning, and microtune commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'N',
        "slide down",
        "Pitch slide down, tuning, and microtune commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'T',
        "tempo",
        "Project tempo commands.",
        TrackerCommandDomain::Project,
    ),
    deferred(
        b'W',
        "swing",
        "Project swing commands.",
        TrackerCommandDomain::Project,
    ),
    deferred(
        b'M',
        "micro move",
        "Micro-timing move commands.",
        TrackerCommandDomain::PlaybackTiming,
    ),
    deferred(
        b'G',
        "glide",
        "Sample pitch glide commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'Q',
        "chance",
        "Probability gates for conditional note/sample playback.",
        TrackerCommandDomain::PlaybackTiming,
    ),
    deferred(
        b'L',
        "roll",
        "Roll and deterministic LFO-rate commands.",
        TrackerCommandDomain::PlaybackTiming,
    ),
    deferred(
        b'A',
        "arp/chord",
        "Arpeggio, chord output, and chord-shape commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'X',
        "random",
        "Random note, random instrument, random FX, and random volume commands.",
        TrackerCommandDomain::PlaybackTiming,
    ),
    deferred(
        b'B',
        "bit depth",
        "Sample-local bit-depth and lo-fi commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'F',
        "filter",
        "Sample-local filter commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'S',
        "send/slice",
        "Track send level and sample slice commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'H',
        "drive",
        "Sample-local overdrive and distortion commands.",
        TrackerCommandDomain::Sample,
    ),
    deferred(
        b'I',
        "MIDI CC",
        "MIDI continuous-controller commands.",
        TrackerCommandDomain::Midi,
    ),
    deferred(
        b'K',
        "program change",
        "MIDI program-change commands.",
        TrackerCommandDomain::Midi,
    ),
    deferred(
        b'Y',
        "aftertouch",
        "MIDI channel and polyphonic aftertouch commands.",
        TrackerCommandDomain::Midi,
    ),
];

const fn supported(
    code: u8,
    name: &'static str,
    semantics: &'static str,
    domain: TrackerCommandDomain,
    min: u8,
    max: u8,
) -> TrackerCommandSpec {
    TrackerCommandSpec {
        code,
        name,
        semantics,
        domain,
        support: TrackerCommandSupport::Supported,
        min,
        max,
    }
}

const fn deferred(
    code: u8,
    name: &'static str,
    semantics: &'static str,
    domain: TrackerCommandDomain,
) -> TrackerCommandSpec {
    TrackerCommandSpec {
        code,
        name,
        semantics,
        domain,
        support: TrackerCommandSupport::Deferred,
        min: 0x00,
        max: 0xff,
    }
}

pub fn tracker_command_specs() -> &'static [TrackerCommandSpec] {
    SPECS
}

pub fn tracker_command_spec(code: u8) -> Option<&'static TrackerCommandSpec> {
    let code = code.to_ascii_uppercase();
    SPECS.iter().find(|spec| spec.code == code)
}

pub fn parse_tracker_command(
    code: char,
    value: u8,
) -> Result<TrackerCommand, TrackerCommandParseError> {
    let code = code.to_ascii_uppercase();
    let spec =
        tracker_command_spec(code as u8).ok_or(TrackerCommandParseError::Unsupported { code })?;
    if spec.support == TrackerCommandSupport::Deferred {
        return Err(TrackerCommandParseError::Deferred { code });
    }
    if !(spec.min..=spec.max).contains(&value) {
        return Err(TrackerCommandParseError::InvalidRange {
            code,
            value,
            min: spec.min,
            max: spec.max,
        });
    }
    Ok(TrackerCommand::from_code_char(code, value))
}

impl Song {
    pub fn tracker_command_diagnostics(&self) -> Vec<TrackerCommandDiagnostic> {
        self.patterns
            .iter()
            .flat_map(|pattern| {
                pattern
                    .rows
                    .iter()
                    .enumerate()
                    .flat_map(move |(row, pattern_row)| {
                        pattern_row
                            .cells
                            .iter()
                            .enumerate()
                            .flat_map(move |(track_index, cell)| {
                                self.tracks
                                    .get(track_index)
                                    .into_iter()
                                    .flat_map(move |track| {
                                        cell_command_diagnostics(pattern.id, row, track.id, cell)
                                    })
                            })
                    })
            })
            .collect()
    }
}

fn cell_command_diagnostics(
    pattern: PatternId,
    row: usize,
    track: TrackId,
    cell: &PatternCell,
) -> impl Iterator<Item = TrackerCommandDiagnostic> + '_ {
    [
        (TrackerCommandSlot::Fx1, cell.command),
        (TrackerCommandSlot::Fx2, cell.command2),
    ]
    .into_iter()
    .filter_map(move |(slot, command)| {
        command.and_then(|command| command_diagnostic(pattern, row, track, slot, command))
    })
}

fn command_diagnostic(
    pattern: PatternId,
    row: usize,
    track: TrackId,
    slot: TrackerCommandSlot,
    command: TrackerCommand,
) -> Option<TrackerCommandDiagnostic> {
    let (kind, message) = if let Some(spec) = tracker_command_spec(command.code) {
        if spec.support == TrackerCommandSupport::Deferred {
            (
                TrackerCommandDiagnosticKind::Deferred,
                format!(
                    "FX command {}{:02X} ({}) is deferred",
                    command.display_code(),
                    command.value,
                    spec.name
                ),
            )
        } else if !(spec.min..=spec.max).contains(&command.value) {
            (
                TrackerCommandDiagnosticKind::InvalidRange,
                format!(
                    "FX command {}{:02X} is outside {:02X}..={:02X}",
                    command.display_code(),
                    command.value,
                    spec.min,
                    spec.max
                ),
            )
        } else {
            return None;
        }
    } else {
        (
            TrackerCommandDiagnosticKind::Unsupported,
            format!(
                "FX command {}{:02X} is unsupported",
                command.display_code(),
                command.value
            ),
        )
    };
    Some(TrackerCommandDiagnostic {
        pattern,
        row,
        track,
        slot,
        command,
        kind,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NoteEvent, Song};

    #[test]
    fn parser_accepts_supported_commands_and_rejects_deferred_or_invalid_values() {
        assert_eq!(
            parse_tracker_command('d', 0x20),
            Ok(TrackerCommand::delay(0x20))
        );
        assert!(matches!(
            parse_tracker_command('R', 0),
            Err(TrackerCommandParseError::InvalidRange { .. })
        ));
        assert!(matches!(
            parse_tracker_command('V', 0x40),
            Err(TrackerCommandParseError::Deferred { .. })
        ));
        assert!(matches!(
            parse_tracker_command('?', 0x40),
            Err(TrackerCommandParseError::Unsupported { .. })
        ));
    }

    #[test]
    fn catalog_documents_deferred_tracker_mini_families() {
        let semantics = tracker_command_specs()
            .iter()
            .map(|spec| spec.semantics)
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();

        for expected in [
            "random note",
            "random instrument",
            "random fx",
            "random volume",
            "microtune",
            "reverse",
            "lfo-rate",
            "overdrive",
            "midi continuous-controller",
            "program-change",
            "aftertouch",
        ] {
            assert!(semantics.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn song_reports_deferred_tracker_command_diagnostics() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 0x7f)
            .expect("note");
        song.current_pattern_mut()
            .expect("pattern")
            .cell_mut(0, 0)
            .expect("cell")
            .command = Some(TrackerCommand::from_code_char('V', 0x40));

        let diagnostics = song.tracker_command_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].kind, TrackerCommandDiagnosticKind::Deferred);
        assert_eq!(diagnostics[0].slot, TrackerCommandSlot::Fx1);
    }

    #[test]
    fn song_reports_unknown_tracker_command_diagnostics() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .cell_mut(0, 0)
            .expect("cell")
            .command2 = Some(TrackerCommand::from_code_char('?', 0x40));

        let diagnostics = song.tracker_command_diagnostics();

        assert_eq!(
            diagnostics[0].kind,
            TrackerCommandDiagnosticKind::Unsupported
        );
        assert_eq!(diagnostics[0].slot, TrackerCommandSlot::Fx2);
    }
}
