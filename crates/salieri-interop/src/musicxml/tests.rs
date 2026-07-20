use salieri_core::{NoteEvent, Song};

use super::*;

#[test]
fn exports_and_imports_basic_partwise_musicxml() {
    let mut song = Song::empty();
    song.metadata.title = "Round Trip".to_string();
    song.metadata.author = Some("Composer".to_string());
    song.tracks[0].name = "Lead".to_string();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("set note");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(4, 0, NoteEvent::Note { pitch: 64 }, 90)
        .expect("set note");

    let xml = export_pattern_musicxml(&song, MusicXmlExportOptions::default()).expect("export");
    assert!(xml.contains("<score-partwise version=\"4.0\">"));
    assert!(xml.contains("<work-title>Round Trip</work-title>"));
    assert!(xml.contains("<part-name>Lead</part-name>"));

    let report = import_musicxml(&xml);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let imported = report.song.expect("song");
    assert_eq!(imported.metadata.title, "Round Trip");
    assert_eq!(imported.metadata.author, Some("Composer".to_string()));
    assert_eq!(imported.tracks[0].name, "Lead");
    assert_eq!(
        imported
            .current_pattern()
            .expect("pattern")
            .cell(0, 0)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert_eq!(
        imported
            .current_pattern()
            .expect("pattern")
            .cell(4, 0)
            .expect("cell")
            .note,
        Some(NoteEvent::Note { pitch: 64 })
    );
}

#[test]
fn imports_musicxml_subset_and_reports_unsupported_chords() {
    let xml = r#"<?xml version="1.0"?>
<score-partwise version="4.0">
  <work><work-title>Import Me</work-title></work>
  <part-list><score-part id="P1"><part-name>Piano</part-name></score-part></part-list>
  <part id="P1">
    <measure number="1">
      <attributes><divisions>4</divisions><time><beats>4</beats><beat-type>4</beat-type></time></attributes>
      <direction><direction-type><metronome><beat-unit>quarter</beat-unit><per-minute>96</per-minute></metronome></direction-type></direction>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>4</duration><velocity>88</velocity></note>
      <note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>4</duration></note>
      <note><rest/><duration>4</duration></note>
      <note><pitch><step>D</step><alter>1</alter><octave>4</octave></pitch><duration>4</duration></note>
    </measure>
  </part>
</score-partwise>"#;

    let report = import_musicxml(xml);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == MusicXmlDiagnosticKind::UnsupportedNotation
            && diagnostic.message.contains("chord")
    }));
    let song = report.song.expect("song");
    assert_eq!(song.metadata.title, "Import Me");
    assert_eq!(song.transport.bpm, 96);
    assert_eq!(song.tracks[0].name, "Piano");
    let pattern = song.current_pattern().expect("pattern");
    assert_eq!(pattern.cell(0, 0).expect("cell").velocity, Some(88));
    assert_eq!(
        pattern.cell(0, 0).expect("cell").note,
        Some(NoteEvent::Note { pitch: 60 })
    );
    assert_eq!(
        pattern.cell(8, 0).expect("cell").note,
        Some(NoteEvent::Note { pitch: 63 })
    );
}

#[test]
fn validates_musicxml_round_trip_survivability() {
    let mut song = Song::empty();
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(2, 1, NoteEvent::Note { pitch: 72 }, 0x70)
        .expect("set note");

    let report =
        validate_musicxml_round_trip(&song, MusicXmlExportOptions::default()).expect("validate");
    assert!(report.survived, "{:?}", report.diagnostics);
    assert_eq!(report.original_note_count, 1);
    assert_eq!(report.imported_note_count, 1);
}
