mod import;

#[cfg(test)]
mod tests;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    io::Read,
};

use flate2::read::DeflateDecoder;
use salieri_core::InstrumentId;

use crate::{
    diagnostics::{
        XrnsArchiveEntry, XrnsDeviceChainInfo, XrnsDiagnostic, XrnsDiagnosticKind,
        XrnsDiagnosticSeverity, XrnsExtractedSample, XrnsImportReport, XrnsInspection,
        XrnsInstrumentInfo, XrnsPatternInfo, XrnsSamplePayload, XrnsTrackInfo,
    },
    shared::{read_le_u16_at, read_le_u32_at},
};

use self::import::{build_song_from_xrns_model, parse_xrns_import_model};

const ZIP_LOCAL_FILE_HEADER: u32 = 0x0403_4b50;
const ZIP_CENTRAL_DIRECTORY_HEADER: u32 = 0x0201_4b50;
const ZIP_END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;

#[must_use]
pub fn inspect_xrns(bytes: &[u8]) -> XrnsInspection {
    let mut inspection = XrnsInspection {
        is_zip: false,
        song_xml_path: None,
        archive_entries: Vec::new(),
        sample_payloads: Vec::new(),
        tracks: Vec::new(),
        patterns: Vec::new(),
        instruments: Vec::new(),
        device_chains: Vec::new(),
        diagnostics: Vec::new(),
    };

    let entries = match parse_zip_entries(bytes) {
        Ok(entries) => entries,
        Err(message) => {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::MalformedArchive,
                XrnsDiagnosticSeverity::Error,
                None,
                message,
            ));
            return inspection;
        }
    };

    inspection.is_zip = true;
    let mut song_xml = None;
    for entry in &entries {
        inspection.archive_entries.push(XrnsArchiveEntry {
            path: entry.path.clone(),
            compressed_size: entry.compressed_size,
            uncompressed_size: entry.uncompressed_size,
            compression_method: entry.compression_method,
            encrypted: entry.encrypted,
        });

        if entry.encrypted {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::EncryptedArchive,
                XrnsDiagnosticSeverity::Error,
                Some(entry.path.clone()),
                format!("encrypted XRNS entry is unsupported: {}", entry.path),
            ));
        }
        if is_nested_archive_path(&entry.path) {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::NestedArchive,
                XrnsDiagnosticSeverity::Warning,
                Some(entry.path.clone()),
                format!("nested archive entry will not be imported: {}", entry.path),
            ));
        }
        if let Some(sample) = sample_payload(entry) {
            if !sample.supported {
                inspection.diagnostics.push(xrns_diagnostic(
                    XrnsDiagnosticKind::UnsupportedSampleFormat,
                    XrnsDiagnosticSeverity::Warning,
                    Some(entry.path.clone()),
                    format!("sample payload is not a supported WAV file: {}", entry.path),
                ));
            }
            inspection.sample_payloads.push(sample);
        }
        if entry.path == "Song.xml" {
            inspection.song_xml_path = Some(entry.path.clone());
            song_xml = Some(entry);
        }
    }

    let Some(song_xml) = song_xml else {
        inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::MissingSongXml,
            XrnsDiagnosticSeverity::Error,
            None,
            "XRNS archive does not contain root Song.xml",
        ));
        return inspection;
    };

    let song_xml_data = match zip_entry_data(song_xml) {
        Ok(data) => data,
        Err(message) => {
            inspection.diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::UnsupportedCompression,
                XrnsDiagnosticSeverity::Error,
                Some(song_xml.path.clone()),
                message,
            ));
            return inspection;
        }
    };

    match std::str::from_utf8(&song_xml_data) {
        Ok(xml) => inspect_song_xml(xml, &mut inspection),
        Err(error) => inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::MalformedSongXml,
            XrnsDiagnosticSeverity::Error,
            Some(song_xml.path.clone()),
            format!("Song.xml is not valid UTF-8: {error}"),
        )),
    }

    inspection
}

