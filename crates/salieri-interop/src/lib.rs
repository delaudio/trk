//! Import and export adapters for tracker and interchange formats.
//!
//! Format implementations are isolated so that adding an adapter does not
//! couple it to existing parsers. The crate-root re-exports preserve the
//! original public API.

pub mod diagnostics;
#[cfg(test)]
mod fixtures;
pub mod legacy;
pub mod midi;
mod shared;
pub mod xrns;

pub use diagnostics::*;
pub use legacy::{extract_tracker_module_samples, import_tracker_module, inspect_tracker_module};
pub use midi::{export_pattern_smf, import_smf, MidiExportOptions};
pub use xrns::{
    extract_xrns_sample_payloads, import_xrns, import_xrns_with_sample_paths, inspect_xrns,
};
