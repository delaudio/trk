use trk_core::{model::DEFAULT_PATTERN_LEN, NoteEvent, PatternId, Song};

use super::*;
use crate::fixtures::hex_fixture;

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
    let bytes = hex_fixture(include_str!("../../../../fixtures/midi/simple-format0.hex"));
    let song = import_smf(&bytes).expect("import");
    let pattern = song.current_pattern().expect("pattern");
    let cell = pattern.cell(0, 1).expect("cell");

    assert_eq!(song.transport.bpm, 120);
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 60 }));
    assert_eq!(cell.velocity, Some(100));
}

#[test]
fn imports_format_one_multitrack_file() {
    let bytes = format_one_fixture();
    let song = import_smf(&bytes).expect("import");

    assert_eq!(song.transport.bpm, 120);
    assert_eq!(song.patterns.len(), 1);
    assert_eq!(song.sequence, vec![PatternId(1)]);
    let cell = song.pattern(0).expect("pattern").cell(0, 1).expect("cell");
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
fn imports_long_midi_into_sixty_four_row_patterns() {
    let mut song = Song::empty();
    song.resize_pattern(0, 128).expect("resize pattern");
    song.current_pattern_mut()
        .expect("pattern")
        .set_note(64, 1, NoteEvent::Note { pitch: 67 }, 96)
        .expect("set note");

    let bytes = export_pattern_smf(&song, MidiExportOptions::default()).expect("export");
    let imported = import_smf(&bytes).expect("import");

    assert_eq!(imported.patterns.len(), 2);
    assert_eq!(imported.sequence, vec![PatternId(1), PatternId(2)]);
    assert!(imported
        .patterns
        .iter()
        .all(|pattern| pattern.row_count() == DEFAULT_PATTERN_LEN));
    let cell = imported
        .pattern(1)
        .expect("second pattern")
        .cell(0, 1)
        .expect("cell");
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 67 }));
    assert_eq!(cell.velocity, Some(96));
}

#[test]
fn rejects_smpte_division() {
    let mut bytes = hex_fixture(include_str!("../../../../fixtures/midi/simple-format0.hex"));
    bytes[12] = 0xe7;
    bytes[13] = 0x28;

    assert!(matches!(
        import_smf(&bytes),
        Err(InteropError::UnsupportedSmpteDivision(0xe728))
    ));
}

#[test]
fn rejects_unsupported_midi_formats() {
    let mut bytes = hex_fixture(include_str!("../../../../fixtures/midi/simple-format0.hex"));
    bytes[9] = 2;

    assert!(matches!(
        import_smf(&bytes),
        Err(InteropError::UnsupportedMidiFormat(2))
    ));
}

fn format_one_fixture() -> Vec<u8> {
    let mut tempo_track = Vec::new();
    write_var_len(0, &mut tempo_track);
    tempo_track.extend_from_slice(&[0xff, 0x51, 0x03, 0x07, 0xa1, 0x20]);
    write_var_len(0, &mut tempo_track);
    tempo_track.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut note_track = Vec::new();
    write_var_len(0, &mut note_track);
    note_track.extend_from_slice(&[0xc0, 0x04]);
    write_var_len(0, &mut note_track);
    note_track.extend_from_slice(&[0xb0, 0x07, 0x64]);
    write_var_len(0, &mut note_track);
    note_track.extend_from_slice(&[0x90, 0x3c, 0x64]);
    write_var_len(480, &mut note_track);
    note_track.extend_from_slice(&[0x80, 0x3c, 0x00]);
    write_var_len(0, &mut note_track);
    note_track.extend_from_slice(&[0xff, 0x2f, 0x00]);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(MTHD);
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&2_u16.to_be_bytes());
    bytes.extend_from_slice(&480_u16.to_be_bytes());
    push_track_chunk(&mut bytes, &tempo_track);
    push_track_chunk(&mut bytes, &note_track);
    bytes
}

fn push_track_chunk(bytes: &mut Vec<u8>, track: &[u8]) {
    bytes.extend_from_slice(MTRK);
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(track);
}