#[must_use]
pub fn import_xrns(bytes: &[u8]) -> XrnsImportReport {
    import_xrns_with_sample_paths(bytes, &HashMap::new())
}

pub fn import_xrns_with_sample_paths(
    bytes: &[u8],
    sample_path_overrides: &HashMap<String, String>,
) -> XrnsImportReport {
    let inspection = inspect_xrns(bytes);
    let mut diagnostics = inspection.diagnostics.clone();
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == XrnsDiagnosticSeverity::Error)
    {
        return XrnsImportReport {
            song: None,
            inspection,
            diagnostics,
        };
    }

    let entries = match parse_zip_entries(bytes) {
        Ok(entries) => entries,
        Err(message) => {
            diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::MalformedArchive,
                XrnsDiagnosticSeverity::Error,
                None,
                message,
            ));
            return XrnsImportReport {
                song: None,
                inspection,
                diagnostics,
            };
        }
    };
    let song_xml = entries
        .iter()
        .find(|entry| entry.path == "Song.xml")
        .and_then(|entry| zip_entry_data(entry).ok());
    let Some(song_xml_data) = song_xml else {
        diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::UnsupportedCompression,
            XrnsDiagnosticSeverity::Error,
            Some("Song.xml".to_string()),
            "Song.xml could not be decoded from the XRNS archive",
        ));
        return XrnsImportReport {
            song: None,
            inspection,
            diagnostics,
        };
    };
    let song_xml = match std::str::from_utf8(&song_xml_data) {
        Ok(xml) => xml,
        Err(error) => {
            diagnostics.push(xrns_diagnostic(
                XrnsDiagnosticKind::MalformedSongXml,
                XrnsDiagnosticSeverity::Error,
                Some("Song.xml".to_string()),
                format!("Song.xml is not valid UTF-8: {error}"),
            ));
            return XrnsImportReport {
                song: None,
                inspection,
                diagnostics,
            };
        }
    };

    let Some(model) = parse_xrns_import_model(song_xml, &mut diagnostics) else {
        return XrnsImportReport {
            song: None,
            inspection,
            diagnostics,
        };
    };

    let song =
        build_song_from_xrns_model(&model, &inspection, sample_path_overrides, &mut diagnostics);
    let song = match song {
        Some(song) => match song.validate() {
            Ok(()) => Some(song),
            Err(error) => {
                diagnostics.push(xrns_diagnostic(
                    XrnsDiagnosticKind::ValidationFailed,
                    XrnsDiagnosticSeverity::Error,
                    None,
                    format!("imported XRNS project failed validation: {error}"),
                ));
                None
            }
        },
        None => None,
    };

    XrnsImportReport {
        song,
        inspection,
        diagnostics,
    }
}

pub fn extract_xrns_sample_payloads(bytes: &[u8]) -> Result<Vec<XrnsExtractedSample>, String> {
    let entries = parse_zip_entries(bytes)?;
    let mut samples = Vec::new();
    for entry in entries {
        let Some(payload) = sample_payload(&entry) else {
            continue;
        };
        let source_path = entry.path.clone();
        let data = zip_entry_data(&entry)?;
        samples.push(XrnsExtractedSample {
            source_path,
            format: payload.format,
            supported: payload.supported,
            bytes: data.into_owned(),
        });
    }
    Ok(samples)
}

#[derive(Debug, Clone, Default)]
struct ZipEntryRef<'a> {
    path: String,
    compressed_size: u32,
    uncompressed_size: u32,
    compression_method: u16,
    encrypted: bool,
    data: &'a [u8],
}

