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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<InstrumentSampleZone>,
}

impl Instrument {
    #[must_use]
    pub fn primary_sample(&self) -> Option<SampleId> {
        self.sample
            .or_else(|| self.zones.first().map(|zone| zone.sample))
    }

    #[must_use]
    pub fn sample_for_note(&self, pitch: u8, velocity: u8) -> Option<SampleId> {
        self.zones
            .iter()
            .find(|zone| zone.contains(pitch, velocity))
            .map(|zone| zone.sample)
            .or_else(|| self.primary_sample())
    }

    pub fn sample_ids(&self) -> impl Iterator<Item = SampleId> + '_ {
        self.sample
            .into_iter()
            .chain(self.zones.iter().map(|zone| zone.sample))
    }

    #[must_use]
    pub fn references_sample(&self, sample: SampleId) -> bool {
        self.sample_ids().any(|candidate| candidate == sample)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentSampleZone {
    pub sample: SampleId,
    pub key_start: u8,
    pub key_end: u8,
    pub velocity_start: u8,
    pub velocity_end: u8,
}

impl InstrumentSampleZone {
    #[must_use]
    pub fn contains(&self, pitch: u8, velocity: u8) -> bool {
        (self.key_start..=self.key_end).contains(&pitch)
            && (self.velocity_start..=self.velocity_end).contains(&velocity)
    }
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

    #[test]
    fn instrument_zones_select_first_matching_sample() {
        let instrument = Instrument {
            id: InstrumentId(1),
            name: "Piano".to_string(),
            sample: None,
            zones: vec![
                InstrumentSampleZone {
                    sample: SampleId(1),
                    key_start: 48,
                    key_end: 72,
                    velocity_start: 0,
                    velocity_end: 127,
                },
                InstrumentSampleZone {
                    sample: SampleId(2),
                    key_start: 60,
                    key_end: 84,
                    velocity_start: 0,
                    velocity_end: 127,
                },
            ],
        };

        assert_eq!(instrument.sample_for_note(64, 100), Some(SampleId(1)));
        assert_eq!(instrument.sample_for_note(80, 100), Some(SampleId(2)));
        assert_eq!(instrument.primary_sample(), Some(SampleId(1)));
    }
}
