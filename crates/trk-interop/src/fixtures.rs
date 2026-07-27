use std::io::Write;

use flate2::{write::DeflateEncoder, Compression};

const ZIP_LOCAL_FILE_HEADER: u32 = 0x0403_4b50;

pub(crate) fn hex_fixture(contents: &str) -> Vec<u8> {
    contents
        .split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).expect("hex byte"))
        .collect()
}

#[derive(Clone, Copy)]
pub(crate) struct XrnsTestEntry<'a> {
    pub(crate) path: &'a str,
    pub(crate) data: &'a [u8],
    pub(crate) flags: u16,
    pub(crate) compression_method: u16,
}

pub(crate) fn xrns_entry<'a>(path: &'a str, data: &'a [u8]) -> XrnsTestEntry<'a> {
    XrnsTestEntry {
        path,
        data,
        flags: 0,
        compression_method: 0,
    }
}

pub(crate) fn xrns_deflated_entry<'a>(path: &'a str, data: &'a [u8]) -> XrnsTestEntry<'a> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).expect("write compressed XML");
    let data = encoder.finish().expect("finish compressed XML");
    XrnsTestEntry {
        path,
        data: Box::leak(data.into_boxed_slice()),
        flags: 0,
        compression_method: 8,
    }
}

pub(crate) fn xrns_archive<'a>(entries: impl IntoIterator<Item = XrnsTestEntry<'a>>) -> Vec<u8> {
    let mut archive = Vec::new();
    for entry in entries {
        archive.extend_from_slice(&ZIP_LOCAL_FILE_HEADER.to_le_bytes());
        archive.extend_from_slice(&20_u16.to_le_bytes());
        archive.extend_from_slice(&entry.flags.to_le_bytes());
        archive.extend_from_slice(&entry.compression_method.to_le_bytes());
        archive.extend_from_slice(&0_u16.to_le_bytes());
        archive.extend_from_slice(&0_u16.to_le_bytes());
        archive.extend_from_slice(&0_u32.to_le_bytes());
        archive.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(entry.path.len() as u16).to_le_bytes());
        archive.extend_from_slice(&0_u16.to_le_bytes());
        archive.extend_from_slice(entry.path.as_bytes());
        archive.extend_from_slice(entry.data);
    }
    archive
}