fn parse_zip_entries(bytes: &[u8]) -> Result<Vec<ZipEntryRef<'_>>, String> {
    if bytes.len() < 4 || read_le_u32_at(bytes, 0) != Some(ZIP_LOCAL_FILE_HEADER) {
        return Err("XRNS data is not a ZIP local-file-header stream".to_string());
    }

    let mut entries = Vec::new();
    let mut position = 0_usize;
    while position + 4 <= bytes.len() {
        let Some(signature) = read_le_u32_at(bytes, position) else {
            break;
        };
        if signature == ZIP_CENTRAL_DIRECTORY_HEADER || signature == ZIP_END_OF_CENTRAL_DIRECTORY {
            break;
        }
        if signature != ZIP_LOCAL_FILE_HEADER {
            return Err(format!("unexpected ZIP signature 0x{signature:08X}"));
        }
        if position + 30 > bytes.len() {
            return Err("truncated ZIP local file header".to_string());
        }

        let flags = read_le_u16_at(bytes, position + 6).expect("bounds checked");
        let compression_method = read_le_u16_at(bytes, position + 8).expect("bounds checked");
        let compressed_size = read_le_u32_at(bytes, position + 18).expect("bounds checked");
        let uncompressed_size = read_le_u32_at(bytes, position + 22).expect("bounds checked");
        let name_len = usize::from(read_le_u16_at(bytes, position + 26).expect("bounds checked"));
        let extra_len = usize::from(read_le_u16_at(bytes, position + 28).expect("bounds checked"));
        let name_start = position + 30;
        let name_end = name_start
            .checked_add(name_len)
            .ok_or_else(|| "ZIP entry name length overflow".to_string())?;
        let data_start = name_end
            .checked_add(extra_len)
            .ok_or_else(|| "ZIP extra field length overflow".to_string())?;
        let data_end = data_start
            .checked_add(compressed_size as usize)
            .ok_or_else(|| "ZIP entry data length overflow".to_string())?;
        if data_end > bytes.len() {
            return Err("truncated ZIP entry data".to_string());
        }

        let path = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|error| format!("ZIP entry name is not valid UTF-8: {error}"))?
            .replace('\\', "/");
        entries.push(ZipEntryRef {
            path,
            compressed_size,
            uncompressed_size,
            compression_method,
            encrypted: flags & 0x0001 != 0,
            data: &bytes[data_start..data_end],
        });
        position = data_end;
    }

    if entries.is_empty() {
        Err("XRNS ZIP archive has no entries".to_string())
    } else {
        Ok(entries)
    }
}

fn zip_entry_data<'a>(entry: &'a ZipEntryRef<'a>) -> Result<Cow<'a, [u8]>, String> {
    match entry.compression_method {
        0 => Ok(Cow::Borrowed(entry.data)),
        8 => {
            let mut decoder = DeflateDecoder::new(entry.data);
            let mut decoded = Vec::with_capacity(entry.uncompressed_size as usize);
            decoder.read_to_end(&mut decoded).map_err(|error| {
                format!("failed to decompress ZIP entry {}: {error}", entry.path)
            })?;
            Ok(Cow::Owned(decoded))
        }
        method => Err(format!(
            "{} uses unsupported ZIP compression method {method}",
            entry.path
        )),
    }
}

