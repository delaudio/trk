use salieri_core::{model::ClipScene, PatternId, Song};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveBridgeOperation {
    Push,
    Pull,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LiveBridgeTarget {
    pub scene: Option<usize>,
    pub track: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveBridgePlan {
    pub operation: LiveBridgeOperation,
    pub actions: Vec<LiveBridgeAction>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveBridgeAction {
    pub scene: usize,
    pub track: usize,
    pub track_name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum LiveBridgeError {
    #[error("Ableton Live bridge is unavailable unless configured; rerun with --dry-run")]
    Unavailable,
    #[error("Ableton bridge scene out of range: {scene}")]
    SceneOutOfRange { scene: usize },
    #[error("Ableton bridge track out of range: {track}")]
    TrackOutOfRange { track: usize },
    #[error("Ableton bridge has no clip scenes to push")]
    NoClipScenes,
}

pub(crate) fn plan_live_bridge(
    song: &Song,
    operation: LiveBridgeOperation,
    target: LiveBridgeTarget,
    dry_run: bool,
) -> Result<LiveBridgePlan, LiveBridgeError> {
    if !dry_run {
        return Err(LiveBridgeError::Unavailable);
    }
    validate_target(song, operation, target)?;
    match operation {
        LiveBridgeOperation::Push => plan_push(song, target),
        LiveBridgeOperation::Pull => Ok(plan_pull(song, target)),
        LiveBridgeOperation::Clear => Ok(plan_clear(song, target)),
    }
}

pub(crate) fn format_live_bridge_plan(plan: &LiveBridgePlan) -> String {
    let label = match plan.operation {
        LiveBridgeOperation::Push => "push",
        LiveBridgeOperation::Pull => "pull",
        LiveBridgeOperation::Clear => "clear",
    };
    let mut lines = vec![format!(
        "Ableton Live bridge dry-run: {label} ({} action(s))",
        plan.actions.len()
    )];
    lines.extend(plan.actions.iter().map(|action| {
        format!(
            "- scene {:02}, track {:02} {}: {}",
            action.scene, action.track, action.track_name, action.description
        )
    }));
    lines.extend(
        plan.diagnostics
            .iter()
            .map(|diagnostic| format!("! {diagnostic}")),
    );
    lines.join("\n")
}

fn validate_target(
    song: &Song,
    operation: LiveBridgeOperation,
    target: LiveBridgeTarget,
) -> Result<(), LiveBridgeError> {
    if let (LiveBridgeOperation::Push, Some(scene)) = (operation, target.scene) {
        if scene >= song.clip_scenes.len() {
            return Err(LiveBridgeError::SceneOutOfRange { scene });
        }
    }
    if let Some(track) = target.track {
        if track >= song.tracks.len() {
            return Err(LiveBridgeError::TrackOutOfRange { track: track + 1 });
        }
    }
    Ok(())
}

fn plan_push(song: &Song, target: LiveBridgeTarget) -> Result<LiveBridgePlan, LiveBridgeError> {
    if song.clip_scenes.is_empty() {
        return Err(LiveBridgeError::NoClipScenes);
    }
    let mut actions = Vec::new();
    let mut diagnostics = Vec::new();
    for (scene_index, scene) in selected_scenes(song, target.scene) {
        for (track_index, track) in song.tracks.iter().enumerate() {
            if target.track.is_some_and(|selected| selected != track_index) {
                continue;
            }
            let Some(clip) = scene.clips.iter().find(|clip| clip.track == track.id) else {
                continue;
            };
            let pattern_name = pattern_name(song, clip.pattern);
            let end_row = clip
                .end_row
                .unwrap_or_else(|| pattern_rows(song, clip.pattern));
            actions.push(LiveBridgeAction {
                scene: scene_index,
                track: track_index + 1,
                track_name: track.name.clone(),
                description: format!(
                    "push pattern {pattern_name} rows {}..{} to session clip",
                    clip.start_row, end_row
                ),
            });
            if track.muted {
                diagnostics.push(format!(
                    "track {:02} {} is muted; pushed clip should stay muted in Live",
                    track_index + 1,
                    track.name
                ));
            }
        }
    }
    Ok(LiveBridgePlan {
        operation: LiveBridgeOperation::Push,
        actions,
        diagnostics,
    })
}

fn plan_pull(song: &Song, target: LiveBridgeTarget) -> LiveBridgePlan {
    let mut actions = Vec::new();
    for scene_index in live_scene_indices(song, target.scene) {
        for (track_index, track) in song.tracks.iter().enumerate() {
            if target.track.is_some_and(|selected| selected != track_index) {
                continue;
            }
            actions.push(LiveBridgeAction {
                scene: scene_index,
                track: track_index + 1,
                track_name: track.name.clone(),
                description: "pull session clip into a tracker pattern or clip slot".to_string(),
            });
        }
    }
    LiveBridgePlan {
        operation: LiveBridgeOperation::Pull,
        actions,
        diagnostics: vec![
            "dry-run only: no project patterns or clip launcher scenes were changed".to_string(),
        ],
    }
}

fn plan_clear(song: &Song, target: LiveBridgeTarget) -> LiveBridgePlan {
    let actions = live_scene_indices(song, target.scene)
        .into_iter()
        .flat_map(|scene_index| {
            song.tracks
                .iter()
                .enumerate()
                .filter(move |(track_index, _)| {
                    target.track.is_none_or(|selected| selected == *track_index)
                })
                .map(move |(track_index, track)| LiveBridgeAction {
                    scene: scene_index,
                    track: track_index + 1,
                    track_name: track.name.clone(),
                    description: "clear selected Ableton session clip".to_string(),
                })
        })
        .collect();
    LiveBridgePlan {
        operation: LiveBridgeOperation::Clear,
        actions,
        diagnostics: vec!["dry-run only: Ableton clips were not cleared".to_string()],
    }
}

fn live_scene_indices(song: &Song, scene: Option<usize>) -> Vec<usize> {
    if let Some(scene) = scene {
        return vec![scene];
    }
    if song.clip_scenes.is_empty() {
        vec![0]
    } else {
        (0..song.clip_scenes.len()).collect()
    }
}

fn selected_scenes(song: &Song, scene: Option<usize>) -> Vec<(usize, &ClipScene)> {
    match scene {
        Some(scene) => song
            .clip_scenes
            .get(scene)
            .map(|clip_scene| vec![(scene, clip_scene)])
            .unwrap_or_default(),
        None => song.clip_scenes.iter().enumerate().collect(),
    }
}

fn pattern_name(song: &Song, pattern_id: PatternId) -> String {
    song.patterns
        .iter()
        .find(|pattern| pattern.id == pattern_id)
        .map(|pattern| format!("{} ({})", pattern.name, pattern_id.0))
        .unwrap_or_else(|| format!("missing pattern {}", pattern_id.0))
}

fn pattern_rows(song: &Song, pattern_id: PatternId) -> usize {
    song.patterns
        .iter()
        .find(|pattern| pattern.id == pattern_id)
        .map_or(0, |pattern| pattern.rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use salieri_core::NoteEvent;

    #[test]
    fn push_dry_run_maps_clip_scenes_to_session_cells() {
        let mut song = Song::empty();
        song.patterns[0]
            .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
            .expect("note");
        song.create_clip_scene_from_pattern("Intro", 0)
            .expect("clip scene");

        let plan = plan_live_bridge(
            &song,
            LiveBridgeOperation::Push,
            LiveBridgeTarget {
                scene: Some(0),
                track: Some(0),
            },
            true,
        )
        .expect("plan");

        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].scene, 0);
        assert_eq!(plan.actions[0].track, 1);
        assert!(plan.actions[0].description.contains("Pattern 01"));
    }

    #[test]
    fn non_dry_run_fails_before_mutation_boundary() {
        let song = Song::empty();
        let error = plan_live_bridge(
            &song,
            LiveBridgeOperation::Clear,
            LiveBridgeTarget {
                scene: None,
                track: None,
            },
            false,
        )
        .expect_err("unconfigured bridge must fail");

        assert_eq!(error, LiveBridgeError::Unavailable);
    }

    #[test]
    fn clear_dry_run_can_target_live_scene_without_local_clip_scenes() {
        let song = Song::empty();
        let plan = plan_live_bridge(
            &song,
            LiveBridgeOperation::Clear,
            LiveBridgeTarget {
                scene: Some(7),
                track: Some(1),
            },
            true,
        )
        .expect("plan");

        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].scene, 7);
        assert_eq!(plan.actions[0].track, 2);
    }
}
