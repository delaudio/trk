use super::*;

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
fn xrns_import_ignores_nested_sample_tags_inside_sample_metadata() {
    let xml = r#"
<RenoiseSong>
  <Tracks><Track><Name>Sampler</Name></Track></Tracks>
  <Patterns>
    <Pattern><NumberOfLines>1</NumberOfLines><Tracks><Track>
      <Line><Index>0</Index><Note>C-5</Note><Instrument>00</Instrument></Line>
    </Track></Tracks></Pattern>
  </Patterns>
  <Instruments>
    <Instrument>
      <Name>Lead</Name>
      <SampleGenerator>
        <Samples>
          <Sample>
            <Name>Real Sample</Name>
            <Volume>1.67880416</Volume>
            <Panning>0.333333</Panning>
            <NestedMetadata>
              <Device>
                <Name>Nested Gain Device</Name>
                <Gain>9.0</Gain>
              </Device>
              <Sample>
                <Name>Nested Non-Payload Sample</Name>
                <Volume>9.0</Volume>
              </Sample>
            </NestedMetadata>
          </Sample>
        </Samples>
      </SampleGenerator>
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
    let sample = song.samples.first().expect("imported sample");

    song.validate().expect("valid song");
    assert_eq!(song.samples.len(), 1);
    assert_eq!(sample.name, "Real Sample");
    assert_eq!(sample.gain, 1.679);
    assert_eq!(sample.pan, -0.333);
}

#[test]
fn xrns_import_preserves_multisample_keyzones() {
    let xml = r#"
<RenoiseSong>
  <Tracks><Track><Name>Sampler</Name></Track></Tracks>
  <Patterns>
    <Pattern><NumberOfLines>1</NumberOfLines><Tracks><Track>
      <Line><Index>0</Index><Note>C-4</Note><Instrument>00</Instrument></Line>
    </Track></Tracks></Pattern>
  </Patterns>
  <Instruments>
    <Instrument>
      <Name>Layered Piano</Name>
      <Samples>
        <Sample>
          <Name>Low layer</Name>
          <KeyStart>C-2</KeyStart><KeyEnd>B-3</KeyEnd>
          <VelocityStart>0</VelocityStart><VelocityEnd>80</VelocityEnd>
        </Sample>
        <Sample>
          <Name>High layer</Name>
          <KeyStart>C-4</KeyStart><KeyEnd>C-6</KeyEnd>
          <VelocityStart>81</VelocityStart><VelocityEnd>127</VelocityEnd>
        </Sample>
      </Samples>
    </Instrument>
  </Instruments>
</RenoiseSong>
"#;
    let archive = xrns_archive([
        xrns_entry("Song.xml", xml.as_bytes()),
        xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
        xrns_entry("SampleData/Instrument00/Sample01.wav", b"RIFF....WAVE"),
    ]);

    let report = import_xrns(&archive);
    let song = report.song.expect("imported song");

    song.validate().expect("valid song");
    assert_eq!(song.samples.len(), 2);
    assert_eq!(song.samples[0].name, "Low layer");
    assert_eq!(song.samples[1].name, "High layer");
    let instrument = song.instruments.first().expect("instrument");
    assert_eq!(instrument.name, "Layered Piano");
    assert_eq!(instrument.zones.len(), 2);
    assert_eq!(instrument.zones[0].sample, song.samples[0].id);
    assert_eq!(
        (instrument.zones[0].key_start, instrument.zones[0].key_end),
        (36, 59)
    );
    assert_eq!(
        (
            instrument.zones[1].key_start,
            instrument.zones[1].key_end,
            instrument.zones[1].velocity_start,
            instrument.zones[1].velocity_end,
        ),
        (60, 84, 81, 127)
    );
}

#[test]
fn xrns_import_defaults_missing_multisample_keyzones_to_full_range() {
    let xml = r#"
<RenoiseSong>
  <Instruments>
    <Instrument>
      <Name>Unmapped</Name>
      <Samples><Sample><Name>A</Name></Sample><Sample><Name>B</Name></Sample></Samples>
    </Instrument>
  </Instruments>
</RenoiseSong>
"#;
    let archive = xrns_archive([
        xrns_entry("Song.xml", xml.as_bytes()),
        xrns_entry("SampleData/Instrument00/Sample00.wav", b"RIFF....WAVE"),
        xrns_entry("SampleData/Instrument00/Sample01.wav", b"RIFF....WAVE"),
    ]);

    let report = import_xrns(&archive);
    let song = report.song.expect("imported song");

    song.validate().expect("valid song");
    let zones = &song.instruments[0].zones;
    assert_eq!(zones.len(), 2);
    assert!(zones.iter().all(|zone| {
        (
            zone.key_start,
            zone.key_end,
            zone.velocity_start,
            zone.velocity_end,
        ) == (0, 127, 0, 127)
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == XrnsDiagnosticKind::UnsupportedRenoiseFeature
            && diagnostic.message.contains("no keyzone mapping")
    }));
}
