use serde::{Deserialize, Serialize};

use super::TrackId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongMetadata {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportSettings {
    pub bpm: u16,
    pub lines_per_beat: u8,
    pub swing: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MidiRoutingSettings {
    pub clock_in: bool,
    pub clock_out: bool,
    pub transport_in: bool,
    pub transport_out: bool,
    pub notes_in: bool,
    pub notes_out: bool,
    pub cc_in: bool,
    pub cc_out: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_channels: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_channels: Vec<u8>,
    pub middle_c: u8,
    pub clock_sync_delay_ms: i16,
    pub recording: MidiRecordingSettings,
}

impl MidiRoutingSettings {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl Default for MidiRoutingSettings {
    fn default() -> Self {
        Self {
            clock_in: false,
            clock_out: false,
            transport_in: false,
            transport_out: false,
            notes_in: true,
            notes_out: true,
            cc_in: false,
            cc_out: false,
            input_channels: Vec::new(),
            output_channels: Vec::new(),
            middle_c: 60,
            clock_sync_delay_ms: 0,
            recording: MidiRecordingSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MidiRecordingSettings {
    pub notes: bool,
    pub velocity: bool,
    pub cc: bool,
}

impl Default for MidiRecordingSettings {
    fn default() -> Self {
        Self {
            notes: true,
            velocity: true,
            cc: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub midi_channel: u8,
    pub muted: bool,
    pub solo: bool,
    pub armed: bool,
}
