use std::collections::HashSet;

use salieri_core::Song;

use crate::{
    diagnostics::{
        ExtractedTrackerModuleSample, InteropError, TrackerModuleDiagnostic,
        TrackerModuleDiagnosticKind, TrackerModuleFormat, TrackerModuleInspection,
        TrackerModuleRecommendation, TrackerModuleSampleExtraction, TrackerModuleSampleInfo,
    },
    shared::read_le_u16_at,
};

#[cfg(test)]
mod tests;

pub fn import_tracker_module(
    _bytes: &[u8],
    format: TrackerModuleFormat,
) -> Result<Song, InteropError> {
    Err(InteropError::UnsupportedTrackerModule(format))
}

#[must_use]
pub fn inspect_tracker_module(
    bytes: &[u8],
    format: TrackerModuleFormat,
) -> TrackerModuleInspection {
    match format {
        TrackerModuleFormat::Mod => inspect_mod_module(bytes),
        TrackerModuleFormat::Xm => inspect_xm_module(bytes),
        TrackerModuleFormat::S3m => inspect_s3m_module(bytes),
        TrackerModuleFormat::It => inspect_it_module(bytes),
        TrackerModuleFormat::Renoise => TrackerModuleInspection {
            format,
            title: None,
            channels: None,
            patterns: None,
            samples: Vec::new(),
            instrument_count: None,
            effect_commands: Vec::new(),
            diagnostics: vec![tracker_module_diagnostic(
                TrackerModuleDiagnosticKind::MalformedModule,
                "XRNS is handled by inspect_xrns/import_xrns, not legacy module inspection",
            )],
            recommendation: TrackerModuleRecommendation::SampleExtractionOnly,
        },
    }
}

#[must_use]
pub fn extract_tracker_module_samples(
    bytes: &[u8],
    format: TrackerModuleFormat,
) -> TrackerModuleSampleExtraction {
    let inspection = inspect_tracker_module(bytes, format);
    let mut diagnostics = inspection.diagnostics.clone();
    let samples = match format {
        TrackerModuleFormat::Mod => extract_mod_samples(bytes, &inspection, &mut diagnostics),
        TrackerModuleFormat::Xm | TrackerModuleFormat::S3m | TrackerModuleFormat::It => {
            diagnostics.push(tracker_module_diagnostic(
                TrackerModuleDiagnosticKind::EffectDecodeIncomplete,
                format!(
                    "{format:?} sample extraction requires instrument/sample offset table decoding"
                ),
            ));
            Vec::new()
        }
        TrackerModuleFormat::Renoise => Vec::new(),
    };
    TrackerModuleSampleExtraction {
        inspection,
        samples,
        diagnostics,
    }
}

fn inspect_mod_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 1084 {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "MOD data is too short for a 31-sample header",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::Mod,
            None,
            None,
            None,
            Vec::new(),
            Some(0),
            Vec::new(),
            diagnostics,
        );
    }

    let signature = &bytes[1080..1084];
    let channels = match signature {
        b"M.K." | b"M!K!" | b"4CHN" => Some(4),
        b"6CHN" => Some(6),
        b"8CHN" => Some(8),
        _ => None,
    };
    let song_len = usize::from(bytes[950]).min(128);
    let patterns = bytes[952..1080]
        .iter()
        .take(song_len)
        .copied()
        .max()
        .map(|pattern| usize::from(pattern) + 1);
    let samples = (0..31)
        .map(|index| {
            let offset = 20 + index * 30;
            let name = clean_ascii(&bytes[offset..offset + 22]);
            let length_words = u16::from_be_bytes([bytes[offset + 22], bytes[offset + 23]]);
            TrackerModuleSampleInfo {
                index,
                name,
                length_bytes: Some(usize::from(length_words) * 2),
            }
        })
        .collect::<Vec<_>>();

    let mut effect_commands = Vec::new();
    if let (Some(channels), Some(patterns)) = (channels, patterns) {
        let pattern_data_start = 1084;
        let pattern_bytes = patterns
            .saturating_mul(64)
            .saturating_mul(channels)
            .saturating_mul(4);
        if bytes.len() >= pattern_data_start + pattern_bytes {
            let mut commands = HashSet::new();
            for event in bytes[pattern_data_start..pattern_data_start + pattern_bytes].chunks(4) {
                let command = event[2] & 0x0f;
                if command != 0 {
                    commands.insert(command);
                }
            }
            effect_commands = commands.into_iter().collect();
            effect_commands.sort_unstable();
        } else {
            diagnostics.push(tracker_module_diagnostic(
                TrackerModuleDiagnosticKind::MalformedModule,
                "MOD pattern data is truncated",
            ));
        }
    }

    tracker_module_inspection(
        TrackerModuleFormat::Mod,
        clean_ascii(&bytes[0..20]),
        channels,
        patterns,
        samples,
        Some(31),
        effect_commands,
        diagnostics,
    )
}

