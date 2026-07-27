use std::collections::{HashMap, HashSet};

use trk_core::{
    Instrument, InstrumentId, NoteEvent, PatternCell, SamplePlaybackMode, SampleReference, Song,
};

use crate::diagnostics::{
    XrnsDiagnostic, XrnsDiagnosticKind, XrnsDiagnosticSeverity, XrnsInspection,
};

mod keyzones;
mod model;
mod samples;

use super::devices::effect_device_from_name;
use super::effects::{
    effect_command_needs_warning, effect_command_warning_message, normalize_xrns_effect_code,
    translate_xrns_effect_command,
};
use keyzones::{instrument_zones, parse_keyzone_note, parse_keyzone_velocity};
use model::{
    PendingXrnsLine, XrnsImportCell, XrnsImportInstrument, XrnsImportModel, XrnsImportPattern,
    XrnsImportSampleMetadata, XrnsImportTrack,
};
use samples::import_sample_references;

use super::{parse_xml_events, stack_contains, xml_location, xrns_diagnostic, XmlEvent};

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
    let mut current_instrument: Option<XrnsImportInstrument> = None;
    let mut current_sample: Option<XrnsImportSampleMetadata> = None;
    let mut current_sample_nested_depth = 0_usize;
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
                    current_instrument = Some(XrnsImportInstrument::default());
                } else if name == "Sample" && current_sample.is_some() {
                    current_sample_nested_depth += 1;
                } else if name == "Sample"
                    && current_instrument.is_some()
                    && is_xrns_instrument_sample_container(&stack)
                {
                    current_sample = Some(XrnsImportSampleMetadata::default());
                    current_sample_nested_depth = 0;
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
                } else if name == "Sample" {
                    if current_sample_nested_depth > 0 {
                        current_sample_nested_depth -= 1;
                    } else {
                        if let (Some(instrument), Some(sample)) =
                            (&mut current_instrument, current_sample.take())
                        {
                            instrument.samples.push(sample);
                        }
                        current_sample_nested_depth = 0;
                    }
                } else if name == "Instrument" {
                    if let Some(mut instrument) = current_instrument.take() {
                        if instrument.name.trim().is_empty() {
                            instrument.name =
                                format!("Instrument {:02}", model.instruments.len() + 1);
                        }
                        model.instruments.push(instrument);
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
                            let value = line.effect_value.take().unwrap_or(0);
                            let translated =
                                translate_xrns_effect_command(&code, value, model.ticks_per_line);
                            let command = translated.command;
                            if effect_command_needs_warning(command) {
                                diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::UnsupportedEffectCommand,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, &name)),
                                    effect_command_warning_message(&translated, value),
                                ));
                            }
                            if line.cell.command.is_none() {
                                line.cell.command = Some(command);
                            } else if line.cell.command2.is_none() {
                                line.cell.command2 = Some(command);
                            } else {
                                diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::DroppedExtraEffectColumn,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, &name)),
                                    "extra XRNS effect column was dropped",
                                ));
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
                if let Some(sample) = &mut current_sample {
                    if current_sample_nested_depth == 0 && is_direct_sample_metadata_text(&stack) {
                        apply_xrns_sample_text(current, &text, sample, diagnostics, &stack);
                    }
                } else if let Some(track) = &mut current_track {
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
                        instrument.name = text;
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
                } else if matches!(current, "TicksPerLine" | "TPL") {
                    model.ticks_per_line = text
                        .trim()
                        .parse::<u8>()
                        .ok()
                        .filter(|ticks_per_line| *ticks_per_line > 0)
                        .or(model.ticks_per_line);
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

fn is_xrns_instrument_sample_container(stack: &[String]) -> bool {
    stack.last().is_some_and(|name| name == "Samples")
}

fn is_direct_sample_metadata_text(stack: &[String]) -> bool {
    stack
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|name| name == "Sample")
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
        "Code" | "Command" => line.effect_code = normalize_xrns_effect_code(text),
        "Value" => line.effect_value = parse_xrns_hex_u8_value(text),
        "SourceTick" | "SourceTime" => diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::TimingQuantized,
            XrnsDiagnosticSeverity::Warning,
            Some(xml_location(stack, current)),
            "XRNS timing was quantized to the nearest trk row",
        )),
        _ => {}
    }
}

