use std::collections::{HashMap, HashSet};

use salieri_core::{
    pattern_events, EffectDevice, InstrumentId, NoteEvent, PatternCell, PlaybackEventKind, Song,
    TrackerCommand,
};

const MTHD: &[u8; 4] = b"MThd";
const MTRK: &[u8; 4] = b"MTrk";
const DEFAULT_TICKS_PER_QUARTER: u16 = 480;
const ZIP_LOCAL_FILE_HEADER: u32 = 0x0403_4b50;
const ZIP_CENTRAL_DIRECTORY_HEADER: u32 = 0x0201_4b50;
const ZIP_END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiExportOptions {
    pub pattern: usize,
    pub ticks_per_quarter: u16,
}

impl Default for MidiExportOptions {
    fn default() -> Self {
        Self {
            pattern: 0,
            ticks_per_quarter: DEFAULT_TICKS_PER_QUARTER,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerModuleFormat {
    Mod,
    Xm,
    It,
    S3m,
    Renoise,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerModuleInspection {
    pub format: TrackerModuleFormat,
    pub title: Option<String>,
    pub channels: Option<usize>,
    pub patterns: Option<usize>,
    pub samples: Vec<TrackerModuleSampleInfo>,
    pub instrument_count: Option<usize>,
    pub effect_commands: Vec<u8>,
    pub diagnostics: Vec<TrackerModuleDiagnostic>,
    pub recommendation: TrackerModuleRecommendation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerModuleSampleInfo {
    pub index: usize,
    pub name: Option<String>,
    pub length_bytes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerModuleDiagnostic {
    pub kind: TrackerModuleDiagnosticKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerModuleDiagnosticKind {
    MalformedModule,
    UnsupportedTimingSemantics,
    UnsupportedEffectMemory,
    EffectDecodeIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerModuleRecommendation {
    SampleExtractionOnly,
    CoarseNoteImportNeedsTimingSpike,
}

#[derive(Debug, thiserror::Error)]
pub enum InteropError {
    #[error("pattern {0} does not exist")]
    MissingPattern(usize),
    #[error("invalid MIDI header")]
    InvalidMidiHeader,
    #[error("invalid MIDI track")]
    InvalidMidiTrack,
    #[error("truncated MIDI data")]
    TruncatedMidiData,
    #[error("unsupported MIDI format {0}")]
    UnsupportedMidiFormat(u16),
    #[error("unsupported SMPTE MIDI division {0:#06x}")]
    UnsupportedSmpteDivision(u16),
    #[error("unsupported MIDI event status {0:#04x}")]
    UnsupportedMidiEvent(u8),
    #[error("unsupported tracker module format {0:?}")]
    UnsupportedTrackerModule(TrackerModuleFormat),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsInspection {
    pub is_zip: bool,
    pub song_xml_path: Option<String>,
    pub archive_entries: Vec<XrnsArchiveEntry>,
    pub sample_payloads: Vec<XrnsSamplePayload>,
    pub tracks: Vec<XrnsTrackInfo>,
    pub patterns: Vec<XrnsPatternInfo>,
    pub instruments: Vec<XrnsInstrumentInfo>,
    pub device_chains: Vec<XrnsDeviceChainInfo>,
    pub diagnostics: Vec<XrnsDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct XrnsImportReport {
    pub song: Option<Song>,
    pub inspection: XrnsInspection,
    pub diagnostics: Vec<XrnsDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsArchiveEntry {
    pub path: String,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub compression_method: u16,
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsSamplePayload {
    pub path: String,
    pub format: String,
    pub bytes: u32,
    pub supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsTrackInfo {
    pub index: usize,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsPatternInfo {
    pub index: usize,
    pub rows: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsInstrumentInfo {
    pub index: usize,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsDeviceChainInfo {
    pub location: String,
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrnsDiagnostic {
    pub kind: XrnsDiagnosticKind,
    pub severity: XrnsDiagnosticSeverity,
    pub location: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrnsDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrnsDiagnosticKind {
    MalformedArchive,
    MissingSongXml,
    MalformedSongXml,
    EncryptedArchive,
    NestedArchive,
    UnsupportedCompression,
    UnsupportedSampleFormat,
    UnsupportedEffectCommand,
    DroppedExtraEffectColumn,
    TimingQuantized,
    UnsupportedRenoiseFeature,
    ValidationFailed,
}

pub fn export_pattern_smf(
    song: &Song,
    options: MidiExportOptions,
) -> Result<Vec<u8>, InteropError> {
    let pattern = song
        .pattern(options.pattern)
        .ok_or(InteropError::MissingPattern(options.pattern))?;
    let ticks_per_quarter = options.ticks_per_quarter.max(1);
    let micros_per_quarter = 60_000_000_u64 / u64::from(song.transport.bpm.max(1));
    let mut track = Vec::new();

    write_var_len(0, &mut track);
    track.extend_from_slice(&[0xff, 0x51, 0x03]);
    let tempo = micros_per_quarter.min(0x00ff_ffff) as u32;
    track.extend_from_slice(&tempo.to_be_bytes()[1..4]);

    let mut last_tick = 0_u64;
    for event in pattern_events(song, pattern) {
        let tick = event
            .position
            .offset_micros
            .saturating_mul(u64::from(ticks_per_quarter))
            / micros_per_quarter;
        write_var_len(tick.saturating_sub(last_tick), &mut track);
        last_tick = tick;

        let channel = event.midi_channel.saturating_sub(1).min(15);
        match event.kind {
            PlaybackEventKind::NoteOn { pitch, velocity } => {
                track.extend_from_slice(&[0x90 | channel, pitch.min(127), velocity.min(127)]);
            }
            PlaybackEventKind::NoteOff { pitch } => {
                track.extend_from_slice(&[0x80 | channel, pitch.min(127), 0]);
            }
        }
    }

    write_var_len(0, &mut track);
    track.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(MTHD);
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&ticks_per_quarter.to_be_bytes());
    bytes.extend_from_slice(MTRK);
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&track);
    Ok(bytes)
}

pub fn import_smf(bytes: &[u8]) -> Result<Song, InteropError> {
    let mut cursor = Cursor::new(bytes);
    if cursor.read_exact(4)? != MTHD {
        return Err(InteropError::InvalidMidiHeader);
    }
    let header_len = cursor.read_u32()? as usize;
    if header_len < 6 {
        return Err(InteropError::InvalidMidiHeader);
    }
    let format = cursor.read_u16()?;
    let track_count = cursor.read_u16()?;
    let division = cursor.read_u16()?;
    cursor.skip(header_len - 6)?;

    if format != 0 {
        return Err(InteropError::UnsupportedMidiFormat(format));
    }
    if track_count != 1 {
        return Err(InteropError::UnsupportedMidiFormat(format));
    }
    if division & 0x8000 != 0 {
        return Err(InteropError::UnsupportedSmpteDivision(division));
    }

    if cursor.read_exact(4)? != MTRK {
        return Err(InteropError::InvalidMidiTrack);
    }
    let track_len = cursor.read_u32()? as usize;
    let track_end = cursor.position().saturating_add(track_len);
    if track_end > bytes.len() {
        return Err(InteropError::TruncatedMidiData);
    }

    let mut song = Song::empty();
    let ticks_per_quarter = u64::from(division.max(1));
    let mut absolute_tick = 0_u64;
    let mut running_status = None;
    let mut channel_tracks = HashMap::new();

    while cursor.position() < track_end {
        absolute_tick = absolute_tick.saturating_add(read_var_len(&mut cursor)?);
        let mut status = cursor.read_u8()?;
        if status < 0x80 {
            status = running_status.ok_or(InteropError::UnsupportedMidiEvent(status))?;
            cursor.rewind(1);
        } else if status < 0xf0 {
            running_status = Some(status);
        }

        match status {
            0x80..=0x9f => {
                let pitch = cursor.read_u8()?.min(127);
                let velocity = cursor.read_u8()?.min(127);
                if status & 0xf0 == 0x90 && velocity > 0 {
                    let channel = (status & 0x0f) + 1;
                    let track = track_for_channel(&mut song, &mut channel_tracks, channel);
                    let row = ticks_to_row(
                        absolute_tick,
                        ticks_per_quarter,
                        song.transport.lines_per_beat,
                    );
                    ensure_pattern_row(&mut song, row)?;
                    song.current_pattern_mut()
                        .expect("default song has a pattern")
                        .set_note(row, track, NoteEvent::Note { pitch }, velocity)
                        .expect("row and track were ensured");
                }
            }
            0xff => {
                let meta_type = cursor.read_u8()?;
                let len = read_var_len(&mut cursor)? as usize;
                if meta_type == 0x51 && len == 3 {
                    let tempo = read_tempo(&mut cursor)?;
                    song.transport.bpm = tempo_to_bpm(tempo);
                } else {
                    cursor.skip(len)?;
                }
                if meta_type == 0x2f {
                    break;
                }
            }
            0xf0 | 0xf7 => {
                return Err(InteropError::UnsupportedMidiEvent(status));
            }
            _ => return Err(InteropError::UnsupportedMidiEvent(status)),
        }
    }

    Ok(song)
}

pub fn import_tracker_module(
    _bytes: &[u8],
    format: TrackerModuleFormat,
) -> Result<Song, InteropError> {
    Err(InteropError::UnsupportedTrackerModule(format))
}

#[must_use]
pub fn inspect_tracker_module(
    bytes: &[u8],
    format: TrackerModuleFormat,
) -> TrackerModuleInspection {
    match format {
        TrackerModuleFormat::Mod => inspect_mod_module(bytes),
        TrackerModuleFormat::Xm => inspect_xm_module(bytes),
        TrackerModuleFormat::S3m => inspect_s3m_module(bytes),
        TrackerModuleFormat::It => inspect_it_module(bytes),
        TrackerModuleFormat::Renoise => TrackerModuleInspection {
            format,
            title: None,
            channels: None,
            patterns: None,
            samples: Vec::new(),
            instrument_count: None,
            effect_commands: Vec::new(),
            diagnostics: vec![tracker_module_diagnostic(
                TrackerModuleDiagnosticKind::MalformedModule,
                "XRNS is handled by inspect_xrns/import_xrns, not legacy module inspection",
            )],
            recommendation: TrackerModuleRecommendation::SampleExtractionOnly,
        },
    }
}

fn inspect_mod_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 1084 {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "MOD data is too short for a 31-sample header",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::Mod,
            None,
            None,
            None,
            Vec::new(),
            Some(0),
            Vec::new(),
            diagnostics,
        );
    }

    let signature = &bytes[1080..1084];
    let channels = match signature {
        b"M.K." | b"M!K!" | b"4CHN" => Some(4),
        b"6CHN" => Some(6),
        b"8CHN" => Some(8),
        _ => None,
    };
    let song_len = usize::from(bytes[950]).min(128);
    let patterns = bytes[952..1080]
        .iter()
        .take(song_len)
        .copied()
        .max()
        .map(|pattern| usize::from(pattern) + 1);
    let samples = (0..31)
        .map(|index| {
            let offset = 20 + index * 30;
            let name = clean_ascii(&bytes[offset..offset + 22]);
            let length_words = u16::from_be_bytes([bytes[offset + 22], bytes[offset + 23]]);
            TrackerModuleSampleInfo {
                index,
                name,
                length_bytes: Some(usize::from(length_words) * 2),
            }
        })
        .collect::<Vec<_>>();

    let mut effect_commands = Vec::new();
    if let (Some(channels), Some(patterns)) = (channels, patterns) {
        let pattern_data_start = 1084;
        let pattern_bytes = patterns
            .saturating_mul(64)
            .saturating_mul(channels)
            .saturating_mul(4);
        if bytes.len() >= pattern_data_start + pattern_bytes {
            let mut commands = HashSet::new();
            for event in bytes[pattern_data_start..pattern_data_start + pattern_bytes].chunks(4) {
                let command = event[2] & 0x0f;
                if command != 0 {
                    commands.insert(command);
                }
            }
            effect_commands = commands.into_iter().collect();
            effect_commands.sort_unstable();
        } else {
            diagnostics.push(tracker_module_diagnostic(
                TrackerModuleDiagnosticKind::MalformedModule,
                "MOD pattern data is truncated",
            ));
        }
    }

    tracker_module_inspection(
        TrackerModuleFormat::Mod,
        clean_ascii(&bytes[0..20]),
        channels,
        patterns,
        samples,
        Some(31),
        effect_commands,
        diagnostics,
    )
}

fn inspect_xm_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 80 || !bytes.starts_with(b"Extended Module: ") {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "XM data is missing the Extended Module header",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::Xm,
            None,
            None,
            None,
            Vec::new(),
            None,
            Vec::new(),
            diagnostics,
        );
    }
    tracker_module_inspection(
        TrackerModuleFormat::Xm,
        clean_ascii(&bytes[17..37]),
        read_le_u16_at(bytes, 68).map(usize::from),
        read_le_u16_at(bytes, 70).map(usize::from),
        Vec::new(),
        read_le_u16_at(bytes, 72).map(usize::from),
        Vec::new(),
        diagnostics,
    )
}

fn inspect_s3m_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 96 || bytes.get(44..48) != Some(&b"SCRM"[..]) {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "S3M data is missing the SCRM signature",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::S3m,
            None,
            None,
            None,
            Vec::new(),
            None,
            Vec::new(),
            diagnostics,
        );
    }
    let channels = bytes[64..96]
        .iter()
        .filter(|channel| **channel < 16)
        .count();
    tracker_module_inspection(
        TrackerModuleFormat::S3m,
        clean_ascii(&bytes[0..28]),
        Some(channels),
        read_le_u16_at(bytes, 36).map(usize::from),
        Vec::new(),
        read_le_u16_at(bytes, 34).map(usize::from),
        Vec::new(),
        diagnostics,
    )
}

fn inspect_it_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 192 || !bytes.starts_with(b"IMPM") {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "IT data is missing the IMPM signature",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::It,
            None,
            None,
            None,
            Vec::new(),
            None,
            Vec::new(),
            diagnostics,
        );
    }
    let channels = bytes[64..128].iter().filter(|pan| **pan != 0xff).count();
    tracker_module_inspection(
        TrackerModuleFormat::It,
        clean_ascii(&bytes[4..30]),
        Some(channels),
        read_le_u16_at(bytes, 38).map(usize::from),
        Vec::new(),
        read_le_u16_at(bytes, 34).map(usize::from),
        Vec::new(),
        diagnostics,
    )
}

#[allow(clippy::too_many_arguments)]
fn tracker_module_inspection(
    format: TrackerModuleFormat,
    title: Option<String>,
    channels: Option<usize>,
    patterns: Option<usize>,
    samples: Vec<TrackerModuleSampleInfo>,
    instrument_count: Option<usize>,
    effect_commands: Vec<u8>,
    diagnostics: Vec<TrackerModuleDiagnostic>,
) -> TrackerModuleInspection {
    TrackerModuleInspection {
        format,
        title,
        channels,
        patterns,
        samples,
        instrument_count,
        effect_commands,
        diagnostics,
        recommendation: TrackerModuleRecommendation::SampleExtractionOnly,
    }
}

fn legacy_module_diagnostics() -> Vec<TrackerModuleDiagnostic> {
    vec![
        tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::UnsupportedTimingSemantics,
            "legacy tracker tick tempo, speed changes, and row effects do not map losslessly to Salieri row timing",
        ),
        tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::UnsupportedEffectMemory,
            "legacy tracker effect memory and per-channel playback state are not represented in Salieri pattern cells",
        ),
        tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::EffectDecodeIncomplete,
            "probe reports effect command numbers but does not implement player-compatible effect semantics",
        ),
    ]
}

fn tracker_module_diagnostic(
    kind: TrackerModuleDiagnosticKind,
    message: impl Into<String>,
) -> TrackerModuleDiagnostic {
    TrackerModuleDiagnostic {
        kind,
        message: message.into(),
    }
}

fn clean_ascii(bytes: &[u8]) -> Option<String> {
    let text = bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0 && (byte.is_ascii_graphic() || *byte == b' '))
        .map(char::from)
        .collect::<String>();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[must_use]
pub fn inspect_xrns(bytes: &[u8]) -> XrnsInspection {
    let mut inspection = XrnsInspection {
        is_zip: false,
        song_xml_path: None,
        archive_entries: Vec::new(),
        sample_payloads: Vec::new(),
        tracks: Vec::new(),
        patterns: Vec::new(),
        instruments: Vec::new(),
        device_chains: Vec::new(),
        diagnostics: Vec::new(),
    };

    let entries = match parse_zip_entries(bytes) {
        Ok(entries) => entries,
        Err(message) => {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::MalformedArchive,
                XrnsDiagnosticSeverity::Error,
                None,
                message,
            ));
            return inspection;
        }
    };

    inspection.is_zip = true;
    let mut song_xml = None;
    for entry in &entries {
        inspection.archive_entries.push(XrnsArchiveEntry {
            path: entry.path.clone(),
            compressed_size: entry.compressed_size,
            uncompressed_size: entry.uncompressed_size,
            compression_method: entry.compression_method,
            encrypted: entry.encrypted,
        });

        if entry.encrypted {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::EncryptedArchive,
                XrnsDiagnosticSeverity::Error,
                Some(entry.path.clone()),
                format!("encrypted XRNS entry is unsupported: {}", entry.path),
            ));
        }
        if is_nested_archive_path(&entry.path) {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::NestedArchive,
                XrnsDiagnosticSeverity::Warning,
                Some(entry.path.clone()),
                format!("nested archive entry will not be imported: {}", entry.path),
            ));
        }
        if let Some(sample) = sample_payload(entry) {
            if !sample.supported {
                inspection.diagnostics.push(xrns_diagnostic(
                    XrnsDiagnosticKind::UnsupportedSampleFormat,
                    XrnsDiagnosticSeverity::Warning,
                    Some(entry.path.clone()),
                    format!("sample payload is not a supported WAV file: {}", entry.path),
                ));
            }
            inspection.sample_payloads.push(sample);
        }
        if entry.path == "Song.xml" {
            inspection.song_xml_path = Some(entry.path.clone());
            song_xml = Some(entry);
        }
    }

    let Some(song_xml) = song_xml else {
        inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::MissingSongXml,
            XrnsDiagnosticSeverity::Error,
            None,
            "XRNS archive does not contain root Song.xml",
        ));
        return inspection;
    };

    if song_xml.compression_method != 0 {
        inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::UnsupportedCompression,
            XrnsDiagnosticSeverity::Error,
            Some(song_xml.path.clone()),
            format!(
                "Song.xml uses unsupported ZIP compression method {}",
                song_xml.compression_method
            ),
        ));
        return inspection;
    }

    match std::str::from_utf8(song_xml.data) {
        Ok(xml) => inspect_song_xml(xml, &mut inspection),
        Err(error) => inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::MalformedSongXml,
            XrnsDiagnosticSeverity::Error,
            Some(song_xml.path.clone()),
            format!("Song.xml is not valid UTF-8: {error}"),
        )),
    }

    inspection
}

