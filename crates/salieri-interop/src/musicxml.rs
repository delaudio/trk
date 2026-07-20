use salieri_core::{NoteEvent, Pattern, Song};

use crate::diagnostics::{
    InteropError, MusicXmlDiagnostic, MusicXmlDiagnosticKind, MusicXmlDiagnosticSeverity,
    MusicXmlImportReport, MusicXmlRoundTripReport,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MusicXmlExportOptions {
    pub pattern: usize,
}

pub fn export_pattern_musicxml(
    song: &Song,
    options: MusicXmlExportOptions,
) -> Result<String, InteropError> {
    let pattern = song
        .pattern(options.pattern)
        .ok_or(InteropError::MissingPattern(options.pattern))?;
    let divisions = song.transport.lines_per_beat.max(1);
    let rows_per_measure = usize::from(divisions).saturating_mul(4).max(1);
    let measure_count = pattern.row_count().max(1).div_ceil(rows_per_measure);
    let mut output = String::new();

    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<score-partwise version=\"4.0\">\n");
    output.push_str("  <work><work-title>");
    output.push_str(&escape_xml(&song.metadata.title));
    output.push_str("</work-title></work>\n");
    if let Some(author) = &song.metadata.author {
        output.push_str("  <identification><creator type=\"composer\">");
        output.push_str(&escape_xml(author));
        output.push_str("</creator></identification>\n");
    }
    output.push_str("  <part-list>\n");
    for (track_index, track) in song.tracks.iter().enumerate() {
        output.push_str(&format!(
            "    <score-part id=\"P{}\"><part-name>{}</part-name></score-part>\n",
            track_index + 1,
            escape_xml(&track.name)
        ));
    }
    output.push_str("  </part-list>\n");

    for (track_index, _) in song.tracks.iter().enumerate() {
        output.push_str(&format!("  <part id=\"P{}\">\n", track_index + 1));
        for measure_index in 0..measure_count {
            output.push_str(&format!("    <measure number=\"{}\">\n", measure_index + 1));
            if measure_index == 0 {
                output.push_str("      <attributes>\n");
                output.push_str(&format!("        <divisions>{divisions}</divisions>\n"));
                output.push_str("        <time><beats>4</beats><beat-type>4</beat-type></time>\n");
                output.push_str("      </attributes>\n");
                output.push_str("      <direction placement=\"above\"><direction-type><metronome>");
                output.push_str("<beat-unit>quarter</beat-unit>");
                output.push_str(&format!(
                    "<per-minute>{}</per-minute>",
                    song.transport.bpm.max(1)
                ));
                output.push_str("</metronome></direction-type></direction>\n");
            }
            let start_row = measure_index.saturating_mul(rows_per_measure);
            let end_row = (start_row + rows_per_measure).min(pattern.row_count());
            for row in start_row..end_row {
                let cell = pattern.cell(row, track_index);
                if let Some(Some(NoteEvent::Note { pitch })) = cell.map(|cell| cell.note) {
                    let (step, alter, octave) = midi_pitch_to_musicxml(pitch);
                    output.push_str("      <note><pitch>");
                    output.push_str(&format!("<step>{step}</step>"));
                    if alter != 0 {
                        output.push_str(&format!("<alter>{alter}</alter>"));
                    }
                    output.push_str(&format!("<octave>{octave}</octave>"));
                    output.push_str("</pitch>");
                    output.push_str(" <duration>1</duration>");
                    if let Some(velocity) = cell.and_then(|cell| cell.velocity) {
                        output.push_str(&format!(" <velocity>{}</velocity>", velocity.min(127)));
                    }
                    output.push_str("</note>\n");
                } else {
                    output.push_str("      <note><rest/><duration>1</duration></note>\n");
                }
            }
            output.push_str("    </measure>\n");
        }
        output.push_str("  </part>\n");
    }

    output.push_str("</score-partwise>\n");
    Ok(output)
}

