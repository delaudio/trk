use serde::{Deserialize, Serialize};

use crate::{ParameterId, ParameterValue};

use super::{InstrumentId, SampleId, TrackId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AutomationTarget {
    SampleGain { sample: SampleId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AutomationInterpolation {
    #[default]
    Step,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationLane {
    pub target: AutomationTarget,
    #[serde(default)]
    pub interpolation: AutomationInterpolation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<AutomationPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationPoint {
    pub row: usize,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParameterLockTarget {
    Sample { sample: SampleId },
    Instrument { instrument: InstrumentId },
    TrackMixer { track: TrackId },
    MasterMixer,
    TrackSend { track: TrackId, send: u32 },
    SendBus { send: u32 },
    TrackEffect { track: TrackId, device: u32 },
    MasterEffect { device: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ParameterLockAction {
    Set { value: ParameterValue },
    Reset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterLock {
    pub target: ParameterLockTarget,
    pub parameter: ParameterId,
    pub action: ParameterLockAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterLockDiagnostic {
    pub pattern_index: usize,
    pub row_index: usize,
    pub track_index: usize,
    pub target: ParameterLockTarget,
    pub parameter: ParameterId,
    pub message: String,
}