#[must_use]
pub fn import_xrns(bytes: &[u8]) -> XrnsImportReport {
    let inspection = inspect_xrns(bytes);
    let mut diagnostics = inspection.diagnostics.clone();
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == XrnsDiagnosticSeverity::Error)
    {
        return XrnsImportReport {
            song: None,
            inspection,
            diagnostics,
        };
    }

    let entries = match parse_zip_entries(bytes) {
        Ok(entries) => entries,
        Err(message) => {
            diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::MalformedArchive,
                XrnsDiagnosticSeverity::Error,
                None,
                message,
            ));
            return XrnsImportReport {
                song: None,
                inspection,
                diagnostics,
            };
        }
    };
    let song_xml = entries
        .iter()
        .find(|entry| entry.path == "Song.xml" && entry.compression_method == 0)
        .and_then(|entry| std::str::from_utf8(entry.data).ok());
    let Some(song_xml) = song_xml else {
        return XrnsImportReport {
            song: None,
            inspection,
            diagnostics,
        };
    };

    let Some(model) = parse_xrns_import_model(song_xml, &mut diagnostics) else {
        return XrnsImportReport {
            song: None,
            inspection,
            diagnostics,
        };
    };

    let song = build_song_from_xrns_model(&model, &inspection, &mut diagnostics);
    let song = match song {
        Some(song) => match song.validate() {
            Ok(()) => Some(song),
            Err(error) => {
                diagnostics.push(xrns_diagnostic(
                    XrnsDiagnosticKind::ValidationFailed,
                    XrnsDiagnosticSeverity::Error,
                    None,
                    format!("imported XRNS project failed validation: {error}"),
                ));
                None
            }
        },
        None => None,
    };

    XrnsImportReport {
        song,
        inspection,
        diagnostics,
    }
}