pub fn import_musicxml(xml: &str) -> MusicXmlImportReport {
    let document = match parse_xml_document(xml) {
        Ok(document) => document,
        Err(error) => {
            return MusicXmlImportReport {
                song: None,
                diagnostics: vec![musicxml_diagnostic(
                    MusicXmlDiagnosticKind::MalformedXml,
                    MusicXmlDiagnosticSeverity::Error,
                    None,
                    error,
                )],
            };
        }
    };
    let Some(root) = document
        .children
        .iter()
        .find(|node| local_name(&node.name) == "score-partwise")
    else {
        return MusicXmlImportReport {
            song: None,
            diagnostics: vec![musicxml_diagnostic(
                MusicXmlDiagnosticKind::UnsupportedRoot,
                MusicXmlDiagnosticSeverity::Error,
                None,
                "only score-partwise MusicXML is supported",
            )],
        };
    };

    let mut diagnostics = Vec::new();
    let mut song = Song::empty();
    if let Some(title) = descendant_text(root, &["work-title", "movement-title"]) {
        song.metadata.title = title;
    }
    if let Some(author) = descendant_text(root, &["creator"]) {
        song.metadata.author = Some(author);
    }

    let part_names = score_part_names(root);
    let parts = children(root, "part").collect::<Vec<_>>();
    let target_tracks = parts.len().max(1);
    while song.tracks.len() < target_tracks {
        song.create_track();
    }
    while song.tracks.len() > target_tracks && song.tracks.len() > 1 {
        let _ = song.delete_track(song.tracks.len() - 1);
    }
    for (track_index, track) in song.tracks.iter_mut().enumerate() {
        let name = parts
            .get(track_index)
            .and_then(|part| attr(part, "id"))
            .and_then(|part_id| part_names.iter().find(|(id, _)| id == part_id))
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| format!("Part {:02}", track_index + 1));
        track.name = name;
        track.midi_channel = (track_index as u8 + 1).min(16);
        track.armed = track_index == 0;
    }

    let mut events = Vec::new();
    let mut max_row = 0_usize;
    for (track_index, part) in parts.iter().enumerate() {
        let mut divisions = 1_u32;
        let mut position = 0_u64;
        for (measure_index, measure) in children(part, "measure").enumerate() {
            for node in &measure.children {
                match local_name(&node.name) {
                    "attributes" => {
                        if let Some(value) = child_text(node, "divisions")
                            .and_then(|value| value.parse::<u32>().ok())
                        {
                            divisions = value.max(1);
                        }
                    }
                    "direction" => {
                        if let Some(bpm) = descendant_text(node, &["per-minute"])
                            .and_then(|value| value.parse::<u16>().ok())
                        {
                            song.transport.bpm = bpm.max(1);
                        }
                    }
                    "sound" => {
                        if let Some(tempo) =
                            attr(node, "tempo").and_then(|value| value.parse::<f32>().ok())
                        {
                            song.transport.bpm =
                                tempo.round().clamp(1.0, f32::from(u16::MAX)) as u16;
                        }
                    }
                    "note" => {
                        import_note(
                            node,
                            ImportNoteContext {
                                track_index,
                                measure_index,
                                divisions,
                                position,
                                lines_per_beat: song.transport.lines_per_beat,
                            },
                            &mut events,
                            &mut max_row,
                            &mut diagnostics,
                        );
                        if child(node, "chord").is_none() {
                            position = position.saturating_add(
                                child_text(node, "duration")
                                    .and_then(|value| value.parse::<u64>().ok())
                                    .unwrap_or(0),
                            );
                        }
                    }
                    "backup" | "forward" => {
                        diagnostics.push(musicxml_diagnostic(
                            MusicXmlDiagnosticKind::UnsupportedNotation,
                            MusicXmlDiagnosticSeverity::Warning,
                            Some(format!(
                                "part {} measure {}",
                                track_index + 1,
                                measure_index + 1
                            )),
                            format!(
                                "{} timing elements are not imported",
                                local_name(&node.name)
                            ),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    if max_row >= song.current_pattern().expect("default pattern").row_count() {
        let _ = song.resize_pattern(0, max_row + 1);
    }
    for event in events {
        let pattern = song.current_pattern_mut().expect("default pattern");
        if pattern
            .cell(event.row, event.track)
            .is_some_and(|cell| cell.note.is_some())
        {
            diagnostics.push(musicxml_diagnostic(
                MusicXmlDiagnosticKind::DroppedCollision,
                MusicXmlDiagnosticSeverity::Warning,
                Some(format!("row {} track {}", event.row + 1, event.track + 1)),
                "multiple notes quantized to the same tracker cell; later note dropped",
            ));
            continue;
        }
        let _ = pattern.set_note(
            event.row,
            event.track,
            NoteEvent::Note { pitch: event.pitch },
            event.velocity,
        );
    }

    if let Err(error) = song.validate() {
        diagnostics.push(musicxml_diagnostic(
            MusicXmlDiagnosticKind::ValidationFailed,
            MusicXmlDiagnosticSeverity::Error,
            None,
            format!("imported project is invalid: {error}"),
        ));
    }
    MusicXmlImportReport {
        song: Some(song),
        diagnostics,
    }
}

pub fn validate_musicxml_round_trip(
    song: &Song,
    options: MusicXmlExportOptions,
) -> Result<MusicXmlRoundTripReport, InteropError> {
    let exported = export_pattern_musicxml(song, options)?;
    let import_report = import_musicxml(&exported);
    let original = note_signature(
        song.pattern(options.pattern)
            .ok_or(InteropError::MissingPattern(options.pattern))?,
    );
    let imported = import_report
        .song
        .as_ref()
        .and_then(|song| song.pattern(0))
        .map(note_signature)
        .unwrap_or_default();
    let survived = original == imported
        && !import_report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == MusicXmlDiagnosticSeverity::Error);
    Ok(MusicXmlRoundTripReport {
        exported,
        imported_song: import_report.song,
        diagnostics: import_report.diagnostics,
        original_note_count: original.len(),
        imported_note_count: imported.len(),
        survived,
    })
}

struct ImportNoteContext {
    track_index: usize,
    measure_index: usize,
    divisions: u32,
    position: u64,
    lines_per_beat: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImportedNote {
    row: usize,
    track: usize,
    pitch: u8,
    velocity: u8,
}

fn import_note(
    note: &XmlNode,
    context: ImportNoteContext,
    events: &mut Vec<ImportedNote>,
    max_row: &mut usize,
    diagnostics: &mut Vec<MusicXmlDiagnostic>,
) {
    let location = Some(format!(
        "part {} measure {}",
        context.track_index + 1,
        context.measure_index + 1
    ));
    for unsupported in ["tie", "tuplet", "grace", "lyric", "notations"] {
        if descendant(note, unsupported).is_some() {
            diagnostics.push(musicxml_diagnostic(
                MusicXmlDiagnosticKind::UnsupportedNotation,
                MusicXmlDiagnosticSeverity::Warning,
                location.clone(),
                format!("{unsupported} is not represented in tracker cells"),
            ));
        }
    }
    if child(note, "chord").is_some() {
        diagnostics.push(musicxml_diagnostic(
            MusicXmlDiagnosticKind::UnsupportedNotation,
            MusicXmlDiagnosticSeverity::Warning,
            location,
            "chord notes are not imported by the monophonic tracker-cell subset",
        ));
        return;
    }
    if child(note, "rest").is_some() {
        return;
    }
    let Some(pitch_node) = child(note, "pitch") else {
        return;
    };
    let Some(pitch) = musicxml_pitch_to_midi(pitch_node) else {
        diagnostics.push(musicxml_diagnostic(
            MusicXmlDiagnosticKind::UnsupportedNotation,
            MusicXmlDiagnosticSeverity::Warning,
            location,
            "note pitch is incomplete or outside MIDI range",
        ));
        return;
    };
    let duration = child_text(note, "duration")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    if duration == 0 {
        diagnostics.push(musicxml_diagnostic(
            MusicXmlDiagnosticKind::UnsupportedNotation,
            MusicXmlDiagnosticSeverity::Warning,
            location,
            "note without duration was imported at current row",
        ));
    }
    let row = divisions_to_row(context.position, context.divisions, context.lines_per_beat);
    if !is_exact_row(context.position, context.divisions, context.lines_per_beat) {
        diagnostics.push(musicxml_diagnostic(
            MusicXmlDiagnosticKind::QuantizedTiming,
            MusicXmlDiagnosticSeverity::Info,
            Some(format!("row {} track {}", row + 1, context.track_index + 1)),
            "MusicXML duration position was quantized to the nearest tracker row",
        ));
    }
    *max_row = (*max_row).max(row);
    events.push(ImportedNote {
        row,
        track: context.track_index,
        pitch,
        velocity: child_text(note, "velocity")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(0x7f)
            .min(0x7f),
    });
}

fn note_signature(pattern: &Pattern) -> Vec<(usize, usize, u8, u8)> {
    let mut signature = Vec::new();
    for (row_index, row) in pattern.rows.iter().enumerate() {
        for (track_index, cell) in row.cells.iter().enumerate() {
            if let Some(NoteEvent::Note { pitch }) = cell.note {
                signature.push((row_index, track_index, pitch, cell.velocity.unwrap_or(0x7f)));
            }
        }
    }
    signature
}

fn divisions_to_row(position: u64, divisions: u32, lines_per_beat: u8) -> usize {
    let numerator = position.saturating_mul(u64::from(lines_per_beat.max(1)));
    ((numerator + u64::from(divisions.max(1)) / 2) / u64::from(divisions.max(1))) as usize
}

fn is_exact_row(position: u64, divisions: u32, lines_per_beat: u8) -> bool {
    position
        .saturating_mul(u64::from(lines_per_beat.max(1)))
        .is_multiple_of(u64::from(divisions.max(1)))
}

fn musicxml_pitch_to_midi(pitch: &XmlNode) -> Option<u8> {
    let step = child_text(pitch, "step")?;
    let alter = child_text(pitch, "alter")
        .and_then(|value| value.parse::<i16>().ok())
        .unwrap_or(0);
    let octave = child_text(pitch, "octave")?.parse::<i16>().ok()?;
    let semitone = match step.as_str() {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        _ => return None,
    };
    let midi = (octave + 1) * 12 + semitone + alter;
    u8::try_from(midi).ok().filter(|pitch| *pitch <= 127)
}

fn midi_pitch_to_musicxml(pitch: u8) -> (&'static str, i8, i16) {
    let octave = i16::from(pitch / 12) - 1;
    match pitch % 12 {
        0 => ("C", 0, octave),
        1 => ("C", 1, octave),
        2 => ("D", 0, octave),
        3 => ("D", 1, octave),
        4 => ("E", 0, octave),
        5 => ("F", 0, octave),
        6 => ("F", 1, octave),
        7 => ("G", 0, octave),
        8 => ("G", 1, octave),
        9 => ("A", 0, octave),
        10 => ("A", 1, octave),
        _ => ("B", 0, octave),
    }
}

fn score_part_names(root: &XmlNode) -> Vec<(String, String)> {
    descendant(root, "part-list")
        .map(|part_list| {
            children(part_list, "score-part")
                .filter_map(|part| {
                    let id = attr(part, "id")?.to_string();
                    let name = child_text(part, "part-name").unwrap_or_else(|| id.clone());
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn musicxml_diagnostic(
    kind: MusicXmlDiagnosticKind,
    severity: MusicXmlDiagnosticSeverity,
    location: Option<String>,
    message: impl Into<String>,
) -> MusicXmlDiagnostic {
    MusicXmlDiagnostic {
        kind,
        severity,
        location,
        message: message.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct XmlNode {
    name: String,
    attributes: Vec<(String, String)>,
    text: String,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: Vec::new(),
            text: String::new(),
            children: Vec::new(),
        }
    }
}

fn parse_xml_document(xml: &str) -> Result<XmlNode, String> {
    let mut root = XmlNode::new("#document");
    let mut stack = Vec::<XmlNode>::new();
    let mut index = 0;
    while index < xml.len() {
        let Some(open_offset) = xml[index..].find('<') else {
            append_text(stack.last_mut(), &mut root, &xml[index..]);
            break;
        };
        let open = index + open_offset;
        append_text(stack.last_mut(), &mut root, &xml[index..open]);
        let close = xml[open..]
            .find('>')
            .map(|offset| open + offset)
            .ok_or_else(|| "unterminated XML tag".to_string())?;
        let raw_tag = &xml[open + 1..close];
        if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
            index = close + 1;
            continue;
        }
        if raw_tag.starts_with('/') {
            let close_name = raw_tag
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or_default();
            let node = stack
                .pop()
                .ok_or_else(|| format!("unexpected closing tag {close_name}"))?;
            if local_name(&node.name) != local_name(close_name) {
                return Err(format!("mismatched closing tag {close_name}"));
            }
            attach_node(&mut stack, &mut root, node);
        } else {
            let self_closing = raw_tag.trim_end().ends_with('/');
            let mut node = parse_start_tag(raw_tag.trim_end_matches('/').trim())?;
            if self_closing {
                attach_node(&mut stack, &mut root, node);
            } else {
                node.text.clear();
                stack.push(node);
            }
        }
        index = close + 1;
    }
    while let Some(node) = stack.pop() {
        attach_node(&mut stack, &mut root, node);
    }
    Ok(root)
}

fn parse_start_tag(tag: &str) -> Result<XmlNode, String> {
    let mut parts = tag.split_whitespace();
    let name = parts
        .next()
        .ok_or_else(|| "empty XML tag".to_string())?
        .to_string();
    let mut node = XmlNode::new(name);
    for part in parts {
        if let Some((name, value)) = part.split_once('=') {
            node.attributes.push((
                name.to_string(),
                unescape_xml(value.trim_matches('"').trim_matches('\'')),
            ));
        }
    }
    Ok(node)
}

fn append_text(current: Option<&mut XmlNode>, root: &mut XmlNode, text: &str) {
    if text.trim().is_empty() {
        return;
    }
    match current {
        Some(node) => node.text.push_str(&unescape_xml(text.trim())),
        None => root.text.push_str(&unescape_xml(text.trim())),
    }
}

fn attach_node(stack: &mut [XmlNode], root: &mut XmlNode, node: XmlNode) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        root.children.push(node);
    }
}

fn child<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    node.children
        .iter()
        .find(|child| local_name(&child.name) == name)
}

fn children<'a>(node: &'a XmlNode, name: &'a str) -> impl Iterator<Item = &'a XmlNode> {
    node.children
        .iter()
        .filter(move |child| local_name(&child.name) == name)
}

fn descendant<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    if local_name(&node.name) == name {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| descendant(child, name))
}

fn descendant_text(node: &XmlNode, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| descendant(node, name).map(|node| node.text.trim().to_string()))
        .filter(|value| !value.is_empty())
}

fn child_text(node: &XmlNode, name: &str) -> Option<String> {
    child(node, name)
        .map(|node| node.text.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn attr<'a>(node: &'a XmlNode, name: &str) -> Option<&'a str> {
    node.attributes
        .iter()
        .find(|(key, _)| local_name(key) == name)
        .map(|(_, value)| value.as_str())
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn unescape_xml(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
