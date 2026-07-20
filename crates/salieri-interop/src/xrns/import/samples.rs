use std::collections::HashMap;

use salieri_core::{InstrumentId, SampleId, Song};

use crate::diagnostics::{
    XrnsDiagnostic, XrnsDiagnosticKind, XrnsDiagnosticSeverity, XrnsInspection,
};

use super::{apply_sample_metadata, sample_name, sample_payload_sample_index, XrnsImportModel};

pub(super) fn import_sample_references(
    song: &mut Song,
    model: &XrnsImportModel,
    inspection: &XrnsInspection,
    sample_path_overrides: &HashMap<String, String>,
    diagnostics: &mut Vec<XrnsDiagnostic>,
) -> HashMap<(InstrumentId, usize), SampleId> {
    let mut samples = HashMap::<(InstrumentId, usize), SampleId>::new();
    for sample in &inspection.sample_payloads {
        let Some(instrument) = super::super::sample_payload_instrument_id(&sample.path) else {
            continue;
        };
        let sample_index = sample_payload_sample_index(&sample.path).unwrap_or(0);
        if sample.supported || sample_path_overrides.contains_key(&sample.path) {
            let sample_path = sample_path_overrides
                .get(&sample.path)
                .map_or(sample.path.as_str(), String::as_str);
            let imported_name = model
                .sample_metadata(instrument, sample_index)
                .and_then(|metadata| metadata.name.as_deref())
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| sample_name(&sample.path));
            let sample_id = song.upsert_sample_reference(sample_path, imported_name);
            if let Some(metadata) = model.sample_metadata(instrument, sample_index) {
                if let Some(reference) = song.sample_for_id_mut(sample_id) {
                    apply_sample_metadata(reference, metadata);
                }
            }
            samples.insert((instrument, sample_index), sample_id);
        } else {
            diagnostics.push(super::super::xrns_diagnostic(
                XrnsDiagnosticKind::UnsupportedSampleFormat,
                XrnsDiagnosticSeverity::Warning,
                Some(sample.path.clone()),
                format!(
                    "instrument {:?} references unsupported sample format {}",
                    instrument, sample.format
                ),
            ));
        }
    }
    samples
}