#[derive(Debug, Clone, Default)]
struct XrnsImportModel {
    tracks: Vec<XrnsImportTrack>,
    patterns: Vec<XrnsImportPattern>,
    instruments: Vec<String>,
    sequence: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
struct XrnsImportTrack {
    name: Option<String>,
    gain: Option<f32>,
    pan: Option<f32>,
    effects: Vec<EffectDevice>,
}

#[derive(Debug, Clone, Default)]
struct XrnsImportPattern {
    rows: Option<usize>,
    cells: Vec<XrnsImportCell>,
}

#[derive(Debug, Clone)]
struct XrnsImportCell {
    track: usize,
    row: usize,
    cell: PatternCell,
}

#[derive(Debug, Clone)]
struct PendingXrnsLine {
    track: usize,
    row: Option<usize>,
    cell: PatternCell,
    effect_code: Option<u8>,
    effect_value: Option<u8>,
}

fn parse_xrns_import_model(
    xml: &str,
    diagnostics: &mut Vec<XrnsDiagnostic>,
) -> Option<XrnsImportModel> {
    let events = match parse_xml_events(xml) {
        Ok(events) => events,
        Err(message) => {
            diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::MalformedSongXml,
                XrnsDiagnosticSeverity::Error,
                Some("Song.xml".to_string()),
                message,
            ));
            return None;
        }
    };

    let mut model = XrnsImportModel::default();
    let mut stack = Vec::<String>::new();
    let mut current_track: Option<XrnsImportTrack> = None;
    let mut current_instrument: Option<String> = None;
    let mut current_pattern: Option<XrnsImportPattern> = None;
    let mut current_pattern_track: Option<usize> = None;
    let mut pattern_track_line_counts = Vec::<usize>::new();
    let mut current_line: Option<PendingXrnsLine> = None;
    let mut next_effect_device_id = 1_u32;

    for event in events {
        match event {
            XmlEvent::Start(name) => {
                let in_pattern = current_pattern.is_some();
                if name == "Track" && stack_contains(&stack, "Tracks") && !in_pattern {
                    current_track = Some(XrnsImportTrack::default());
                } else if name == "Instrument" && stack_contains(&stack, "Instruments") {
                    current_instrument = Some(String::new());
                } else if name == "Pattern" && stack_contains(&stack, "Patterns") {
                    current_pattern = Some(XrnsImportPattern::default());
                    current_pattern_track = None;
                    pattern_track_line_counts.clear();
                } else if name == "Track" && in_pattern {
                    let track = current_pattern_track.map_or(0, |track| track + 1);
                    current_pattern_track = Some(track);
                    if pattern_track_line_counts.len() <= track {
                        pattern_track_line_counts.resize(track + 1, 0);
                    }
                } else if name == "Line" && current_pattern.is_some() {
                    let track = current_pattern_track.unwrap_or(0);
                    current_line = Some(PendingXrnsLine {
                        track,
                        row: None,
                        cell: PatternCell::default(),
                        effect_code: None,
                        effect_value: None,
                    });
                }
                stack.push(name);
            }
            XmlEvent::End(name) => {
                if name == "Track" && current_line.is_none() {
                    if current_pattern.is_some() {
                        current_pattern_track = None;
                    } else if let Some(track) = current_track.take() {
                        model.tracks.push(track);
                    }
                } else if name == "Instrument" {
                    if let Some(name) = current_instrument.take() {
                        model.instruments.push(if name.trim().is_empty() {
                            format!("Instrument {:02}", model.instruments.len() + 1)
                        } else {
                            name
                        });
                    }
                } else if name == "Pattern" {
                    if let Some(pattern) = current_pattern.take() {
                        model.patterns.push(pattern);
                    }
                    current_pattern_track = None;
                } else if name == "Line" {
                    if let (Some(mut pattern), Some(mut line)) =
                        (current_pattern.take(), current_line.take())
                    {
                        let row = line.row.unwrap_or_else(|| {
                            let count = pattern_track_line_counts
                                .get_mut(line.track)
                                .expect("line counter exists");
                            let row = *count;
                            *count += 1;
                            row
                        });
                        line.row = Some(row);
                        pattern.cells.push(XrnsImportCell {
                            track: line.track,
                            row,
                            cell: line.cell,
                        });
                        current_pattern = Some(pattern);
                    }
                } else if name == "Effect" {
                    if let Some(line) = &mut current_line {
                        if let Some(code) = line.effect_code.take() {
                            let command = TrackerCommand {
                                code,
                                value: line.effect_value.take().unwrap_or(0),
                            };
                            if !matches!(
                                code,
                                TrackerCommand::DELAY_CODE | TrackerCommand::RETRIGGER_CODE
                            ) {
                                diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::UnsupportedEffectCommand,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, &name)),
                                    format!(
                                        "unknown Renoise effect command {} preserved as tracker command",
                                        code as char
                                    ),
                                ));
                            }
                            if line.cell.command.is_some() {
                                diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::DroppedExtraEffectColumn,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, &name)),
                                    "extra XRNS effect column was dropped",
                                ));
                            } else {
                                line.cell.command = Some(command);
                            }
                        }
                    }
                }
                let _ = stack.pop();
            }
            XmlEvent::Text(text) => {
                let current = stack.last().map(String::as_str).unwrap_or_default();
                if let Some(line) = &mut current_line {
                    apply_xrns_line_text(current, &text, line, diagnostics, &stack);
                    continue;
                }
                if let Some(track) = &mut current_track {
                    match current {
                        "Name" => track.name = Some(text),
                        "Gain" | "Volume" => track.gain = parse_float(&text),
                        "Pan" | "Panning" => track.pan = parse_float(&text),
                        "Device" | "Type" => {
                            if let Some(effect) =
                                effect_device_from_name(next_effect_device_id, &text)
                            {
                                next_effect_device_id += 1;
                                track.effects.push(effect);
                            } else {
                                diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, current)),
                                    format!("unsupported Renoise device: {text}"),
                                ));
                            }
                        }
                        _ => {}
                    }
                } else if let Some(instrument) = &mut current_instrument {
                    if current == "Name" {
                        *instrument = text;
                    }
                } else if let Some(pattern) = &mut current_pattern {
                    if matches!(current, "NumberOfLines" | "Lines") {
                        pattern.rows = text.parse::<usize>().ok().or(pattern.rows);
                    }
                } else if current == "Pattern" && stack_contains(&stack, "SequenceEntry") {
                    if let Ok(pattern) = text.parse::<usize>() {
                        model.sequence.push(pattern);
                    }
                }
            }
        }
    }

    Some(model)
}

