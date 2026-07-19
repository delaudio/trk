use super::*;

#[test]
fn inspects_mod_metadata_samples_and_effect_commands() {
    let mut bytes = vec![0_u8; 1084 + 64 * 4 * 4];
    bytes[0..10].copy_from_slice(b"Test Song ");
    bytes[20..24].copy_from_slice(b"Kick");
    bytes[42..44].copy_from_slice(&4_u16.to_be_bytes());
    bytes[950] = 1;
    bytes[952] = 0;
    bytes[1080..1084].copy_from_slice(b"M.K.");
    bytes[1084 + 2] = 0x0f;
    bytes[1084 + 3] = 0x01;

    let inspection = inspect_tracker_module(&bytes, TrackerModuleFormat::Mod);

    assert_eq!(inspection.title.as_deref(), Some("Test Song"));
    assert_eq!(inspection.channels, Some(4));
    assert_eq!(inspection.patterns, Some(1));
    assert_eq!(inspection.instrument_count, Some(31));
    assert_eq!(inspection.samples[0].name.as_deref(), Some("Kick"));
    assert_eq!(inspection.samples[0].length_bytes, Some(8));
    assert_eq!(inspection.effect_commands, vec![0x0f]);
    assert_eq!(
        inspection.recommendation,
        TrackerModuleRecommendation::SampleExtractionOnly
    );
    assert!(inspection.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TrackerModuleDiagnosticKind::UnsupportedTimingSemantics
    }));
}

#[test]
fn extracts_mod_samples_without_importing_song_data() {
    let mut bytes = vec![0_u8; 1084 + 64 * 4 * 4];
    bytes[0..10].copy_from_slice(b"Test Song ");
    bytes[20..24].copy_from_slice(b"Kick");
    bytes[42..44].copy_from_slice(&4_u16.to_be_bytes());
    bytes[50..55].copy_from_slice(b"Snare");
    bytes[72..74].copy_from_slice(&2_u16.to_be_bytes());
    bytes[950] = 1;
    bytes[1080..1084].copy_from_slice(b"M.K.");
    bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    bytes.extend_from_slice(&[9, 10, 11, 12]);

    let extraction = extract_tracker_module_samples(&bytes, TrackerModuleFormat::Mod);

    assert_eq!(extraction.samples.len(), 2);
    assert_eq!(extraction.samples[0].info.name.as_deref(), Some("Kick"));
    assert_eq!(extraction.samples[0].data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(extraction.samples[1].info.name.as_deref(), Some("Snare"));
    assert_eq!(extraction.samples[1].data, vec![9, 10, 11, 12]);
}

#[test]
fn sample_extraction_reports_truncated_mod_payloads() {
    let mut bytes = vec![0_u8; 1084 + 64 * 4 * 4];
    bytes[20..24].copy_from_slice(b"Kick");
    bytes[42..44].copy_from_slice(&4_u16.to_be_bytes());
    bytes[950] = 1;
    bytes[1080..1084].copy_from_slice(b"M.K.");
    bytes.extend_from_slice(&[1, 2, 3]);

    let extraction = extract_tracker_module_samples(&bytes, TrackerModuleFormat::Mod);

    assert!(extraction.samples.is_empty());
    assert!(extraction.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == TrackerModuleDiagnosticKind::MalformedModule
            && diagnostic.message.contains("truncated")
    }));
}

#[test]
fn inspects_xm_s3m_and_it_headers_without_full_import() {
    let mut xm = vec![0_u8; 80];
    xm[0..17].copy_from_slice(b"Extended Module: ");
    xm[17..25].copy_from_slice(b"XM Song ");
    xm[68..70].copy_from_slice(&8_u16.to_le_bytes());
    xm[70..72].copy_from_slice(&3_u16.to_le_bytes());
    xm[72..74].copy_from_slice(&2_u16.to_le_bytes());
    let xm = inspect_tracker_module(&xm, TrackerModuleFormat::Xm);
    assert_eq!(xm.title.as_deref(), Some("XM Song"));
    assert_eq!(xm.channels, Some(8));
    assert_eq!(xm.patterns, Some(3));
    assert_eq!(xm.instrument_count, Some(2));

    let mut s3m = vec![0xff_u8; 96];
    s3m[0..8].copy_from_slice(b"S3M Song");
    s3m[32..34].copy_from_slice(&4_u16.to_le_bytes());
    s3m[34..36].copy_from_slice(&5_u16.to_le_bytes());
    s3m[36..38].copy_from_slice(&6_u16.to_le_bytes());
    s3m[44..48].copy_from_slice(b"SCRM");
    s3m[64] = 0;
    s3m[65] = 1;
    let s3m = inspect_tracker_module(&s3m, TrackerModuleFormat::S3m);
    assert_eq!(s3m.title.as_deref(), Some("S3M Song"));
    assert_eq!(s3m.channels, Some(2));
    assert_eq!(s3m.patterns, Some(6));
    assert_eq!(s3m.instrument_count, Some(5));

    let mut it = vec![0xff_u8; 192];
    it[0..4].copy_from_slice(b"IMPM");
    it[4..11].copy_from_slice(b"IT Song");
    it[34..36].copy_from_slice(&7_u16.to_le_bytes());
    it[38..40].copy_from_slice(&9_u16.to_le_bytes());
    it[64] = 0;
    it[65] = 32;
    let it = inspect_tracker_module(&it, TrackerModuleFormat::It);
    assert_eq!(it.title.as_deref(), Some("IT Song"));
    assert_eq!(it.channels, Some(2));
    assert_eq!(it.patterns, Some(9));
    assert_eq!(it.instrument_count, Some(7));
}

#[test]
fn legacy_module_probe_reports_malformed_data() {
    let inspection = inspect_tracker_module(b"short", TrackerModuleFormat::Mod);

    assert!(inspection
        .diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.kind == TrackerModuleDiagnosticKind::MalformedModule }));
    assert!(inspection.samples.is_empty());
}

#[test]
fn rejects_unsupported_legacy_imports() {
    assert!(matches!(
        import_tracker_module(&[], TrackerModuleFormat::Xm),
        Err(InteropError::UnsupportedTrackerModule(
            TrackerModuleFormat::Xm
        ))
    ));
}
