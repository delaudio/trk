use std::collections::{HashMap, HashSet};

use salieri_core::{
    EffectDevice, Instrument, InstrumentId, NoteEvent, PatternCell, Song, TrackerCommand,
};

use crate::diagnostics::{
    XrnsDiagnostic, XrnsDiagnosticKind, XrnsDiagnosticSeverity, XrnsInspection,
};

use super::{
    parse_xml_events, sample_payload_instrument_id, stack_contains, xml_location, xrns_diagnostic,
    XmlEvent,
};

#[derive(Debug, Clone, Default)]
pub(super) struct XrnsImportModel {
    tracks: Vec<XrnsImportTrack>,
    patterns: Vec<XrnsImportPattern>,
    instruments: Vec<String>,
    sequence: Vec<usize>,
    bpm: Option<u16>,
    lines_per_beat: Option<u8>,
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

pub(super) fn parse_xrns_import_model(
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
                if is_xrns_song_track_container(&name)
                    && stack_contains(&stack, "Tracks")
                    && !in_pattern
                {
                    current_track = Some(XrnsImportTrack::default());
                } else if name == "Instrument" && stack_contains(&stack, "Instruments") {
                    current_instrument = Some(String::new());
                } else if name == "Pattern" && stack_contains(&stack, "Patterns") {
                    current_pattern = Some(XrnsImportPattern::default());
                    current_pattern_track = None;
                    pattern_track_line_counts.clear();
                } else if is_xrns_pattern_track_container(&name) && in_pattern {
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
                if is_xrns_pattern_track_container(&name)
                    && current_line.is_none()
                    && current_pattern.is_some()
                {
                    current_pattern_track = None;
                } else if is_xrns_song_track_container(&name) && current_line.is_none() {
                    if let Some(track) = current_track.take() {
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
                            if pattern_track_line_counts.len() <= line.track {
                                pattern_track_line_counts.resize(line.track + 1, 0);
                            }
                            let count = &mut pattern_track_line_counts[line.track];
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
                } else if matches!(current, "BeatsPerMin" | "BeatsPerMinute" | "BPM") {
                    model.bpm = text
                        .trim()
                        .parse::<u16>()
                        .ok()
                        .filter(|bpm| *bpm > 0)
                        .or(model.bpm);
                } else if matches!(current, "LinesPerBeat" | "LPB") {
                    model.lines_per_beat = text
                        .trim()
                        .parse::<u8>()
                        .ok()
                        .filter(|lines_per_beat| *lines_per_beat > 0)
                        .or(model.lines_per_beat);
                }
            }
        }
    }

    Some(model)
}

fn is_xrns_pattern_track_container(name: &str) -> bool {
    matches!(name, "Track" | "PatternTrack")
}

fn is_xrns_song_track_container(name: &str) -> bool {
    matches!(name, "Track" | "SequencerTrack")
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
            line.cell.instrument = parse_xrns_hex_u32_value(text).map(InstrumentId);
        }
        "Volume" => line.cell.volume = parse_xrns_note_column_level(text),
        "Pan" | "Panning" => line.cell.pan = parse_xrns_note_column_level(text),
        "Delay" => line.cell.delay = parse_xrns_hex_u8_value(text),
        "Code" | "Command" => {
            line.effect_code = text
                .as_bytes()
                .first()
                .copied()
                .map(|byte| byte.to_ascii_uppercase());
        }
        "Value" => line.effect_value = parse_xrns_hex_u8_value(text),
        "SourceTick" | "SourceTime" => diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::TimingQuantized,
            XrnsDiagnosticSeverity::Warning,
            Some(xml_location(stack, current)),
            "XRNS timing was quantized to the nearest Salieri row",
        )),
        _ => {}
    }
}

pub(super) fn build_song_from_xrns_model(
    model: &XrnsImportModel,
    inspection: &XrnsInspection,
    sample_path_overrides: &HashMap<String, String>,
    diagnostics: &mut Vec<XrnsDiagnostic>,
) -> Option<Song> {
    let mut song = Song::empty();
    if let Some(bpm) = model.bpm {
        song.transport.bpm = bpm;
    }
    if let Some(lines_per_beat) = model.lines_per_beat {
        song.transport.lines_per_beat = lines_per_beat;
    }
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

    let mut sample_by_instrument = HashMap::<InstrumentId, _>::new();
    for sample in &inspection.sample_payloads {
        let Some(instrument) = sample_payload_instrument_id(&sample.path) else {
            continue;
        };
        if sample.supported || sample_path_overrides.contains_key(&sample.path) {
            let sample_path = sample_path_overrides
                .get(&sample.path)
                .map_or(sample.path.as_str(), String::as_str);
            let sample_id = song.upsert_sample_reference(sample_path, sample_name(&sample.path));
            sample_by_instrument.insert(instrument, sample_id);
        } else {
            diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::UnsupportedSampleFormat,
                XrnsDiagnosticSeverity::Warning,
                Some(sample.path.clone()),
                format!(
                    "instrument {:?} references unsupported sample format {}",
                    instrument, sample.format
                ),
            ));
        }
    }
    let referenced_instruments = model
        .patterns
        .iter()
        .flat_map(|pattern| &pattern.cells)
        .filter_map(|cell| cell.cell.instrument)
        .collect::<HashSet<_>>();
    let instrument_count = model
        .instruments
        .len()
        .max(
            referenced_instruments
                .iter()
                .map(|id| id.0 as usize + 1)
                .max()
                .unwrap_or(0),
        )
        .max(
            sample_by_instrument
                .keys()
                .map(|id| id.0 as usize + 1)
                .max()
                .unwrap_or(0),
        );
    song.instruments.clear();
    for index in 0..instrument_count {
        let id = InstrumentId(index as u32);
        let name = model
            .instruments
            .get(index)
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("Instrument {index:02}"));
        song.instruments.push(Instrument {
            id,
            name,
            sample: sample_by_instrument.get(&id).copied(),
        });
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
            song.pattern_mut(index)?
                .set_cell(imported.row, imported.track, imported.cell.clone())
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

fn parse_xrns_hex_u8_value(value: &str) -> Option<u8> {
    parse_xrns_hex_u32_value(value).and_then(|value| u8::try_from(value).ok())
}

fn parse_xrns_hex_u32_value(value: &str) -> Option<u32> {
    let value = value.trim();
    if value.is_empty() || matches!(value, ".." | "---") {
        return None;
    }
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u32::from_str_radix(value, 16).ok()
}

fn parse_xrns_note_column_level(value: &str) -> Option<u8> {
    let value = parse_xrns_hex_u32_value(value)?;
    match value {
        0x00..=0x7f => Some(value as u8),
        0x80 => Some(0x7f),
        0xff => None,
        _ => None,
    }
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