fn inspect_song_xml(xml: &str, inspection: &mut XrnsInspection) {
    match parse_xml_events(xml) {
        Ok(events) => inspect_xml_events(&events, inspection),
        Err(message) => inspection.diagnostics.push(xrns_diagnostic(
            XrnsDiagnosticKind::MalformedSongXml,
            XrnsDiagnosticSeverity::Error,
            Some("Song.xml".to_string()),
            message,
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum XmlEvent {
    Start(String),
    End(String),
    Text(String),
}

fn parse_xml_events(xml: &str) -> Result<Vec<XmlEvent>, String> {
    let mut events = Vec::new();
    let mut stack = Vec::new();
    let mut position = 0_usize;
    while let Some(relative_start) = xml[position..].find('<') {
        let start = position + relative_start;
        push_xml_text(&xml[position..start], &mut events);
        let Some(relative_end) = xml[start..].find('>') else {
            return Err("Song.xml contains an unterminated tag".to_string());
        };
        let end = start + relative_end;
        let raw_tag = xml[start + 1..end].trim();
        position = end + 1;

        if raw_tag.is_empty()
            || raw_tag.starts_with('?')
            || raw_tag.starts_with('!')
            || raw_tag.starts_with("!--")
        {
            continue;
        }
        if let Some(stripped) = raw_tag.strip_prefix('/') {
            let name = xml_tag_name(stripped);
            let Some(open) = stack.pop() else {
                return Err(format!("Song.xml closes unopened tag {name}"));
            };
            if open != name {
                return Err(format!("Song.xml closes tag {name} while {open} is open"));
            }
            events.push(XmlEvent::End(name));
        } else {
            let self_closing = raw_tag.ends_with('/');
            let name = xml_tag_name(raw_tag.trim_end_matches('/'));
            events.push(XmlEvent::Start(name.clone()));
            if self_closing {
                events.push(XmlEvent::End(name));
            } else {
                stack.push(name);
            }
        }
    }
    push_xml_text(&xml[position..], &mut events);

    if let Some(open) = stack.pop() {
        return Err(format!("Song.xml leaves tag {open} unclosed"));
    }
    Ok(events)
}

fn push_xml_text(text: &str, events: &mut Vec<XmlEvent>) {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        events.push(XmlEvent::Text(decode_xml_entities(trimmed)));
    }
}

fn xml_tag_name(raw_tag: &str) -> String {
    raw_tag
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches('/')
        .to_string()
}

fn inspect_xml_events(events: &[XmlEvent], inspection: &mut XrnsInspection) {
    let mut stack = Vec::<String>::new();
    let mut current_track: Option<XrnsTrackInfo> = None;
    let mut current_pattern: Option<XrnsPatternInfo> = None;
    let mut current_instrument: Option<XrnsInstrumentInfo> = None;
    let mut current_device_chain: Option<XrnsDeviceChainInfo> = None;
    let mut current_device: Option<String> = None;
    let mut reported_features = HashSet::new();

    for event in events {
        match event {
            XmlEvent::Start(name) => {
                if is_unsupported_feature_tag(name) && reported_features.insert(name.clone()) {
                    inspection.diagnostics.push(xrns_diagnostic(
                        XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                        XrnsDiagnosticSeverity::Warning,
                        Some(xml_location(&stack, name)),
                        format!("unsupported Renoise feature tag: {name}"),
                    ));
                }

                if name == "Track" && stack_contains(&stack, "Tracks") {
                    current_track = Some(XrnsTrackInfo {
                        index: inspection.tracks.len(),
                        name: None,
                    });
                } else if name == "Pattern" && stack_contains(&stack, "Patterns") {
                    current_pattern = Some(XrnsPatternInfo {
                        index: inspection.patterns.len(),
                        rows: None,
                    });
                } else if name == "Instrument" && stack_contains(&stack, "Instruments") {
                    current_instrument = Some(XrnsInstrumentInfo {
                        index: inspection.instruments.len(),
                        name: None,
                    });
                } else if name == "DeviceChain" {
                    current_device_chain = Some(XrnsDeviceChainInfo {
                        location: xml_location(&stack, name),
                        devices: Vec::new(),
                    });
                } else if name == "Device" && current_device_chain.is_some() {
                    current_device = Some(String::new());
                }
                stack.push(name.clone());
            }
            XmlEvent::End(name) => {
                if name == "Track" {
                    if let Some(track) = current_track.take() {
                        inspection.tracks.push(track);
                    }
                } else if name == "Pattern" {
                    if let Some(pattern) = current_pattern.take() {
                        inspection.patterns.push(pattern);
                    }
                } else if name == "Instrument" {
                    if let Some(instrument) = current_instrument.take() {
                        inspection.instruments.push(instrument);
                    }
                } else if name == "Device" {
                    if let Some(device) = current_device.take() {
                        let device = device.trim();
                        if !device.is_empty() {
                            if !is_supported_native_device(device) {
                                inspection.diagnostics.push(xrns_diagnostic(
                                    XrnsDiagnosticKind::UnsupportedRenoiseFeature,
                                    XrnsDiagnosticSeverity::Warning,
                                    Some(xml_location(&stack, name)),
                                    format!("unsupported Renoise device: {device}"),
                                ));
                            }
                            if let Some(chain) = &mut current_device_chain {
                                chain.devices.push(device.to_string());
                            }
                        }
                    }
                } else if name == "DeviceChain" {
                    if let Some(chain) = current_device_chain.take() {
                        inspection.device_chains.push(chain);
                    }
                }
                let _ = stack.pop();
            }
            XmlEvent::Text(text) => {
                let current = stack.last().map(String::as_str).unwrap_or_default();
                if current == "Name" {
                    if let Some(track) = &mut current_track {
                        track.name = Some(text.clone());
                    } else if let Some(instrument) = &mut current_instrument {
                        instrument.name = Some(text.clone());
                    } else if let Some(device) = &mut current_device {
                        *device = text.clone();
                    }
                } else if matches!(current, "NumberOfLines" | "Lines") {
                    if let Some(pattern) = &mut current_pattern {
                        if pattern.rows.is_none() {
                            pattern.rows = text.parse::<usize>().ok();
                        }
                    }
                } else if matches!(current, "Type" | "PluginIdentifier") {
                    if let Some(device) = &mut current_device {
                        *device = text.clone();
                    }
                }
            }
        }
    }
}

fn xrns_diagnostic(
    kind: XrnsDiagnosticKind,
    severity: XrnsDiagnosticSeverity,
    location: Option<String>,
    message: impl Into<String>,
) -> XrnsDiagnostic {
    XrnsDiagnostic {
        kind,
        severity,
        location,
        message: message.into(),
    }
}

fn sample_payload(entry: &ZipEntryRef<'_>) -> Option<XrnsSamplePayload> {
    let extension = entry.path.rsplit('.').next()?.to_ascii_lowercase();
    let is_sample = matches!(
        extension.as_str(),
        "wav" | "aif" | "aiff" | "flac" | "mp3" | "ogg"
    );
    is_sample.then_some(XrnsSamplePayload {
        path: entry.path.clone(),
        format: extension.clone(),
        bytes: entry.uncompressed_size,
        supported: extension == "wav",
    })
}

fn sample_payload_instrument_id(path: &str) -> Option<InstrumentId> {
    let segment = path
        .split('/')
        .find(|segment| segment.starts_with("Instrument"))?;
    let digits = segment
        .trim_start_matches("Instrument")
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let id = digits.parse::<u32>().ok()?;
    Some(InstrumentId(id))
}

fn is_nested_archive_path(path: &str) -> bool {
    path.rsplit('.')
        .next()
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "zip" | "xrns"))
}

fn is_unsupported_feature_tag(name: &str) -> bool {
    matches!(
        name,
        "PluginDevice"
            | "VstPlugin"
            | "VSTPlugin"
            | "AudioUnitPlugin"
            | "AuPlugin"
            | "MetaDevice"
            | "AutomationEnvelope"
            | "Phrase"
            | "Phrases"
    )
}

fn is_supported_native_device(device: &str) -> bool {
    let normalized = device.to_ascii_lowercase();
    normalized.contains("gain")
        || normalized.contains("gainer")
        || normalized.contains("volume")
        || normalized.contains("pan")
}

fn stack_contains(stack: &[String], name: &str) -> bool {
    stack.iter().any(|item| item == name)
}

fn xml_location(stack: &[String], name: &str) -> String {
    stack
        .iter()
        .chain(std::iter::once(&name.to_string()))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("/")
}

fn decode_xml_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
