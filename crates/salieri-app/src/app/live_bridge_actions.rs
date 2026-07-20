use super::*;
use crate::live_bridge::{
    format_live_bridge_plan, plan_live_bridge, LiveBridgeOperation, LiveBridgeTarget,
};

impl App {
    pub(crate) fn handle_live_bridge_command(&mut self, values: &[&str]) {
        let Some(operation) = values.first().and_then(|value| parse_operation(value)) else {
            self.notify_warning("Usage: :ableton push|pull|clear --dry-run [scene N] [track N]");
            return;
        };
        let options = parse_live_bridge_options(&values[1..]);
        let Ok((dry_run, target)) = options else {
            self.notify_warning("Usage: :ableton push|pull|clear --dry-run [scene N] [track N]");
            return;
        };
        match plan_live_bridge(&self.song, operation, target, dry_run) {
            Ok(plan) => {
                let action_count = plan.actions.len();
                let report = format_live_bridge_plan(&plan);
                self.push_ai_message(AiMessageRole::Assistant, report);
                self.notify_info(format!(
                    "Ableton bridge dry-run ready: {action_count} action(s)"
                ));
            }
            Err(error) => self.notify_warning(format!("Ableton bridge failed: {error}")),
        }
    }
}

fn parse_operation(value: &str) -> Option<LiveBridgeOperation> {
    match value {
        "push" | "export" => Some(LiveBridgeOperation::Push),
        "pull" | "import" => Some(LiveBridgeOperation::Pull),
        "clear" | "delete" => Some(LiveBridgeOperation::Clear),
        _ => None,
    }
}

fn parse_live_bridge_options(values: &[&str]) -> Result<(bool, LiveBridgeTarget), ()> {
    let mut dry_run = false;
    let mut scene = None;
    let mut track = None;
    let mut index = 0;
    while index < values.len() {
        match values[index] {
            "--dry-run" | "dry-run" | "dryrun" => {
                dry_run = true;
                index += 1;
            }
            "scene" | "--scene" => {
                let value = values
                    .get(index + 1)
                    .and_then(|value| value.parse::<usize>().ok())
                    .ok_or(())?;
                scene = Some(value);
                index += 2;
            }
            "track" | "--track" => {
                let value = values
                    .get(index + 1)
                    .and_then(|value| value.parse::<usize>().ok())
                    .ok_or(())?;
                track = Some(value.saturating_sub(1));
                index += 2;
            }
            _ => return Err(()),
        }
    }
    Ok((dry_run, LiveBridgeTarget { scene, track }))
}