fn apply_xrns_line_text(
    current: &str,
    text: &str,
    line: &mut PendingXrnsLine,
    diagnostics: &mut Vec<XrnsDiagnostic>,
    stack: &[String],
) {
    match current {
        "Index" | "Row" => line.row = text.parse::<usize>().ok(),
        "Note" => line.cell.note = parse_xrns_note(text),
        "Velocity" => line.cell.velocity = parse_u8_value(text),
        "Instrument" => {
            line.cell.instrument = parse_u32_value(text).map(InstrumentId);
        }
        "Volume" => line.cell.volume = parse_u8_value(text).map(|value| value.min(127)),
        "Pan" | "Panning" => line.cell.pan = parse_u8_value(text).map(|value| value.min(127)),
        "Delay" => line.cell.delay = parse_u8_value(text),
        "Code" | "Command" => {
            line.effect_code = text
                .as_bytes()
                .first()
                .copied()
                .map(|byte| byte.to_ascii_uppercase());
        }
        "Value" => line.effect_value = parse_u8_value(text),
        "SourceTick" | "SourceTime" => diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::TimingQuantized,
            XrnsDiagnosticSeverity::Warning,
            Some(xml_location(stack, current)),
            "XRNS timing was quantized to the nearest Salieri row",
        )),
        _ => {}
    }
}

fn build_song_from_xrns_model(
    model: &XrnsImportModel,
    inspection: &XrnsInspection,
    diagnostics: &mut Vec<XrnsDiagnostic>,
) -> Option<Song> {
    let mut song = Song::empty();
    let track_count = model.tracks.len().max(1);
    while song.tracks.len() < track_count {
        song.create_track();
    }
    while song.tracks.len() > track_count {
        song.delete_track(song.tracks.len() - 1).ok()?;
    }

    for (index, track) in model.tracks.iter().enumerate() {
        if let Some(name) = &track.name {
            song.rename_track(index, name).ok()?;
        }
        if let Some(gain) = track.gain {
            let _ = song.set_track_mixer_gain(index, gain.max(0.0));
        }
        if let Some(pan) = track.pan {
            let _ = song.set_track_mixer_pan(index, pan.clamp(-1.0, 1.0));
        }
        let track_id = song.tracks[index].id;
        if let Some(mixer) = song
            .mixer
            .tracks
            .iter_mut()
            .find(|mixer| mixer.track == track_id)
        {
            mixer.effects = track.effects.clone();
        }
    }

    let mut imported_instruments = HashSet::new();
    for sample in inspection
        .sample_payloads
        .iter()
        .filter(|sample| sample.supported)
    {
        let sample_id = song.upsert_sample_reference(&sample.path, sample_name(&sample.path));
        if let Ok(instrument) = song.upsert_sample_instrument(sample_id) {
            imported_instruments.insert(instrument);
        }
    }

    let pattern_count = model.patterns.len().max(1);
    while song.patterns.len() < pattern_count {
        let rows = model
            .patterns
            .get(song.patterns.len())
            .and_then(|pattern| pattern.rows)
            .unwrap_or(64);
        song.create_pattern(rows.max(1));
    }
    while song.patterns.len() > pattern_count {
        song.delete_pattern(song.patterns.len() - 1).ok()?;
    }

    for (index, pattern) in model.patterns.iter().enumerate() {
        let rows = pattern.rows.unwrap_or(64).max(1);
        song.resize_pattern(index, rows).ok()?;
        for imported in &pattern.cells {
            if imported.track >= song.tracks.len() || imported.row >= rows {
                continue;
            }
            let mut cell = imported.cell.clone();
            if let Some(instrument) = cell.instrument {
                if !imported_instruments.contains(&instrument) {
                    diagnostics.push(xrns_diagnostic(
                        XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                        XrnsDiagnosticSeverity::Warning,
                        Some(format!("Pattern {index} row {}", imported.row)),
                        format!(
                            "instrument {:?} has no supported sample payload",
                            instrument
                        ),
                    ));
                    cell.instrument = None;
                }
            }
            song.pattern_mut(index)?
                .set_cell(imported.row, imported.track, cell)
                .ok()?;
        }
    }

    if !model.sequence.is_empty() {
        song.sequence.clear();
        for pattern in &model.sequence {
            if let Some(pattern_id) = song.patterns.get(*pattern).map(|pattern| pattern.id) {
                song.sequence.push(pattern_id);
            } else {
                diagnostics.push(xrns_diagnostic(
                    XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                    XrnsDiagnosticSeverity::Warning,
                    Some("PatternSequence".to_string()),
                    format!("sequence references missing pattern {pattern}"),
                ));
            }
        }
        if song.sequence.is_empty() {
            song.sequence.push(song.patterns[0].id);
        }
    }

    Some(song)
}

