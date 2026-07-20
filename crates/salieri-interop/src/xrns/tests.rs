use salieri_core::{EffectDevice, InstrumentId, NoteEvent, SamplePlaybackMode, TrackerCommand};

use super::*;
use crate::fixtures::{xrns_archive, xrns_deflated_entry, xrns_entry, XrnsTestEntry};

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
fn xrns_inspector_reports_malformed_archive_and_unsupported_song_xml_compression() {
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
        compression_method: 99,
    }]);
    let inspection = inspect_xrns(&compressed);
    assert!(inspection
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.kind == XrnsDiagnosticKind::UnsupportedCompression }));
}

#[test]
fn xrns_import_accepts_deflated_song_xml() {
    let xml = r#"
<RenoiseSong>
  <Tracks><SequencerTrack type="SequencerTrack"><Name>Deflated</Name></SequencerTrack></Tracks>
  <PatternSequence><SequenceEntry><Pattern>0</Pattern></SequenceEntry></PatternSequence>
  <Patterns>
    <Pattern>
      <NumberOfLines>1</NumberOfLines>
      <Tracks>
        <PatternTrack type="PatternTrack">
          <Line><Index>0</Index><Note>C-4</Note><Volume>7F</Volume></Line>
        </PatternTrack>
      </Tracks>
    </Pattern>
  </Patterns>
</RenoiseSong>
"#;
    let archive = xrns_archive([xrns_deflated_entry("Song.xml", xml.as_bytes())]);
    let report = import_xrns(&archive);
    let song = report.song.expect("deflated Song.xml imports");

    assert_eq!(song.tracks[0].name, "Deflated");
    assert_eq!(song.patterns[0].rows.len(), 1);
}

#[test]
fn extracts_supported_xrns_wav_sample_payloads() {
    let archive = xrns_archive([
        xrns_entry("Song.xml", b"<RenoiseSong />"),
        xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
        xrns_entry("SampleData/Instrument01/Sample00.flac", b"fLaC"),
    ]);

    let samples = extract_xrns_sample_payloads(&archive).expect("extract samples");

    assert_eq!(samples.len(), 2);
    assert_eq!(
        samples[0].source_path,
        "SampleData/Instrument00/Sample00.wav"
    );
    assert_eq!(samples[0].format, "wav");
    assert!(samples[0].supported);
    assert_eq!(samples[0].bytes, b"RIFF....WAVE");
    assert_eq!(
        samples[1].source_path,
        "SampleData/Instrument01/Sample00.flac"
    );
    assert_eq!(samples[1].format, "flac");
    assert!(!samples[1].supported);
}

#[test]
fn imports_minimal_xrns_subset_to_valid_song() {
    let xml = include_str!("../../../../fixtures/xrns/minimal-song.xml");
    let archive = xrns_archive([
        xrns_entry("Song.xml", xml.as_bytes()),
        xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
    ]);

    let report = import_xrns(&archive);
    let song = report.song.expect("imported song");

    song.validate().expect("valid song");
    assert_eq!(song.transport.bpm, 172);
    assert_eq!(song.transport.lines_per_beat, 8);
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
    assert_eq!(cell.instrument, Some(InstrumentId(0)));
    assert_eq!(cell.volume, Some(64));
    assert_eq!(cell.pan, Some(127));
    assert_eq!(cell.delay, Some(32));
    assert_eq!(cell.command, Some(TrackerCommand::retrigger(4)));
    assert_eq!(song.samples.len(), 1);
    assert_eq!(song.instruments.len(), 1);
}

#[test]
fn xrns_import_preserves_sample_playback_metadata() {
    let xml = r#"
<RenoiseSong>
  <Tracks><Track><Name>Sampler</Name></Track></Tracks>
  <Patterns>
    <Pattern>
      <NumberOfLines>4</NumberOfLines>
      <Tracks>
        <Track>
          <Line><Index>0</Index><Note>C-5</Note><Instrument>00</Instrument></Line>
        </Track>
      </Tracks>
    </Pattern>
  </Patterns>
  <Instruments>
    <Instrument>
      <Name>Lead</Name>
      <Samples>
        <Sample>
          <Name>Lead C5</Name>
          <BaseNote>C-5</BaseNote>
          <Transpose>-12</Transpose>
          <FineTune>25</FineTune>
          <Volume>0.625</Volume>
          <Panning>0.25</Panning>
          <LoopMode>Forward</LoopMode>
          <LoopStart>100</LoopStart>
          <LoopEnd>900</LoopEnd>
          <Attack>0.01</Attack>
          <Decay>0.20</Decay>
          <Sustain>80</Sustain>
          <Release>0.50</Release>
          <InterpolationMode>Cubic</InterpolationMode>
        </Sample>
      </Samples>
    </Instrument>
  </Instruments>
</RenoiseSong>
"#;
    let archive = xrns_archive([
        xrns_entry("Song.xml", xml.as_bytes()),
        xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
    ]);

    let report = import_xrns(&archive);
    let song = report.song.expect("imported song");

    song.validate().expect("valid song");
    let sample = song.samples.first().expect("imported sample");
    assert_eq!(sample.name, "Lead C5");
    assert_eq!(sample.root_pitch, 72);
    assert_eq!(sample.transpose_semitones, -12);
    assert_eq!(sample.fine_tune_cents, 25);
    assert_eq!(sample.gain, 0.625);
    assert_eq!(sample.pan, -0.5);
    assert_eq!(sample.playback.mode, SamplePlaybackMode::Loop);
    assert_eq!(sample.playback.loop_start_frame, Some(100));
    assert_eq!(sample.playback.loop_end_frame, Some(900));
    assert!((sample.playback.envelope.attack_seconds - 0.01).abs() < f32::EPSILON);
    assert!((sample.playback.envelope.decay_seconds - 0.20).abs() < f32::EPSILON);
    assert!((sample.playback.envelope.sustain - 0.80).abs() < f32::EPSILON);
    assert!((sample.playback.envelope.release_seconds - 0.50).abs() < f32::EPSILON);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == XrnsDiagnosticKind::UnsupportedRenoiseFeature
            && diagnostic.message.contains("InterpolationMode")
    }));
}

#[test]
fn xrns_import_reads_note_column_values_as_renoise_hex_strings() {
    let xml = r#"
<RenoiseSong>
  <Tracks><Track><Name>Drums</Name></Track></Tracks>
  <Patterns>
    <Pattern>
      <NumberOfLines>4</NumberOfLines>
      <Tracks>
        <Track>
          <Line>
            <Index>0</Index>
            <Note>C-4</Note>
            <Instrument>10</Instrument>
            <Volume>80</Volume>
            <Pan>40</Pan>
            <Delay>0A</Delay>
            <Effect><Code>D</Code><Value>10</Value></Effect>
          </Line>
        </Track>
      </Tracks>
    </Pattern>
  </Patterns>
</RenoiseSong>
"#;
    let archive = xrns_archive([xrns_entry("Song.xml", xml.as_bytes())]);

    let report = import_xrns(&archive);
    let song = report.song.expect("imported song");
    let cell = song.patterns[0].cell(0, 0).expect("cell");

    assert_eq!(cell.instrument, Some(InstrumentId(16)));
    assert_eq!(cell.volume, Some(127));
    assert_eq!(cell.pan, Some(64));
    assert_eq!(cell.delay, Some(10));
    assert_eq!(cell.command, Some(TrackerCommand::delay(16)));
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
    assert_eq!(cell.instrument, Some(InstrumentId(1)));
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
            && diagnostic.message.contains("Comb Filter")
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
