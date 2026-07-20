use serde::{Deserialize, Serialize};

use super::{InstrumentId, SampleId, TrackId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleReference {
    pub id: SampleId,
    pub name: String,
    pub path: String,
    pub root_pitch: u8,
    pub gain: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub pan: f32,
    #[serde(default, skip_serializing_if = "is_zero_i8")]
    pub transpose_semitones: i8,
    #[serde(default, skip_serializing_if = "is_zero_i16")]
    pub fine_tune_cents: i16,
    #[serde(default, skip_serializing_if = "SamplePlaybackSettings::is_default")]
    pub playback: SamplePlaybackSettings,
}

fn is_zero_f32(value: &f32) -> bool {
    *value == 0.0
}
fn is_zero_i8(value: &i8) -> bool {
    *value == 0
}
fn is_zero_i16(value: &i16) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SamplePlaybackMode {
    #[default]
    OneShot,
    Loop,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleEnvelope {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain: f32,
    pub release_seconds: f32,
}

impl Default for SampleEnvelope {
    fn default() -> Self {
        Self {
            attack_seconds: 0.0,
            decay_seconds: 0.0,
            sustain: 1.0,
            release_seconds: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplePlaybackSettings {
    pub mode: SamplePlaybackMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_start_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_end_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "SampleEnvelope::is_default")]
    pub envelope: SampleEnvelope,
}

impl Default for SamplePlaybackSettings {
    fn default() -> Self {
        Self {
            mode: SamplePlaybackMode::OneShot,
            start_frame: None,
            end_frame: None,
            loop_start_frame: None,
            loop_end_frame: None,
            envelope: SampleEnvelope::default(),
        }
    }
}

impl SampleEnvelope {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl SamplePlaybackSettings {
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSampleAssignment {
    pub track: TrackId,
    pub sample: SampleId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instrument {
    pub id: InstrumentId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<SampleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInstrumentAssignment {
    pub track: TrackId,
    pub instrument: InstrumentId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_reference_omits_default_extended_metadata() {
        let sample = SampleReference {
            id: SampleId(1),
            name: "kick".to_string(),
            path: "samples/kick.wav".to_string(),
            root_pitch: 60,
            gain: 1.0,
            pan: 0.0,
            transpose_semitones: 0,
            fine_tune_cents: 0,
            playback: SamplePlaybackSettings::default(),
        };

        let value = serde_json::to_value(sample).expect("serialize sample");

        assert!(value.get("pan").is_none());
        assert!(value.get("transposeSemitones").is_none());
        assert!(value.get("fineTuneCents").is_none());
        assert!(value.get("playback").is_none());
    }
}
