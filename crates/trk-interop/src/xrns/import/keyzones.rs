use trk_core::{InstrumentId, InstrumentSampleZone, SampleId};

use crate::diagnostics::{XrnsDiagnostic, XrnsDiagnosticKind, XrnsDiagnosticSeverity};

use super::model::XrnsImportModel;

pub(super) fn parse_keyzone_note(value: &str) -> Option<u8> {
    super::parse_u8_value(value)
        .filter(|pitch| *pitch <= 127)
        .or_else(|| super::parse_note_name(value))
}

pub(super) fn parse_keyzone_velocity(value: &str) -> Option<u8> {
    super::parse_u8_value(value).map(|velocity| velocity.min(127))
}

pub(super) fn instrument_zones(
    model: &XrnsImportModel,
    instrument: InstrumentId,
    samples: &[(usize, SampleId)],
    diagnostics: &mut Vec<XrnsDiagnostic>,
) -> Vec<InstrumentSampleZone> {
    if samples.len() <= 1
        && !samples.iter().any(|(sample_index, _)| {
            model
                .sample_metadata(instrument, *sample_index)
                .is_some_and(|metadata| metadata.has_keyzone_mapping())
        })
    {
        return Vec::new();
    }

    samples
        .iter()
        .map(|(sample_index, sample)| {
            let metadata = model.sample_metadata(instrument, *sample_index);
            if !metadata.is_some_and(|metadata| metadata.has_keyzone_mapping()) {
                diagnostics.push(super::super::xrns_diagnostic(
                    XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                    XrnsDiagnosticSeverity::Warning,
                    Some(format!("Instrument{:02}/Sample{:02}", instrument.0, sample_index)),
                    format!(
                        "instrument {:?} sample {sample_index} has no keyzone mapping; defaulting to full range",
                        instrument
                    ),
                ));
            }
            let key = ordered_range(
                metadata.and_then(|metadata| metadata.key_start).unwrap_or(0),
                metadata.and_then(|metadata| metadata.key_end).unwrap_or(127),
            );
            let velocity = ordered_range(
                metadata
                    .and_then(|metadata| metadata.velocity_start)
                    .unwrap_or(0),
                metadata
                    .and_then(|metadata| metadata.velocity_end)
                    .unwrap_or(127),
            );
            InstrumentSampleZone {
                sample: *sample,
                key_start: key.0,
                key_end: key.1,
                velocity_start: velocity.0,
                velocity_end: velocity.1,
            }
        })
        .collect()
}

fn ordered_range(start: u8, end: u8) -> (u8, u8) {
    (start.min(end), start.max(end))
}