fn parse_xrns_note(value: &str) -> Option<NoteEvent> {
    let value = value.trim();
    if value.is_empty() || value == "---" {
        return None;
    }
    if matches!(value.to_ascii_uppercase().as_str(), "OFF" | "NOTE_OFF") {
        return Some(NoteEvent::NoteOff);
    }
    if matches!(value.to_ascii_uppercase().as_str(), "CUT" | "NOTE_CUT") {
        return Some(NoteEvent::NoteCut);
    }
    if let Some(pitch) = parse_u8_value(value) {
        return Some(NoteEvent::Note {
            pitch: pitch.min(127),
        });
    }
    parse_note_name(value).map(|pitch| NoteEvent::Note { pitch })
}

fn parse_note_name(value: &str) -> Option<u8> {
    let value = value.trim().to_ascii_uppercase();
    let bytes = value.as_bytes();
    let semitone = match bytes.first().copied()? as char {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut index = 1;
    let accidental = match bytes.get(index).copied().map(char::from) {
        Some('#') => {
            index += 1;
            1
        }
        Some('B') => {
            index += 1;
            -1
        }
        Some('-') => {
            index += 1;
            0
        }
        _ => 0,
    };
    let octave = value.get(index..)?.parse::<i16>().ok()?;
    let pitch = (octave + 1) * 12 + semitone + accidental;
    u8::try_from(pitch).ok().filter(|pitch| *pitch <= 127)
}

fn parse_u8_value(value: &str) -> Option<u8> {
    parse_u32_value(value).and_then(|value| u8::try_from(value).ok())
}

fn parse_u32_value(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    value
        .parse::<u32>()
        .ok()
        .or_else(|| u32::from_str_radix(value.trim_start_matches("0x"), 16).ok())
}

fn parse_float(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn effect_device_from_name(id: u32, name: &str) -> Option<EffectDevice> {
    let normalized = name.to_ascii_lowercase();
    if normalized.contains("gain") || normalized.contains("gainer") || normalized.contains("volume")
    {
        Some(EffectDevice::gain(id, 1.0))
    } else if normalized.contains("pan") {
        Some(EffectDevice::pan(id, 0.0))
    } else {
        None
    }
}

fn sample_name(path: &str) -> String {
    path.rsplit('/')
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(path)
        .to_string()
}

#[derive(Debug, Clone)]
struct ZipEntryRef<'a> {
    path: String,
    compressed_size: u32,
    uncompressed_size: u32,
    compression_method: u16,
    encrypted: bool,
    data: &'a [u8],
}

fn parse_zip_entries(bytes: &[u8]) -> Result<Vec<ZipEntryRef<'_>>, String> {
    if bytes.len() < 4 || read_le_u32_at(bytes, 0) != Some(ZIP_LOCAL_FILE_HEADER) {
        return Err("XRNS data is not a ZIP local-file-header stream".to_string());
    }

    let mut entries = Vec::new();
    let mut position = 0_usize;
    while position + 4 <= bytes.len() {
        let Some(signature) = read_le_u32_at(bytes, position) else {
            break;
        };
        if signature == ZIP_CENTRAL_DIRECTORY_HEADER || signature == ZIP_END_OF_CENTRAL_DIRECTORY {
            break;
        }
        if signature != ZIP_LOCAL_FILE_HEADER {
            return Err(format!("unexpected ZIP signature 0x{signature:08X}"));
        }
        if position + 30 > bytes.len() {
            return Err("truncated ZIP local file header".to_string());
        }

        let flags = read_le_u16_at(bytes, position + 6).expect("bounds checked");
        let compression_method = read_le_u16_at(bytes, position + 8).expect("bounds checked");
        let compressed_size = read_le_u32_at(bytes, position + 18).expect("bounds checked");
        let uncompressed_size = read_le_u32_at(bytes, position + 22).expect("bounds checked");
        let name_len = usize::from(read_le_u16_at(bytes, position + 26).expect("bounds checked"));
        let extra_len = usize::from(read_le_u16_at(bytes, position + 28).expect("bounds checked"));
        let name_start = position + 30;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or_else(|| "ZIP entry name length overflow".to_string())?;
        let data_start = name_end
            .checked_add(extra_len)
            .ok_or_else(|| "ZIP extra field length overflow".to_string())?;
        let data_end = data_start
            .checked_add(compressed_size as usize)
            .ok_or_else(|| "ZIP entry data length overflow".to_string())?;
        if data_end > bytes.len() {
            return Err("truncated ZIP entry data".to_string());
        }

        let path = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|error| format!("ZIP entry name is not valid UTF-8: {error}"))?
            .replace('\\', "/");
        entries.push(ZipEntryRef {
            path,
            compressed_size,
            uncompressed_size,
            compression_method,
            encrypted: flags & 0x0001 != 0,
            data: &bytes[data_start..data_end],
        });
        position = data_end;
    }

    if entries.is_empty() {
        Err("XRNS ZIP archive has no entries".to_string())
    } else {
        Ok(entries)
    }
}

fn inspect_song_xml(xml: &str, inspection: &mut XrnsInspection) {
    match parse_xml_events(xml) {
        Ok(events) => inspect_xml_events(&events, inspection),
        Err(message) => inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::MalformedSongXml,
            XrnsDiagnosticSeverity::Error,
            Some("Song.xml".to_string()),
            message,
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum XmlEvent {
    Start(String),
    End(String),
    Text(String),
}

fn parse_xml_events(xml: &str) -> Result<Vec<XmlEvent>, String> {
    let mut events = Vec::new();
    let mut stack = Vec::new();
    let mut position = 0_usize;
    while let Some(relative_start) = xml[position..].find('<') {
        let start = position + relative_start;
        push_xml_text(&xml[position..start], &mut events);
        let Some(relative_end) = xml[start..].find('>') else {
            return Err("Song.xml contains an unterminated tag".to_string());
        };
        let end = start + relative_end;
        let raw_tag = xml[start + 1..end].trim();
        position = end + 1;

        if raw_tag.is_empty()
            || raw_tag.starts_with('?')
            || raw_tag.starts_with('!')
            || raw_tag.starts_with("!--")
        {
            continue;
        }
        if let Some(stripped) = raw_tag.strip_prefix('/') {
            let name = xml_tag_name(stripped);
            let Some(open) = stack.pop() else {
                return Err(format!("Song.xml closes unopened tag {name}"));
            };
            if open != name {
                return Err(format!("Song.xml closes tag {name} while {open} is open"));
            }
            events.push(XmlEvent::End(name));
        } else {
            let self_closing = raw_tag.ends_with('/');
            let name = xml_tag_name(raw_tag.trim_end_matches('/'));
            events.push(XmlEvent::Start(name.clone()));
            if self_closing {
                events.push(XmlEvent::End(name));
            } else {
                stack.push(name);
            }
        }
    }
    push_xml_text(&xml[position..], &mut events);

    if let Some(open) = stack.pop() {
        return Err(format!("Song.xml leaves tag {open} unclosed"));
    }
    Ok(events)
}

fn push_xml_text(text: &str, events: &mut Vec<XmlEvent>) {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        events.push(XmlEvent::Text(decode_xml_entities(trimmed)));
    }
}

fn xml_tag_name(raw_tag: &str) -> String {
    raw_tag
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches('/')
        .to_string()
}

fn inspect_xml_events(events: &[XmlEvent], inspection: &mut XrnsInspection) {
    let mut stack = Vec::<String>::new();
    let mut current_track: Option<XrnsTrackInfo> = None;
    let mut current_pattern: Option<XrnsPatternInfo> = None;
    let mut current_instrument: Option<XrnsInstrumentInfo> = None;
    let mut current_device_chain: Option<XrnsDeviceChainInfo> = None;
    let mut current_device: Option<String> = None;
    let mut reported_features = HashSet::new();

    for event in events {
        match event {
            XmlEvent::Start(name) => {
                if is_unsupported_feature_tag(name) && reported_features.insert(name.clone()) {
                    inspection.diagnostics.push(xrns_diagnostic(
                        XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                        XrnsDiagnosticSeverity::Warning,
                        Some(xml_location(&stack, name)),
                        format!("unsupported Renoise feature tag: {name}"),
                    ));
                }

                if name == "Track" && stack_contains(&stack, "Tracks") {
                    current_track = Some(XrnsTrackInfo {
                        index: inspection.tracks.len(),
                        name: None,
                    });
                } else if name == "Pattern" && stack_contains(&stack, "Patterns") {
                    current_pattern = Some(XrnsPatternInfo {
                        index: inspection.patterns.len(),
                        rows: None,
                    });
                } else if name == "Instrument" && stack_contains(&stack, "Instruments") {
                    current_instrument = Some(XrnsInstrumentInfo {
                        index: inspection.instruments.len(),
                        name: None,
                    });
                } else if name == "DeviceChain" {
                    current_device_chain = Some(XrnsDeviceChainInfo {
                        location: xml_location(&stack, name),
                        devices: Vec::new(),
                    });
                } else if name == "Device" && current_device_chain.is_some() {
                    current_device = Some(String::new());
                }
                stack.push(name.clone());
            }
            XmlEvent::End(name) => {
                if name == "Track" {
                    if let Some(track) = current_track.take() {
                        inspection.tracks.push(track);
                    }
                } else if name == "Pattern" {
                    if let Some(pattern) = current_pattern.take() {
                        inspection.patterns.push(pattern);
                    }
                } else if name == "Instrument" {
                    if let Some(instrument) = current_instrument.take() {
                        inspection.instruments.push(instrument);
                    }
                } else if name == "Device" {
                    if let Some(device) = current_device.take() {
                        let device = device.trim();
                        if !device.is_empty() {
                            if !is_supported_native_device(device) {
                                inspection.diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, name)),
                                    format!("unsupported Renoise device: {device}"),
                                ));
                            }
                            if let Some(chain) = &mut current_device_chain {
                                chain.devices.push(device.to_string());
                            }
                        }
                    }
                } else if name == "DeviceChain" {
                    if let Some(chain) = current_device_chain.take() {
                        inspection.device_chains.push(chain);
                    }
                }
                let _ = stack.pop();
            }
            XmlEvent::Text(text) => {
                let current = stack.last().map(String::as_str).unwrap_or_default();
                if current == "Name" {
                    if let Some(track) = &mut current_track {
                        track.name = Some(text.clone());
                    } else if let Some(instrument) = &mut current_instrument {
                        instrument.name = Some(text.clone());
                    } else if let Some(device) = &mut current_device {
                        *device = text.clone();
                    }
                } else if matches!(current, "NumberOfLines" | "Lines") {
                    if let Some(pattern) = &mut current_pattern {
                        if pattern.rows.is_none() {
                            pattern.rows = text.parse::<usize>().ok();
                        }
                    }
                } else if matches!(current, "Type" | "PluginIdentifier") {
                    if let Some(device) = &mut current_device {
                        *device = text.clone();
                    }
                }
            }
        }
    }
}

