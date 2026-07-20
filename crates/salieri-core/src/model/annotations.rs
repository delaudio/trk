use serde::{Deserialize, Serialize};

use super::PatternId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextAnnotation {
    pub id: u32,
    pub kind: TextAnnotationKind,
    pub scope: TextAnnotationScope,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAnnotationKind {
    Note,
    Lyric,
    Cue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TextAnnotationScope {
    Project,
    Pattern {
        pattern: PatternId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        row: Option<usize>,
    },
    Sequence {
        position: usize,
    },
}
