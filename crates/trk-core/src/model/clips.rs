use serde::{Deserialize, Serialize};

use super::{PatternId, TrackId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipScene {
    pub id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<ClipSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSlot {
    pub track: TrackId,
    pub pattern: PatternId,
    #[serde(default)]
    pub start_row: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_row: Option<usize>,
}
