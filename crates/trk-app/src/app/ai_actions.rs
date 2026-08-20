use super::*;
use crate::task_integration::format_ai_provider_status;

impl App {
    pub(crate) fn handle_ai_command(&mut self, values: &[&str]) {
        match values {
            ["chat"] | ["open"] => self.open_ai_chat_view(),
            [] | ["provider"] | ["status"] => {
                self.notify_info(format_ai_provider_status(&self.ai_config));
            }
            ["propose", prompt @ ..] | ["sketch", prompt @ ..] => {
                let prompt = prompt.join(" ");
                self.create_ai_proposal(prompt);
            }
            ["guidance", command @ ..] => self.handle_ai_guidance_command(command),
            ["show"] | ["preview"] => self.show_ai_proposal(),
            ["accept"] | ["apply"] => self.apply_ai_proposal(),
            ["reject"] | ["clear"] => self.reject_ai_proposal(),
            ["load"] => self.load_ai_session_command(),
            ["save"] => self.save_ai_session_command(),
            ["delete"] | ["forget"] => self.delete_ai_session_command(),
            ["retention", value] => match value.parse::<usize>() {
                Ok(retention_messages) => self.set_ai_retention_messages(retention_messages),
                Err(_) => self.notify_warning("Usage: :ai retention MESSAGES"),
            },
            _ => self.notify_warning(
                "Usage: :ai chat | :ai provider | :ai propose PROMPT | :ai guidance list/show/apply/clear | :ai show | :ai accept | :ai reject | :ai load/save/delete | :ai retention N",
            ),
        }
    }

    pub(crate) fn open_ai_chat_view(&mut self) {
        self.focus_panel(FocusPanel::Ai);
        self.notify_info(format_ai_provider_status(&self.ai_config));
    }

    pub(crate) fn submit_ai_chat_prompt(&mut self) {
        let prompt = self.ai_thread.composer.trim().to_string();
        if prompt.is_empty() {
            self.notify_warning("AI prompt cannot be empty");
            return;
        }
        self.ai_thread.composer.clear();
        self.create_ai_proposal(prompt);
    }

    pub(crate) fn show_ai_proposal(&mut self) {
        let Some(pending) = &self.pending_ai_proposal else {
            self.notify_warning("No pending AI proposal");
            return;
        };
        let summary = format_ai_proposal_summary(&pending.proposal, &pending.touched_cells);
        self.push_ai_message(AiMessageRole::Assistant, format!("Preview: {summary}"));
        self.notify_info(summary);
    }

    pub(crate) fn apply_ai_proposal(&mut self) {
        let Some(pending) = self.pending_ai_proposal.clone() else {
            self.notify_warning("No pending AI proposal");
            return;
        };
        let mut touched_cells = Vec::new();
        let original_song = self.song.clone();
        let mut next_variation_history = self.variation_history.clone();
        let result = self.try_mutate_song(TransactionSpec::new("Apply AI proposal"), |song, _| {
            touched_cells = apply_proposal(song, &pending.proposal)
                .map_err(|error| error.to_string())?
                .touched_cells;
            if song == &original_song {
                return Ok(());
            }
            let mut affected =
                std::collections::BTreeMap::<usize, std::collections::BTreeSet<usize>>::new();
            for cell in &touched_cells {
                affected.entry(cell.pattern).or_default().insert(cell.track);
            }
            for (pattern_index, tracks) in affected {
                let snapshot = song
                    .patterns
                    .get(pattern_index)
                    .cloned()
                    .ok_or_else(|| format!("pattern {} no longer exists", pattern_index + 1))?;
                let track_index = (tracks.len() == 1)
                    .then(|| tracks.into_iter().next().expect("one affected track"));
                next_variation_history
                    .record_now(
                        pending.proposal.prompt.clone(),
                        PatternVariationSource::AiProposal,
                        pattern_index,
                        track_index,
                        snapshot,
                    )
                    .map_err(|error| error.to_string())?;
            }
            Ok::<(), String>(())
        });
        match result {
            Ok(changed) => {
                if changed {
                    self.variation_history = next_variation_history;
                    self.refresh_dirty();
                }
                self.pending_ai_proposal = None;
                let summary = format!(
                    "AI proposal applied to {} cell(s): {}",
                    touched_cells.len(),
                    format_touched_cells(&touched_cells)
                );
                self.push_ai_message(AiMessageRole::Assistant, summary.clone());
                self.notify_success(summary);
            }
            Err(error) => self.notify_warning(format!("AI apply failed: {error}")),
        }
    }

    pub(crate) fn reject_ai_proposal(&mut self) {
        if self.pending_ai_proposal.take().is_some() {
            self.push_ai_message(AiMessageRole::Progress, "AI proposal rejected");
            self.notify_info("AI proposal rejected");
        } else {
            self.notify_warning("No pending AI proposal");
        }
    }

    pub(crate) fn ai_root_pitch(&self) -> u8 {
        u8::try_from((u16::from(self.octave) + 1).saturating_mul(12))
            .unwrap_or(127)
            .min(127)
    }
}