fn apply_xrns_sample_text(
    current: &str,
    text: &str,
    sample: &mut XrnsImportSampleMetadata,
    diagnostics: &mut Vec<XrnsDiagnostic>,
    stack: &[String],
) {
    match current {
        "Name" => sample.name = Some(text.to_string()),
        "BaseNote" | "RootNote" | "RootPitch" => {
            sample.root_pitch = parse_sample_root_pitch(text);
        }
        "Transpose" | "TransposeSemitones" => {
            sample.transpose_semitones =
                parse_i16_value(text).and_then(|value| i8::try_from(value.clamp(-120, 120)).ok());
        }
        "FineTune" | "Finetune" | "FineTuneCents" => {
            sample.fine_tune_cents = parse_i16_value(text).map(|value| value.clamp(-1200, 1200));
        }
        "Volume" | "Gain" => {
            sample.gain = parse_float(text).map(|gain| quantize_milli(gain.clamp(0.0, 2.0)));
        }
        "Panning" | "Pan" => {
            sample.pan = parse_sample_pan(text);
        }
        "KeyStart" | "KeyRangeStart" | "NoteStart" | "LowNote" | "KeyLow" => {
            sample.key_start = parse_keyzone_note(text);
        }
        "KeyEnd" | "KeyRangeEnd" | "NoteEnd" | "HighNote" | "KeyHigh" => {
            sample.key_end = parse_keyzone_note(text);
        }
        "VelocityStart" | "VelocityRangeStart" | "VelocityLow" => {
            sample.velocity_start = parse_keyzone_velocity(text);
        }
        "VelocityEnd" | "VelocityRangeEnd" | "VelocityHigh" => {
            sample.velocity_end = parse_keyzone_velocity(text);
        }
        "Start" | "StartFrame" | "SampleStart" => {
            sample.playback.start_frame = parse_usize_value(text);
        }
        "End" | "EndFrame" | "SampleEnd" => {
            sample.playback.end_frame = parse_usize_value(text);
        }
        "LoopMode" | "LoopType" => {
            sample.playback.mode = parse_loop_mode(text);
        }
        "LoopStart" | "LoopStartFrame" => {
            sample.playback.loop_start_frame = parse_usize_value(text);
        }
        "LoopEnd" | "LoopEndFrame" => {
            sample.playback.loop_end_frame = parse_usize_value(text);
        }
        "Attack" | "AttackSeconds" => {
            sample.envelope_mut().attack_seconds = parse_envelope_seconds(text);
        }
        "Decay" | "DecaySeconds" => {
            sample.envelope_mut().decay_seconds = parse_envelope_seconds(text);
        }
        "Sustain" => {
            sample.envelope_mut().sustain = parse_float(text).map_or(1.0, |value| {
                if value > 1.0 {
                    (value / 100.0).clamp(0.0, 1.0)
                } else {
                    value.clamp(0.0, 1.0)
                }
            });
        }
        "Release" | "ReleaseSeconds" => {
            sample.envelope_mut().release_seconds = parse_envelope_seconds(text);
        }
        "Autofade" | "InterpolationMode" | "BeatSyncMode" | "BeatSyncLines" | "NewNoteAction"
        | "NNA" | "Oversample" | "SliceMarkers" => diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::UnsupportedRenoiseFeature,
            XrnsDiagnosticSeverity::Warning,
            Some(xml_location(stack, current)),
            format!("unsupported Renoise sample metadata was not imported: {current}"),
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

    let sample_by_instrument_sample = import_sample_references(
        &mut song,
        model,
        inspection,
        sample_path_overrides,
        diagnostics,
    );
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
            sample_by_instrument_sample
                .keys()
                .map(|(id, _)| id.0 as usize + 1)
                .max()
                .unwrap_or(0),
        );
    song.instruments.clear();
    for index in 0..instrument_count {
        let id = InstrumentId(index as u32);
        let name = model
            .instruments
            .get(index)
            .map(|instrument| instrument.name.as_str())
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("Instrument {index:02}"));
        let mut instrument_samples = sample_by_instrument_sample
            .iter()
            .filter_map(|((instrument, sample_index), sample)| {
                (*instrument == id).then_some((*sample_index, *sample))
            })
            .collect::<Vec<_>>();
        instrument_samples.sort_unstable_by_key(|(sample_index, _)| *sample_index);
        song.instruments.push(Instrument {
            id,
            name,
            sample: instrument_samples.first().map(|(_, sample)| *sample),
            zones: instrument_zones(model, id, &instrument_samples, diagnostics),
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

fn apply_sample_metadata(reference: &mut SampleReference, metadata: &XrnsImportSampleMetadata) {
    if let Some(root_pitch) = metadata.root_pitch {
        reference.root_pitch = root_pitch;
    }
    if let Some(transpose_semitones) = metadata.transpose_semitones {
        reference.transpose_semitones = transpose_semitones;
    }
    if let Some(fine_tune_cents) = metadata.fine_tune_cents {
        reference.fine_tune_cents = fine_tune_cents;
    }
    if let Some(gain) = metadata.gain {
        reference.gain = gain;
    }
    if let Some(pan) = metadata.pan {
        reference.pan = pan;
    }
    if let Some(mode) = metadata.playback.mode {
        reference.playback.mode = mode;
    }
    if let Some(start_frame) = metadata.playback.start_frame {
        reference.playback.start_frame = Some(start_frame);
    }
    if let Some(end_frame) = metadata.playback.end_frame {
        reference.playback.end_frame = Some(end_frame);
    }
    if let Some(loop_start_frame) = metadata.playback.loop_start_frame {
        reference.playback.loop_start_frame = Some(loop_start_frame);
    }
    if let Some(loop_end_frame) = metadata.playback.loop_end_frame {
        reference.playback.loop_end_frame = Some(loop_end_frame);
    }
    if let Some(envelope) = metadata.playback.envelope {
        reference.playback.envelope = envelope;
    }
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

fn parse_sample_root_pitch(value: &str) -> Option<u8> {
    parse_u8_value(value)
        .filter(|pitch| *pitch <= 127)
        .or_else(|| parse_note_name(value))
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

fn parse_i16_value(value: &str) -> Option<i16> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    value.parse::<i16>().ok()
}

fn parse_usize_value(value: &str) -> Option<usize> {
    parse_u32_value(value).and_then(|value| usize::try_from(value).ok())
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

#[rustfmt::skip]
fn parse_envelope_seconds(value: &str) -> f32 { parse_float(value).map_or(0.0, |seconds| seconds.clamp(0.0, 60.0)) }

#[rustfmt::skip]
fn parse_sample_pan(value: &str) -> Option<f32> {
    let pan = parse_float(value)?;
    let normalized = if (0.0..=1.0).contains(&pan) { (pan - 0.5) * 2.0 } else { pan };
    Some(quantize_milli(normalized.clamp(-1.0, 1.0)))
}

fn quantize_milli(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

#[rustfmt::skip]
fn parse_loop_mode(value: &str) -> Option<SamplePlaybackMode> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() { return None; }
    if matches!(value.as_str(), "off" | "none" | "oneshot" | "one-shot" | "0") { Some(SamplePlaybackMode::OneShot) }
    else if value.contains("loop") || matches!(value.as_str(), "on" | "1" | "forward") { Some(SamplePlaybackMode::Loop) }
    else { None }
}

#[rustfmt::skip]
fn sample_payload_sample_index(path: &str) -> Option<usize> {
    let segment = path.split('/').find(|segment| segment.strip_prefix("Sample").and_then(|suffix| suffix.chars().next()).is_some_and(|char| char.is_ascii_digit()))?;
    let digits = segment.trim_start_matches("Sample").chars().take_while(char::is_ascii_digit).collect::<String>();
    digits.parse::<usize>().ok()
}

fn sample_name(path: &str) -> String {
    path.rsplit('/')
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(path)
        .to_string()
}
