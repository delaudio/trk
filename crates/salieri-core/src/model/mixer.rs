use serde::{Deserialize, Serialize};

use crate::effect_kind::EffectDeviceKind;

use super::{Track, TrackId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixerState {
    pub master_gain: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<TrackMixerState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sends: Vec<MixerSend>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub master_effects: Vec<EffectDevice>,
}

impl MixerState {
    #[must_use]
    pub fn for_tracks(tracks: &[Track]) -> Self {
        Self {
            master_gain: 1.0,
            tracks: tracks
                .iter()
                .map(|track| TrackMixerState::default_for_track(track.id))
                .collect(),
            sends: Vec::new(),
            master_effects: Vec::new(),
        }
    }
}

impl Default for MixerState {
    fn default() -> Self {
        Self {
            master_gain: 1.0,
            tracks: Vec::new(),
            sends: Vec::new(),
            master_effects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackMixerState {
    pub track: TrackId,
    pub gain: f32,
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sends: Vec<TrackSendLevel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectDevice>,
}

impl TrackMixerState {
    #[must_use]
    pub fn default_for_track(track: TrackId) -> Self {
        Self {
            track,
            gain: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            sends: Vec::new(),
            effects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectDevice {
    pub id: u32,
    pub name: String,
    pub bypassed: bool,
    #[serde(flatten)]
    pub kind: EffectDeviceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixerSend {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSendLevel {
    pub send: u32,
    pub gain: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_state_builds_default_track_strips() {
        let tracks = vec![
            Track {
                id: TrackId(1),
                name: "Drums".to_string(),
                midi_channel: 0,
                muted: false,
                solo: false,
                armed: true,
            },
            Track {
                id: TrackId(2),
                name: "Bass".to_string(),
                midi_channel: 1,
                muted: false,
                solo: false,
                armed: false,
            },
        ];

        let mixer = MixerState::for_tracks(&tracks);

        assert_eq!(mixer.master_gain, 1.0);
        assert_eq!(mixer.tracks.len(), 2);
        assert_eq!(mixer.tracks[0].track, TrackId(1));
        assert_eq!(mixer.tracks[1].track, TrackId(2));
    }
}
