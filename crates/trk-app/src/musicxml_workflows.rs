use super::*;

pub(crate) fn run_import_musicxml(args: &ImportMusicXmlArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing import input path: usage is trk import musicxml INPUT OUTPUT")?;
    let output_path = args
        .output_path
        .as_deref()
        .context("missing import output path: usage is trk import musicxml INPUT OUTPUT")?;
    let xml = fs::read_to_string(input_path)
        .with_context(|| format!("failed to read MusicXML import {}", input_path.display()))?;
    let report = import_musicxml(&xml);
    print_musicxml_diagnostics(&report.diagnostics);
    if report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == MusicXmlDiagnosticSeverity::Error)
    {
        anyhow::bail!("MusicXML import failed; project was not written");
    }
    let song = report
        .song
        .context("MusicXML import produced no project; project was not written")?;
    let track_count = song.tracks.len();
    let pattern_rows = song
        .current_pattern()
        .map_or(0, |pattern| pattern.row_count());
    save_song_project(output_path, &song)?;
    println!(
        "Imported {} to {}: {} tracks, {} rows, {} diagnostic(s)",
        input_path.display(),
        output_path.display(),
        track_count,
        pattern_rows,
        report.diagnostics.len()
    );
    Ok(())
}

pub(crate) fn run_export_musicxml(args: &MusicXmlExportArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing export input path: usage is trk export musicxml INPUT [OUTPUT]")?;
    let song = load_project(input_path)?;
    let xml = export_pattern_musicxml(
        &song,
        MusicXmlExportOptions {
            pattern: args.pattern - 1,
        },
    )?;
    if let Some(output_path) = &args.output_path {
        write_bytes_atomically(output_path, xml.as_bytes()).with_context(|| {
            format!("failed to write MusicXML export {}", output_path.display())
        })?;
        println!(
            "Exported pattern {} to {}",
            args.pattern,
            output_path.display()
        );
    } else {
        print!("{xml}");
    }
    Ok(())
}

pub(crate) fn run_validate_round_trip(args: &RoundTripValidationArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing validation input path: usage is trk validate roundtrip INPUT [OUTPUT]")?;
    let song = load_project(input_path)?;
    let musicxml = validate_musicxml_round_trip(
        &song,
        MusicXmlExportOptions {
            pattern: args.pattern - 1,
        },
    )?;
    let midi = validate_midi_round_trip(&song, args.pattern - 1)?;
    let report = RoundTripValidationReport { musicxml, midi };
    let output = match args.format {
        AnalysisOutputFormat::Text => format_round_trip_text(&report),
        AnalysisOutputFormat::Json => format_round_trip_json(&report)?,
    };
    if let Some(output_path) = &args.output_path {
        write_bytes_atomically(output_path, output.as_bytes()).with_context(|| {
            format!(
                "failed to write round-trip validation report {}",
                output_path.display()
            )
        })?;
    } else {
        print!("{output}");
    }
    if !report.musicxml.survived || !report.midi.survived {
        anyhow::bail!("round-trip validation found survivability loss");
    }
    Ok(())
}

struct RoundTripValidationReport {
    musicxml: MusicXmlRoundTripReport,
    midi: MidiRoundTripReport,
}

struct MidiRoundTripReport {
    original_note_count: usize,
    imported_note_count: usize,
    survived: bool,
    diagnostics: Vec<String>,
}

fn validate_midi_round_trip(song: &Song, pattern: usize) -> Result<MidiRoundTripReport> {
    let original = midi_note_signature(song, pattern)?;
    let bytes = trk_interop::export_pattern_smf(
        song,
        MidiExportOptions {
            pattern,
            ..MidiExportOptions::default()
        },
    )?;
    let imported = import_smf(&bytes)?;
    let imported_signature = midi_note_signature(&imported, 0)?;
    Ok(MidiRoundTripReport {
        original_note_count: original.len(),
        imported_note_count: imported_signature.len(),
        survived: original == imported_signature,
        diagnostics: if original == imported_signature {
            Vec::new()
        } else {
            vec!["MIDI round-trip note rows/tracks/pitches/velocities differ".to_string()]
        },
    })
}

fn midi_note_signature(song: &Song, pattern: usize) -> Result<Vec<(usize, u8, u8, u8)>> {
    let pattern = song
        .pattern(pattern)
        .ok_or_else(|| anyhow::anyhow!("pattern {} does not exist", pattern + 1))?;
    let mut signature = Vec::new();
    for (row_index, row) in pattern.rows.iter().enumerate() {
        for (track_index, cell) in row.cells.iter().enumerate() {
            if let Some(NoteEvent::Note { pitch }) = cell.note {
                let channel = song
                    .tracks
                    .get(track_index)
                    .map_or(1, |track| track.midi_channel);
                signature.push((row_index, channel, pitch, cell.velocity.unwrap_or(0x7f)));
            }
        }
    }
    Ok(signature)
}

fn format_round_trip_text(report: &RoundTripValidationReport) -> String {
    let mut output = String::new();
    output.push_str("Round-trip validation\n");
    output.push_str(&format!(
        "- MusicXML: {} ({} -> {} notes, {} diagnostic(s))\n",
        if report.musicxml.survived {
            "PASS"
        } else {
            "FAIL"
        },
        report.musicxml.original_note_count,
        report.musicxml.imported_note_count,
        report.musicxml.diagnostics.len()
    ));
    for diagnostic in &report.musicxml.diagnostics {
        output.push_str(&format!(
            "  - {:?} {:?}: {}{}\n",
            diagnostic.severity,
            diagnostic.kind,
            diagnostic.message,
            diagnostic
                .location
                .as_deref()
                .map_or_else(String::new, |location| format!(" ({location})"))
        ));
    }
    output.push_str(&format!(
        "- MIDI: {} ({} -> {} notes, {} diagnostic(s))\n",
        if report.midi.survived { "PASS" } else { "FAIL" },
        report.midi.original_note_count,
        report.midi.imported_note_count,
        report.midi.diagnostics.len()
    ));
    for diagnostic in &report.midi.diagnostics {
        output.push_str(&format!("  - {diagnostic}\n"));
    }
    output
}

fn format_round_trip_json(report: &RoundTripValidationReport) -> Result<String> {
    let value = serde_json::json!({
        "musicxml": {
            "survived": report.musicxml.survived,
            "originalNoteCount": report.musicxml.original_note_count,
            "importedNoteCount": report.musicxml.imported_note_count,
            "diagnostics": report.musicxml.diagnostics.iter().map(|diagnostic| {
                serde_json::json!({
                    "kind": format!("{:?}", diagnostic.kind),
                    "severity": format!("{:?}", diagnostic.severity),
                    "location": diagnostic.location,
                    "message": diagnostic.message,
                })
            }).collect::<Vec<_>>(),
        },
        "midi": {
            "survived": report.midi.survived,
            "originalNoteCount": report.midi.original_note_count,
            "importedNoteCount": report.midi.imported_note_count,
            "diagnostics": report.midi.diagnostics,
        }
    });
    serde_json::to_string_pretty(&value).context("failed to encode round-trip report JSON")
}

fn print_musicxml_diagnostics(diagnostics: &[trk_interop::MusicXmlDiagnostic]) {
    for diagnostic in diagnostics {
        eprintln!(
            "MusicXML {:?}: {}{}",
            diagnostic.severity,
            diagnostic.message,
            diagnostic
                .location
                .as_deref()
                .map_or_else(String::new, |location| format!(" ({location})"))
        );
    }
}