fn xrns_diagnostic(
    kind: XrnsDiagnosticKind,
    severity: XrnsDiagnosticSeverity,
    location: Option<String>,
    message: impl Into<String>,
) -> XrnsDiagnostic {
    XrnsDiagnostic {
        kind,
        severity,
        location,
        message: message.into(),
    }
}

fn sample_payload(entry: &ZipEntryRef<'_>) -> Option<XrnsSamplePayload> {
    let extension = entry.path.rsplit('.').next()?.to_ascii_lowercase();
    let is_sample = matches!(
        extension.as_str(),
        "wav" | "aif" | "aiff" | "flac" | "mp3" | "ogg"
    );
    is_sample.then_some(XrnsSamplePayload {
        path: entry.path.clone(),
        format: extension.clone(),
        bytes: entry.uncompressed_size,
        supported: extension == "wav",
    })
}

fn is_nested_archive_path(path: &str) -> bool {
    path.rsplit('.')
        .next()
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "zip" | "xrns"))
}

fn is_unsupported_feature_tag(name: &str) -> bool {
    matches!(
        name,
        "PluginDevice"
            | "VstPlugin"
            | "VSTPlugin"
            | "AudioUnitPlugin"
            | "AuPlugin"
            | "MetaDevice"
            | "AutomationEnvelope"
            | "Phrase"
            | "Phrases"
    )
}

fn is_supported_native_device(device: &str) -> bool {
    let normalized = device.to_ascii_lowercase();
    normalized.contains("gain")
        || normalized.contains("gainer")
        || normalized.contains("volume")
        || normalized.contains("pan")
}

fn stack_contains(stack: &[String], name: &str) -> bool {
    stack.iter().any(|item| item == name)
}

fn xml_location(stack: &[String], name: &str) -> String {
    stack
        .iter()
        .chain(std::iter::once(&name.to_string()))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("/")
}

fn decode_xml_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn read_le_u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let bytes = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_le_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let bytes = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn track_for_channel(
    song: &mut Song,
    channel_tracks: &mut HashMap<u8, usize>,
    channel: u8,
) -> usize {
    if let Some(track) = channel_tracks.get(&channel) {
        return *track;
    }
    let track = song
        .tracks
        .iter()
        .position(|track| track.midi_channel == channel)
        .unwrap_or_else(|| {
            song.create_track();
            let index = song.tracks.len() - 1;
            song.tracks[index].midi_channel = channel;
            index
        });
    channel_tracks.insert(channel, track);
    track
}

fn ensure_pattern_row(song: &mut Song, row: usize) -> Result<(), InteropError> {
    let row_count = song
        .current_pattern()
        .expect("default song has a pattern")
        .row_count();
    if row >= row_count {
        song.resize_pattern(0, row + 1)
            .map_err(|_| InteropError::InvalidMidiTrack)?;
    }
    Ok(())
}

fn ticks_to_row(tick: u64, ticks_per_quarter: u64, lines_per_beat: u8) -> usize {
    let lines_per_beat = u64::from(lines_per_beat.max(1));
    ((tick.saturating_mul(lines_per_beat) + ticks_per_quarter / 2) / ticks_per_quarter) as usize
}

fn tempo_to_bpm(tempo_micros_per_quarter: u32) -> u16 {
    if tempo_micros_per_quarter == 0 {
        return 120;
    }
    (60_000_000 / tempo_micros_per_quarter).clamp(1, u32::from(u16::MAX)) as u16
}

fn write_var_len(mut value: u64, bytes: &mut Vec<u8>) {
    let mut buffer = [0_u8; 10];
    let mut index = buffer.len() - 1;
    buffer[index] = (value & 0x7f) as u8;
    value >>= 7;
    while value > 0 {
        index -= 1;
        buffer[index] = ((value & 0x7f) as u8) | 0x80;
        value >>= 7;
    }
    bytes.extend_from_slice(&buffer[index..]);
}

