use trk_core::Song;

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
pub struct TrackerModuleSampleExtraction {
    pub inspection: TrackerModuleInspection,
    pub samples: Vec<ExtractedTrackerModuleSample>,
    pub diagnostics: Vec<TrackerModuleDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedTrackerModuleSample {
    pub info: TrackerModuleSampleInfo,
    pub data: Vec<u8>,
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
    #[error("invalid MusicXML document: {0}")]
    InvalidMusicXml(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MusicXmlImportReport {
    pub song: Option<Song>,
    pub diagnostics: Vec<MusicXmlDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MusicXmlRoundTripReport {
    pub exported: String,
    pub imported_song: Option<Song>,
    pub diagnostics: Vec<MusicXmlDiagnostic>,
    pub original_note_count: usize,
    pub imported_note_count: usize,
    pub survived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicXmlDiagnostic {
    pub kind: MusicXmlDiagnosticKind,
    pub severity: MusicXmlDiagnosticSeverity,
    pub location: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicXmlDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicXmlDiagnosticKind {
    MalformedXml,
    UnsupportedRoot,
    UnsupportedNotation,
    QuantizedTiming,
    DroppedCollision,
    ValidationFailed,
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
pub struct XrnsExtractedSample {
    pub source_path: String,
    pub format: String,
    pub supported: bool,
    pub bytes: Vec<u8>,
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
