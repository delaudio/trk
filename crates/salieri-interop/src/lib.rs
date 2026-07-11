use std::collections::HashMap;

use salieri_core::{
    pattern_events, row_duration_micros, ClipId, ClipSource, NoteEvent, PatternId, PlaybackEvent,
    PlaybackEventKind, PlaybackPosition, SceneId, Song,
};

const MTHD: &[u8; 4] = b"MThd";
const MTRK: &[u8; 4] = b"MTrk";
const DEFAULT_TICKS_PER_QUARTER: u16 = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiExportOptions {
    pub pattern: usize,
    pub ticks_per_quarter: u16,
    pub target: MidiExportTarget,
}

impl Default for MidiExportOptions {
    fn default() -> Self {
        Self {
            pattern: 0,
            ticks_per_quarter: DEFAULT_TICKS_PER_QUARTER,
            target: MidiExportTarget::Pattern { pattern: 0 },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiExportTarget {
    Pattern { pattern: usize },
    Sequence,
    Clip { clip_id: ClipId },
    Scene { scene_id: SceneId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerModuleFormat {
    Mod,
    Xm,
    It,
    S3m,
    Renoise,
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
    #[error("clip {0:?} does not exist")]
    MissingClip(ClipId),
    #[error("scene {0:?} does not exist")]
    MissingScene(SceneId),
    #[error("invalid symbolic document")]
    InvalidSymbolicDocument,
}

pub fn export_pattern_smf(
    song: &Song,
    options: MidiExportOptions,
) -> Result<Vec<u8>, InteropError> {
    let target = match options.target {
        MidiExportTarget::Pattern { pattern: 0 } if options.pattern != 0 => {
            MidiExportTarget::Pattern {
                pattern: options.pattern,
            }
        }
        target => target,
    };
    let events = midi_target_events(song, target)?;
    let ticks_per_quarter = options.ticks_per_quarter.max(1);
    let micros_per_quarter = 60_000_000_u64 / u64::from(song.transport.bpm.max(1));
    let mut track = Vec::new();

    write_var_len(0, &mut track);
    track.extend_from_slice(&[0xff, 0x51, 0x03]);
    let tempo = micros_per_quarter.min(0x00ff_ffff) as u32;
    track.extend_from_slice(&tempo.to_be_bytes()[1..4]);

    let mut last_tick = 0_u64;
    for event in events {
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

    if format > 1 {
        return Err(InteropError::UnsupportedMidiFormat(format));
    }
    if division & 0x8000 != 0 {
        return Err(InteropError::UnsupportedSmpteDivision(division));
    }

    let mut song = Song::empty();
    let ticks_per_quarter = u64::from(division.max(1));
    let mut channel_tracks = HashMap::new();

    for track_number in 0..usize::from(track_count) {
        if cursor.read_exact(4)? != MTRK {
            return Err(InteropError::InvalidMidiTrack);
        }
        let track_len = cursor.read_u32()? as usize;
        let track_end = cursor.position().saturating_add(track_len);
        if track_end > bytes.len() {
            return Err(InteropError::TruncatedMidiData);
        }
        parse_midi_track(
            &mut cursor,
            track_end,
            &mut song,
            ticks_per_quarter,
            format,
            track_number,
            &mut channel_tracks,
        )?;
    }

    Ok(song)
}

fn parse_midi_track(
    cursor: &mut Cursor<'_>,
    track_end: usize,
    song: &mut Song,
    ticks_per_quarter: u64,
    format: u16,
    track_number: usize,
    channel_tracks: &mut HashMap<u8, usize>,
) -> Result<(), InteropError> {
    let mut absolute_tick = 0_u64;
    let mut running_status = None;
    let mut track_name = None;

    while cursor.position() < track_end {
        absolute_tick = absolute_tick.saturating_add(read_var_len(cursor)?);
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
                    let track = if format == 1 {
                        track_for_midi_track(song, track_number, channel, track_name.as_deref())
                    } else {
                        track_for_channel(song, channel_tracks, channel)
                    };
                    let row = ticks_to_row(
                        absolute_tick,
                        ticks_per_quarter,
                        song.transport.lines_per_beat,
                    );
                    ensure_pattern_row(song, row)?;
                    song.current_pattern_mut()
                        .expect("default song has a pattern")
                        .set_note(row, track, NoteEvent::Note { pitch }, velocity)
                        .expect("row and track were ensured");
                }
            }
            0xff => {
                let meta_type = cursor.read_u8()?;
                let len = read_var_len(cursor)? as usize;
                if meta_type == 0x51 && len == 3 {
                    let tempo = read_tempo(cursor)?;
                    song.transport.bpm = tempo_to_bpm(tempo);
                } else if meta_type == 0x03 {
                    let name = String::from_utf8_lossy(cursor.read_exact(len)?).to_string();
                    track_name = (!name.trim().is_empty()).then_some(name);
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

    Ok(())
}

pub fn import_tracker_module(
    _bytes: &[u8],
    format: TrackerModuleFormat,
) -> Result<Song, InteropError> {
    Err(InteropError::UnsupportedTrackerModule(format))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundTripReport {
    pub source_notes: usize,
    pub imported_notes: usize,
    pub preserved_notes: usize,
    pub lost_notes: usize,
    pub warnings: Vec<String>,
}

pub fn validate_midi_round_trip(
    song: &Song,
    options: MidiExportOptions,
) -> Result<RoundTripReport, InteropError> {
    let source_events = midi_target_events(song, options.target)?;
    let source_notes = source_events
        .iter()
        .filter(|event| matches!(event.kind, PlaybackEventKind::NoteOn { .. }))
        .count();
    let bytes = export_pattern_smf(song, options)?;
    let imported = import_smf(&bytes)?;
    let imported_notes = count_song_notes(&imported);
    let preserved_notes = source_notes.min(imported_notes);
    let lost_notes = source_notes.saturating_sub(preserved_notes);
    let mut warnings = Vec::new();
    if lost_notes > 0 {
        warnings.push(format!("{lost_notes} note(s) were not preserved"));
    }
    Ok(RoundTripReport {
        source_notes,
        imported_notes,
        preserved_notes,
        lost_notes,
        warnings,
    })
}

pub fn export_musicxml_pattern(song: &Song, pattern_index: usize) -> Result<String, InteropError> {
    let pattern = song
        .pattern(pattern_index)
        .ok_or(InteropError::MissingPattern(pattern_index))?;
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><score-partwise version="3.1"><part-list><score-part id="P1"><part-name>Salieri</part-name></score-part></part-list><part id="P1"><measure number="1">"#,
    );
    for (row_index, row) in pattern.rows.iter().enumerate() {
        for (track_index, cell) in row.cells.iter().enumerate() {
            if let Some(NoteEvent::Note { pitch }) = cell.note {
                xml.push_str(&format!(
                    r#"<note row="{row_index}" track="{track_index}" pitch="{pitch}" velocity="{}"/>"#,
                    cell.velocity.unwrap_or(0x7f)
                ));
            }
        }
    }
    xml.push_str("</measure></part></score-partwise>");
    Ok(xml)
}

pub fn import_musicxml_subset(xml: &str) -> Result<Song, InteropError> {
    if !xml.contains("score-partwise") {
        return Err(InteropError::InvalidSymbolicDocument);
    }
    let mut song = Song::empty();
    for tag in note_tags(xml) {
        let row = attr(tag, "row")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let track = attr(tag, "track")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let pitch = attr(tag, "pitch")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(60)
            .min(127);
        let velocity = attr(tag, "velocity")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(0x7f)
            .min(127);
        ensure_import_shape(&mut song, row, track)?;
        song.current_pattern_mut()
            .expect("default song has a pattern")
            .set_note(row, track, NoteEvent::Note { pitch }, velocity)
            .map_err(|_| InteropError::InvalidSymbolicDocument)?;
    }
    Ok(song)
}

pub fn import_renoise_song_xml_subset(xml: &str) -> Result<Song, InteropError> {
    if !xml.contains("RenoiseSong") {
        return Err(InteropError::InvalidSymbolicDocument);
    }
    let mut song = Song::empty();
    for (track_index, tag) in xml
        .match_indices("<Track ")
        .map(|(_, rest)| rest)
        .enumerate()
    {
        if let Some(name) = attr(tag, "name") {
            while song.tracks.len() <= track_index {
                song.create_track();
            }
            song.tracks[track_index].name = name.to_string();
        }
    }
    for tag in note_tags(xml) {
        let row = attr(tag, "row")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let track = attr(tag, "track")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let pitch = attr(tag, "pitch")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(60)
            .min(127);
        let velocity = attr(tag, "velocity")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(0x7f)
            .min(127);
        ensure_import_shape(&mut song, row, track)?;
        song.current_pattern_mut()
            .expect("default song has a pattern")
            .set_note(row, track, NoteEvent::Note { pitch }, velocity)
            .map_err(|_| InteropError::InvalidSymbolicDocument)?;
    }
    Ok(song)
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

fn track_for_midi_track(
    song: &mut Song,
    track_number: usize,
    channel: u8,
    track_name: Option<&str>,
) -> usize {
    while song.tracks.len() <= track_number {
        song.create_track();
    }
    let track = &mut song.tracks[track_number];
    track.midi_channel = channel;
    if let Some(track_name) = track_name {
        track.name = track_name.to_string();
    }
    track_number
}

fn midi_target_events(
    song: &Song,
    target: MidiExportTarget,
) -> Result<Vec<PlaybackEvent>, InteropError> {
    match target {
        MidiExportTarget::Pattern { pattern } => {
            let pattern = song
                .pattern(pattern)
                .ok_or(InteropError::MissingPattern(pattern))?;
            Ok(pattern_events(song, pattern))
        }
        MidiExportTarget::Sequence => {
            let mut events = Vec::new();
            let mut row_offset = 0usize;
            let mut micros_offset = 0u64;
            let row_duration = row_duration_micros(&song.transport);
            for pattern_id in &song.sequence {
                let Some(pattern) = pattern_by_id(song, *pattern_id) else {
                    continue;
                };
                for mut event in pattern_events(song, pattern) {
                    event.position = PlaybackPosition {
                        row: event.position.row.saturating_add(row_offset),
                        offset_micros: event.position.offset_micros.saturating_add(micros_offset),
                    };
                    events.push(event);
                }
                row_offset = row_offset.saturating_add(pattern.row_count());
                micros_offset = micros_offset
                    .saturating_add(row_duration.saturating_mul(pattern.row_count() as u64));
            }
            events.sort_by_key(|event| event.position.offset_micros);
            Ok(events)
        }
        MidiExportTarget::Clip { clip_id } => {
            let clip = song
                .session
                .clips
                .iter()
                .find(|clip| clip.id == clip_id)
                .ok_or(InteropError::MissingClip(clip_id))?;
            let ClipSource::Pattern {
                pattern_id,
                row_start,
                row_count,
            } = clip.source;
            let pattern = pattern_by_id(song, pattern_id)
                .ok_or(InteropError::MissingPattern(pattern_id.0 as usize))?;
            Ok(pattern_events(song, pattern)
                .into_iter()
                .filter(|event| {
                    event.position.row >= row_start
                        && event.position.row < row_start.saturating_add(row_count)
                })
                .map(|mut event| {
                    let row_delta = event.position.row.saturating_sub(row_start);
                    event.position.row = row_delta;
                    event.position.offset_micros = event.position.offset_micros.saturating_sub(
                        row_duration_micros(&song.transport).saturating_mul(row_start as u64),
                    );
                    event
                })
                .collect())
        }
        MidiExportTarget::Scene { scene_id } => {
            let scene = song
                .session
                .scenes
                .iter()
                .find(|scene| scene.id == scene_id)
                .ok_or(InteropError::MissingScene(scene_id))?;
            let mut events = Vec::new();
            for slot in &scene.slots {
                let Some(clip_id) = slot.clip else {
                    continue;
                };
                let clip_events = midi_target_events(song, MidiExportTarget::Clip { clip_id })?;
                events.extend(
                    clip_events
                        .into_iter()
                        .filter(|event| event.track == slot.track),
                );
            }
            events.sort_by_key(|event| event.position.offset_micros);
            Ok(events)
        }
    }
}

fn pattern_by_id(song: &Song, pattern_id: PatternId) -> Option<&salieri_core::Pattern> {
    song.patterns
        .iter()
        .find(|pattern| pattern.id == pattern_id)
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

fn ensure_import_shape(song: &mut Song, row: usize, track: usize) -> Result<(), InteropError> {
    while song.tracks.len() <= track {
        song.create_track();
    }
    ensure_pattern_row(song, row)
}

fn count_song_notes(song: &Song) -> usize {
    song.patterns
        .iter()
        .flat_map(|pattern| &pattern.rows)
        .flat_map(|row| &row.cells)
        .filter(|cell| matches!(cell.note, Some(NoteEvent::Note { .. })))
        .count()
}

fn note_tags(xml: &str) -> Vec<&str> {
    let mut tags = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<note") {
        rest = &rest[start..];
        if let Some(end) = rest.find("/>") {
            let tag = &rest[..end];
            if tag.contains("pitch=") {
                tags.push(tag);
            }
            rest = &rest[end + 2..];
        } else if let Some(end) = rest.find('>') {
            let tag = &rest[..end];
            if tag.contains("pitch=") {
                tags.push(tag);
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    tags
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=\"");
    let start = tag.find(&prefix)? + prefix.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
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
    fn imports_format_one_tracks_names_and_tempo() {
        let bytes = format_one_fixture();

        let song = import_smf(&bytes).expect("format 1 import");

        assert_eq!(song.transport.bpm, 100);
        assert_eq!(song.tracks[1].name, "Kick");
        assert_eq!(song.tracks[2].name, "Bass");
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("kick")
                .note,
            Some(NoteEvent::Note { pitch: 36 })
        );
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(4, 2)
                .expect("bass")
                .note,
            Some(NoteEvent::Note { pitch: 40 })
        );
    }

    #[test]
    fn exports_clip_and_scene_targets() {
        let mut song = Song::empty();
        song.resize_pattern(0, 16).expect("resize");
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(4, 0, NoteEvent::Note { pitch: 60 }, 100)
            .expect("note");
        let clip = song
            .create_clip(song.patterns[0].id, "Clip", 4, 4)
            .expect("clip");
        let scene = song.create_scene("Scene").expect("scene");
        song.set_scene_clip(scene, song.tracks[0].id, Some(clip))
            .expect("slot");

        let clip_bytes = export_pattern_smf(
            &song,
            MidiExportOptions {
                target: MidiExportTarget::Clip { clip_id: clip },
                ..MidiExportOptions::default()
            },
        )
        .expect("clip export");
        let scene_bytes = export_pattern_smf(
            &song,
            MidiExportOptions {
                target: MidiExportTarget::Scene { scene_id: scene },
                ..MidiExportOptions::default()
            },
        )
        .expect("scene export");

        let clip_imported = import_smf(&clip_bytes).expect("clip import");
        let scene_imported = import_smf(&scene_bytes).expect("scene import");

        assert_eq!(count_song_notes(&clip_imported), 1);
        assert_eq!(count_song_notes(&scene_imported), 1);
    }

    #[test]
    fn musicxml_subset_round_trips_notes() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(2, 0, NoteEvent::Note { pitch: 65 }, 80)
            .expect("note");

        let xml = export_musicxml_pattern(&song, 0).expect("export xml");
        let imported = import_musicxml_subset(&xml).expect("import xml");

        assert_eq!(
            imported
                .current_pattern()
                .expect("pattern")
                .cell(2, 0)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 65 })
        );
    }

    #[test]
    fn imports_renoise_song_xml_subset() {
        let xml = r#"<RenoiseSong><Tracks><Track name="Drums"/><Track name="Bass"/></Tracks><Pattern><note row="0" track="1" pitch="40" velocity="96"/></Pattern></RenoiseSong>"#;

        let song = import_renoise_song_xml_subset(xml).expect("renoise import");

        assert_eq!(song.tracks[1].name, "Bass");
        assert_eq!(
            song.current_pattern()
                .expect("pattern")
                .cell(0, 1)
                .expect("cell")
                .note,
            Some(NoteEvent::Note { pitch: 40 })
        );
    }

    #[test]
    fn midi_round_trip_report_counts_preserved_notes() {
        let mut song = Song::empty();
        song.current_pattern_mut()
            .expect("pattern")
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
            .expect("note");

        let report =
            validate_midi_round_trip(&song, MidiExportOptions::default()).expect("round trip");

        assert_eq!(report.source_notes, 1);
        assert_eq!(report.imported_notes, 1);
        assert_eq!(report.lost_notes, 0);
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
        bytes[9] = 2;

        assert!(matches!(
            import_smf(&bytes),
            Err(InteropError::UnsupportedMidiFormat(2))
        ));
        assert!(matches!(
            import_tracker_module(&[], TrackerModuleFormat::Xm),
            Err(InteropError::UnsupportedTrackerModule(
                TrackerModuleFormat::Xm
            ))
        ));
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

    fn hex_fixture(contents: &str) -> Vec<u8> {
        contents
            .split_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).expect("hex byte"))
            .collect()
    }

    fn format_one_fixture() -> Vec<u8> {
        let mut tempo_track = Vec::new();
        write_var_len(0, &mut tempo_track);
        tempo_track.extend_from_slice(&[0xff, 0x51, 0x03, 0x09, 0x27, 0xc0]);
        write_var_len(0, &mut tempo_track);
        tempo_track.extend_from_slice(&[0xff, 0x2f, 0x00]);

        let mut kick_track = Vec::new();
        write_var_len(0, &mut kick_track);
        kick_track.extend_from_slice(&[0xff, 0x03, 0x04]);
        kick_track.extend_from_slice(b"Kick");
        write_var_len(0, &mut kick_track);
        kick_track.extend_from_slice(&[0x99, 36, 100]);
        write_var_len(480, &mut kick_track);
        kick_track.extend_from_slice(&[0x89, 36, 0]);
        write_var_len(0, &mut kick_track);
        kick_track.extend_from_slice(&[0xff, 0x2f, 0x00]);

        let mut bass_track = Vec::new();
        write_var_len(0, &mut bass_track);
        bass_track.extend_from_slice(&[0xff, 0x03, 0x04]);
        bass_track.extend_from_slice(b"Bass");
        write_var_len(480, &mut bass_track);
        bass_track.extend_from_slice(&[0x90, 40, 100]);
        write_var_len(480, &mut bass_track);
        bass_track.extend_from_slice(&[0x80, 40, 0]);
        write_var_len(0, &mut bass_track);
        bass_track.extend_from_slice(&[0xff, 0x2f, 0x00]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(MTHD);
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&3_u16.to_be_bytes());
        bytes.extend_from_slice(&480_u16.to_be_bytes());
        for track in [tempo_track, kick_track, bass_track] {
            bytes.extend_from_slice(MTRK);
            bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&track);
        }
        bytes
    }
}