fn read_var_len(cursor: &mut Cursor<'_>) -> Result<u64, InteropError> {
    let mut value = 0_u64;
    for _ in 0..4 {
        let byte = cursor.read_u8()?;
        value = (value << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(InteropError::InvalidMidiTrack)
}

fn read_tempo(cursor: &mut Cursor<'_>) -> Result<u32, InteropError> {
    let bytes = cursor.read_exact(3)?;
    Ok((u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    const fn position(&self) -> usize {
        self.position
    }

    fn rewind(&mut self, count: usize) {
        self.position = self.position.saturating_sub(count);
    }

    fn skip(&mut self, count: usize) -> Result<(), InteropError> {
        self.read_exact(count).map(|_| ())
    }

    fn read_u8(&mut self) -> Result<u8, InteropError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, InteropError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, InteropError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_exact(&mut self, count: usize) -> Result<&'a [u8], InteropError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(InteropError::TruncatedMidiData)?;
        if end > self.bytes.len() {
            return Err(InteropError::TruncatedMidiData);
        }
        let bytes = &self.bytes[self.position..end];
        self.position = end;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_pattern_to_standard_midi_file() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 1, NoteEvent::Note { pitch: 60 }, 100)
            .expect("set note");

        let bytes = export_pattern_smf(&song, MidiExportOptions::default()).expect("export");

        assert_eq!(&bytes[0..4], MTHD);
        assert_eq!(&bytes[14..18], MTRK);
        assert!(bytes.windows(3).any(|window| window == [0x90, 0x3c, 0x64]));
        assert!(bytes.windows(3).any(|window| window == [0x80, 0x3c, 0x00]));
    }

    #[test]
    fn imports_representative_format_zero_fixture() {
        let bytes = hex_fixture(include_str!("../../../fixtures/midi/simple-format0.hex"));
        let song = import_smf(&bytes).expect("import");
        let pattern = song.current_pattern().expect("pattern");
        let cell = pattern.cell(0, 1).expect("cell");

        assert_eq!(song.transport.bpm, 120);
        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
        assert_eq!(cell.velocity, Some(100));
    }

    #[test]
    fn exported_subset_can_be_imported_back() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(8, 1, NoteEvent::Note { pitch: 64 }, 90)
            .expect("set note");

        let bytes = export_pattern_smf(&song, MidiExportOptions::default()).expect("export");
        let imported = import_smf(&bytes).expect("import");
        let cell = imported
            .current_pattern()
            .expect("pattern")
            .cell(8, 1)
            .expect("cell");

        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 64 }));
        assert_eq!(cell.velocity, Some(90));
    }

    #[test]
    fn unsupported_formats_fail_clearly() {
        let mut bytes = hex_fixture(include_str!("../../../fixtures/midi/simple-format0.hex"));
        bytes[9] = 1;

        assert!(matches!(
            import_smf(&bytes),
            Err(InteropError::UnsupportedMidiFormat(1))
        ));
        assert!(matches!(
            import_tracker_module(&[], TrackerModuleFormat::Xm),
            Err(InteropError::UnsupportedTrackerModule(
                TrackerModuleFormat::Xm
            ))
        ));
    }

    #[test]
    fn inspects_mod_metadata_samples_and_effect_commands() {
        let mut bytes = vec![0_u8; 1084 + 64 * 4 * 4];
        bytes[0..10].copy_from_slice(b"Test Song ");
        bytes[20..24].copy_from_slice(b"Kick");
        bytes[42..44].copy_from_slice(&4_u16.to_be_bytes());
        bytes[950] = 1;
        bytes[952] = 0;
        bytes[1080..1084].copy_from_slice(b"M.K.");
        bytes[1084 + 2] = 0x0f;
        bytes[1084 + 3] = 0x01;

        let inspection = inspect_tracker_module(&bytes, TrackerModuleFormat::Mod);

        assert_eq!(inspection.title.as_deref(), Some("Test Song"));
        assert_eq!(inspection.channels, Some(4));
        assert_eq!(inspection.patterns, Some(1));
        assert_eq!(inspection.instrument_count, Some(31));
        assert_eq!(inspection.samples[0].name.as_deref(), Some("Kick"));
        assert_eq!(inspection.samples[0].length_bytes, Some(8));
        assert_eq!(inspection.effect_commands, vec![0x0f]);
        assert_eq!(
            inspection.recommendation,
            TrackerModuleRecommendation::SampleExtractionOnly
        );
        assert!(inspection.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == TrackerModuleDiagnosticKind::UnsupportedTimingSemantics
        }));
    }

    #[test]
    fn inspects_xm_s3m_and_it_headers_without_full_import() {
        let mut xm = vec![0_u8; 80];
        xm[0..17].copy_from_slice(b"Extended Module: ");
        xm[17..25].copy_from_slice(b"XM Song ");
        xm[68..70].copy_from_slice(&8_u16.to_le_bytes());
        xm[70..72].copy_from_slice(&3_u16.to_le_bytes());
        xm[72..74].copy_from_slice(&2_u16.to_le_bytes());
        let xm = inspect_tracker_module(&xm, TrackerModuleFormat::Xm);
        assert_eq!(xm.title.as_deref(), Some("XM Song"));
        assert_eq!(xm.channels, Some(8));
        assert_eq!(xm.patterns, Some(3));
        assert_eq!(xm.instrument_count, Some(2));

        let mut s3m = vec![0xff_u8; 96];
        s3m[0..8].copy_from_slice(b"S3M Song");
        s3m[32..34].copy_from_slice(&4_u16.to_le_bytes());
        s3m[34..36].copy_from_slice(&5_u16.to_le_bytes());
        s3m[36..38].copy_from_slice(&6_u16.to_le_bytes());
        s3m[44..48].copy_from_slice(b"SCRM");
        s3m[64] = 0;
        s3m[65] = 1;
        let s3m = inspect_tracker_module(&s3m, TrackerModuleFormat::S3m);
        assert_eq!(s3m.title.as_deref(), Some("S3M Song"));
        assert_eq!(s3m.channels, Some(2));
        assert_eq!(s3m.patterns, Some(6));
        assert_eq!(s3m.instrument_count, Some(5));

        let mut it = vec![0xff_u8; 192];
        it[0..4].copy_from_slice(b"IMPM");
        it[4..11].copy_from_slice(b"IT Song");
        it[34..36].copy_from_slice(&7_u16.to_le_bytes());
        it[38..40].copy_from_slice(&9_u16.to_le_bytes());
        it[64] = 0;
        it[65] = 32;
        let it = inspect_tracker_module(&it, TrackerModuleFormat::It);
        assert_eq!(it.title.as_deref(), Some("IT Song"));
        assert_eq!(it.channels, Some(2));
        assert_eq!(it.patterns, Some(9));
        assert_eq!(it.instrument_count, Some(7));
    }

    #[test]
    fn legacy_module_probe_reports_malformed_data() {
        let inspection = inspect_tracker_module(b"short", TrackerModuleFormat::Mod);

        assert!(inspection
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == TrackerModuleDiagnosticKind::MalformedModule }));
        assert!(inspection.samples.is_empty());
    }

    #[test]
    fn rejects_smpte_division() {
        let mut bytes = hex_fixture(include_str!("../../../fixtures/midi/simple-format0.hex"));
        bytes[12] = 0xe7;
        bytes[13] = 0x28;

        assert!(matches!(
            import_smf(&bytes),
            Err(InteropError::UnsupportedSmpteDivision(0xe728))
        ));
    }

    #[test]
    fn inspects_representative_xrns_archive() {
        let xml = r#"
<RenoiseSong>
  <Tracks>
    <Track><Name>Drums</Name></Track>
    <Track><Name>Bass</Name></Track>
  </Tracks>
  <PatternSequence><SequenceEntries><SequenceEntry><Pattern>0</Pattern></SequenceEntry></SequenceEntries></PatternSequence>
  <Patterns>
    <Pattern><NumberOfLines>64</NumberOfLines></Pattern>
  </Patterns>
  <Instruments>
    <Instrument><Name>Kick</Name></Instrument>
  </Instruments>
  <DeviceChain>
    <Device><Name>Gainer</Name></Device>
    <Device><Name>Comb Filter</Name></Device>
  </DeviceChain>
  <PluginDevice><Name>Third Party</Name></PluginDevice>
</RenoiseSong>
"#;
        let archive = xrns_archive([
            xrns_entry("Song.xml", xml.as_bytes()),
            xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
            xrns_entry("SampleData/Instrument01/Sample00.flac", b"fLaC"),
        ]);

        let inspection = inspect_xrns(&archive);

        assert!(inspection.is_zip);
        assert_eq!(inspection.song_xml_path.as_deref(), Some("Song.xml"));
        assert_eq!(inspection.tracks.len(), 2);
        assert_eq!(inspection.tracks[0].name.as_deref(), Some("Drums"));
        assert_eq!(inspection.patterns[0].rows, Some(64));
        assert_eq!(inspection.instruments[0].name.as_deref(), Some("Kick"));
        assert_eq!(inspection.sample_payloads.len(), 2);
        assert!(inspection.sample_payloads[0].supported);
        assert!(!inspection.sample_payloads[1].supported);
        assert_eq!(
            inspection.device_chains[0].devices,
            vec!["Gainer".to_string(), "Comb Filter".to_string()]
        );
        assert!(inspection.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == XrnsDiagnosticKind::UnsupportedRenoiseFeature
                && diagnostic.message.contains("Comb Filter")
        }));
        assert!(inspection
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == XrnsDiagnosticKind::UnsupportedSampleFormat }));
    }

    #[test]
    fn xrns_inspector_reports_missing_song_xml() {
        let archive = xrns_archive([xrns_entry("SampleData/Sample00.wav", b"RIFF....WAVE")]);
        let inspection = inspect_xrns(&archive);

        assert!(inspection.is_zip);
        assert!(inspection.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == XrnsDiagnosticKind::MissingSongXml
                && diagnostic.severity == XrnsDiagnosticSeverity::Error
        }));
    }

    #[test]
    fn xrns_inspector_reports_malformed_xml() {
        let archive = xrns_archive([xrns_entry(
            "Song.xml",
            b"<RenoiseSong><Tracks></RenoiseSong>",
        )]);
        let inspection = inspect_xrns(&archive);

        assert!(inspection.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == XrnsDiagnosticKind::MalformedSongXml
                && diagnostic.severity == XrnsDiagnosticSeverity::Error
        }));
    }

    #[test]
    fn xrns_inspector_reports_nested_and_encrypted_entries() {
        let archive = xrns_archive([
            xrns_entry("Song.xml", b"<RenoiseSong />"),
            XrnsTestEntry {
                path: "Embedded/inner.xrns",
                data: b"nested",
                flags: 0x0001,
                compression_method: 0,
            },
        ]);
        let inspection = inspect_xrns(&archive);

        assert!(inspection
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == XrnsDiagnosticKind::NestedArchive }));
        assert!(inspection.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == XrnsDiagnosticKind::EncryptedArchive
                && diagnostic.severity == XrnsDiagnosticSeverity::Error
        }));
    }

    #[test]
    fn xrns_inspector_reports_malformed_archive_and_compressed_song_xml() {
        let malformed = inspect_xrns(b"not a zip");
        assert!(!malformed.is_zip);
        assert!(malformed
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == XrnsDiagnosticKind::MalformedArchive }));

        let compressed = xrns_archive([XrnsTestEntry {
            path: "Song.xml",
            data: b"",
            flags: 0,
            compression_method: 8,
        }]);
        let inspection = inspect_xrns(&compressed);
        assert!(inspection
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == XrnsDiagnosticKind::UnsupportedCompression }));
    }

    #[test]
    fn imports_minimal_xrns_subset_to_valid_song() {
        let xml = r#"
<RenoiseSong>
  <Tracks>
    <Track><Name>Drums</Name><Gain>0.75</Gain><Pan>-0.25</Pan><Device>Gainer</Device></Track>
    <Track><Name>Bass</Name></Track>
  </Tracks>
  <PatternSequence>
    <SequenceEntry><Pattern>0</Pattern></SequenceEntry>
    <SequenceEntry><Pattern>1</Pattern></SequenceEntry>
  </PatternSequence>
  <Patterns>
    <Pattern>
      <NumberOfLines>8</NumberOfLines>
      <Tracks>
        <Track>
          <Line>
            <Index>0</Index>
            <Note>C-4</Note>
            <Velocity>100</Velocity>
            <Instrument>1</Instrument>
            <Volume>64</Volume>
            <Pan>127</Pan>
            <Delay>32</Delay>
            <Effect><Code>R</Code><Value>4</Value></Effect>
          </Line>
        </Track>
      </Tracks>
    </Pattern>
    <Pattern><NumberOfLines>4</NumberOfLines></Pattern>
  </Patterns>
  <Instruments><Instrument><Name>Kick</Name></Instrument></Instruments>
</RenoiseSong>
"#;
        let archive = xrns_archive([
            xrns_entry("Song.xml", xml.as_bytes()),
            xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
        ]);

        let report = import_xrns(&archive);
        let song = report.song.expect("imported song");

        song.validate().expect("valid song");
        assert_eq!(song.tracks.len(), 2);
        assert_eq!(song.tracks[0].name, "Drums");
        assert_eq!(song.track_mixer_for_track(song.tracks[0].id).gain, 0.75);
        assert_eq!(song.track_mixer_for_track(song.tracks[0].id).pan, -0.25);
        assert_eq!(
            song.track_mixer_for_track(song.tracks[0].id).effects,
            vec![EffectDevice::gain(1, 1.0)]
        );
        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.patterns[0].row_count(), 8);
        assert_eq!(song.patterns[1].row_count(), 4);
        assert_eq!(
            song.sequence,
            vec![song.patterns[0].id, song.patterns[1].id]
        );

        let cell = song.patterns[0].cell(0, 0).expect("cell");
        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
        assert_eq!(cell.velocity, Some(100));
        assert_eq!(cell.instrument, Some(InstrumentId(1)));
        assert_eq!(cell.volume, Some(64));
        assert_eq!(cell.pan, Some(127));
        assert_eq!(cell.delay, Some(32));
        assert_eq!(cell.command, Some(TrackerCommand::retrigger(4)));
        assert_eq!(song.samples.len(), 1);
        assert_eq!(song.instruments.len(), 1);
    }

    #[test]
    fn xrns_import_reports_warnings_without_silent_drops() {
        let xml = r#"
<RenoiseSong>
  <Tracks><Track><Name>Lead</Name><Device>Comb Filter</Device></Track></Tracks>
  <Patterns>
    <Pattern>
      <NumberOfLines>4</NumberOfLines>
      <Tracks>
        <Track>
          <Line>
            <Row>1</Row>
            <Note>72</Note>
            <Instrument>1</Instrument>
            <SourceTick>37</SourceTick>
            <Effect><Code>Z</Code><Value>1</Value></Effect>
            <Effect><Code>D</Code><Value>20</Value></Effect>
          </Line>
        </Track>
      </Tracks>
    </Pattern>
  </Patterns>
</RenoiseSong>
"#;
        let archive = xrns_archive([
            xrns_entry("Song.xml", xml.as_bytes()),
            xrns_entry("SampleData/Instrument00/Sample00.flac", b"fLaC"),
        ]);

        let report = import_xrns(&archive);
        let song = report.song.expect("lossy import still produces song");

        song.validate().expect("valid lossy song");
        let cell = song.patterns[0].cell(1, 0).expect("cell");
        assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 72 }));
        assert_eq!(cell.instrument, None);
        assert_eq!(
            cell.command,
            Some(TrackerCommand {
                code: b'Z',
                value: 1
            })
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == XrnsDiagnosticKind::UnsupportedSampleFormat));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == XrnsDiagnosticKind::UnsupportedRenoiseFeature
                && diagnostic.message.contains("instrument")
        }));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == XrnsDiagnosticKind::UnsupportedEffectCommand));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == XrnsDiagnosticKind::DroppedExtraEffectColumn));
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == XrnsDiagnosticKind::TimingQuantized));
    }

    #[test]
    fn xrns_import_rejects_archives_without_song_xml() {
        let archive = xrns_archive([xrns_entry("SampleData/Sample00.wav", b"RIFF....WAVE")]);
        let report = import_xrns(&archive);

        assert!(report.song.is_none());
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == XrnsDiagnosticKind::MissingSongXml));
    }

    fn hex_fixture(contents: &str) -> Vec<u8> {
        contents
            .split_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).expect("hex byte"))
            .collect()
    }

    #[derive(Clone, Copy)]
    struct XrnsTestEntry<'a> {
        path: &'a str,
        data: &'a [u8],
        flags: u16,
        compression_method: u16,
    }

    fn xrns_entry<'a>(path: &'a str, data: &'a [u8]) -> XrnsTestEntry<'a> {
        XrnsTestEntry {
            path,
            data,
            flags: 0,
            compression_method: 0,
        }
    }

    fn xrns_archive<'a>(entries: impl IntoIterator<Item = XrnsTestEntry<'a>>) -> Vec<u8> {
        let mut archive = Vec::new();
        for entry in entries {
            archive.extend_from_slice(&ZIP_LOCAL_FILE_HEADER.to_le_bytes());
            archive.extend_from_slice(&20_u16.to_le_bytes());
            archive.extend_from_slice(&entry.flags.to_le_bytes());
            archive.extend_from_slice(&entry.compression_method.to_le_bytes());
            archive.extend_from_slice(&0_u16.to_le_bytes());
            archive.extend_from_slice(&0_u16.to_le_bytes());
            archive.extend_from_slice(&0_u32.to_le_bytes());
            archive.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            archive.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            archive.extend_from_slice(&(entry.path.len() as u16).to_le_bytes());
            archive.extend_from_slice(&0_u16.to_le_bytes());
            archive.extend_from_slice(entry.path.as_bytes());
            archive.extend_from_slice(entry.data);
        }
        archive
    }
}