fn extract_mod_samples(
    bytes: &[u8],
    inspection: &TrackerModuleInspection,
    diagnostics: &mut Vec<TrackerModuleDiagnostic>,
) -> Vec<ExtractedTrackerModuleSample> {
    let Some(channels) = inspection.channels else {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "cannot extract MOD samples without a recognized channel signature",
        ));
        return Vec::new();
    };
    let Some(patterns) = inspection.patterns else {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "cannot extract MOD samples without a pattern count",
        ));
        return Vec::new();
    };
    let sample_data_start = 1084_usize.saturating_add(
        patterns
            .saturating_mul(64)
            .saturating_mul(channels)
            .saturating_mul(4),
    );
    if sample_data_start > bytes.len() {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "MOD sample data starts beyond the available bytes",
        ));
        return Vec::new();
    }

    let mut offset = sample_data_start;
    let mut extracted = Vec::new();
    for sample in &inspection.samples {
        let length = sample.length_bytes.unwrap_or(0);
        if length == 0 {
            continue;
        }
        let end = offset.saturating_add(length);
        if end > bytes.len() {
            diagnostics.push(tracker_module_diagnostic(
                TrackerModuleDiagnosticKind::MalformedModule,
                format!("MOD sample {} is truncated", sample.index),
            ));
            break;
        }
        extracted.push(ExtractedTrackerModuleSample {
            info: sample.clone(),
            data: bytes[offset..end].to_vec(),
        });
        offset = end;
    }
    extracted
}

fn inspect_xm_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 80 || !bytes.starts_with(b"Extended Module: ") {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "XM data is missing the Extended Module header",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::Xm,
            None,
            None,
            None,
            Vec::new(),
            None,
            Vec::new(),
            diagnostics,
        );
    }
    tracker_module_inspection(
        TrackerModuleFormat::Xm,
        clean_ascii(&bytes[17..37]),
        read_le_u16_at(bytes, 68).map(usize::from),
        read_le_u16_at(bytes, 70).map(usize::from),
        Vec::new(),
        read_le_u16_at(bytes, 72).map(usize::from),
        Vec::new(),
        diagnostics,
    )
}

fn inspect_s3m_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 96 || bytes.get(44..48) != Some(&b"SCRM"[..]) {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "S3M data is missing the SCRM signature",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::S3m,
            None,
            None,
            None,
            Vec::new(),
            None,
            Vec::new(),
            diagnostics,
        );
    }
    let channels = bytes[64..96]
        .iter()
        .filter(|channel| **channel < 16)
        .count();
    tracker_module_inspection(
        TrackerModuleFormat::S3m,
        clean_ascii(&bytes[0..28]),
        Some(channels),
        read_le_u16_at(bytes, 36).map(usize::from),
        Vec::new(),
        read_le_u16_at(bytes, 34).map(usize::from),
        Vec::new(),
        diagnostics,
    )
}

fn inspect_it_module(bytes: &[u8]) -> TrackerModuleInspection {
    let mut diagnostics = legacy_module_diagnostics();
    if bytes.len() < 192 || !bytes.starts_with(b"IMPM") {
        diagnostics.push(tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::MalformedModule,
            "IT data is missing the IMPM signature",
        ));
        return tracker_module_inspection(
            TrackerModuleFormat::It,
            None,
            None,
            None,
            Vec::new(),
            None,
            Vec::new(),
            diagnostics,
        );
    }
    let channels = bytes[64..128].iter().filter(|pan| **pan != 0xff).count();
    tracker_module_inspection(
        TrackerModuleFormat::It,
        clean_ascii(&bytes[4..30]),
        Some(channels),
        read_le_u16_at(bytes, 38).map(usize::from),
        Vec::new(),
        read_le_u16_at(bytes, 34).map(usize::from),
        Vec::new(),
        diagnostics,
    )
}

#[allow(clippy::too_many_arguments)]
fn tracker_module_inspection(
    format: TrackerModuleFormat,
    title: Option<String>,
    channels: Option<usize>,
    patterns: Option<usize>,
    samples: Vec<TrackerModuleSampleInfo>,
    instrument_count: Option<usize>,
    effect_commands: Vec<u8>,
    diagnostics: Vec<TrackerModuleDiagnostic>,
) -> TrackerModuleInspection {
    TrackerModuleInspection {
        format,
        title,
        channels,
        patterns,
        samples,
        instrument_count,
        effect_commands,
        diagnostics,
        recommendation: TrackerModuleRecommendation::SampleExtractionOnly,
    }
}

fn legacy_module_diagnostics() -> Vec<TrackerModuleDiagnostic> {
    vec![
        tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::UnsupportedTimingSemantics,
            "legacy tracker tick tempo, speed changes, and row effects do not map losslessly to Salieri row timing",
        ),
        tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::UnsupportedEffectMemory,
            "legacy tracker effect memory and per-channel playback state are not represented in Salieri pattern cells",
        ),
        tracker_module_diagnostic(
            TrackerModuleDiagnosticKind::EffectDecodeIncomplete,
            "probe reports effect command numbers but does not implement player-compatible effect semantics",
        ),
    ]
}

fn tracker_module_diagnostic(
    kind: TrackerModuleDiagnosticKind,
    message: impl Into<String>,
) -> TrackerModuleDiagnostic {
    TrackerModuleDiagnostic {
        kind,
        message: message.into(),
    }
}

fn clean_ascii(bytes: &[u8]) -> Option<String> {
    let text = bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0 && (byte.is_ascii_graphic() || *byte == b' '))
        .map(char::from)
        .collect::<String>();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}
