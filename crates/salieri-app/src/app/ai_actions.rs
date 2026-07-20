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
            ["show"] | ["preview"] => self.show_ai_proposal(),
            ["accept"] | ["apply"] => self.apply_ai_proposal(),
            ["reject"] | ["clear"] => {
                if self.pending_ai_proposal.take().is_some() {
                    self.notify_info("AI proposal rejected");
                } else {
                    self.notify_warning("No pending AI proposal");
                }
            }
            _ => self.notify_warning(
                "Usage: :ai chat | :ai provider | :ai propose PROMPT | :ai show | :ai accept | :ai reject",
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
        self.notify_info(format_ai_proposal_summary(
            &pending.proposal,
            &pending.touched_cells,
        ));
    }

    pub(crate) fn apply_ai_proposal(&mut self) {
        let Some(pending) = self.pending_ai_proposal.clone() else {
            self.notify_warning("No pending AI proposal");
            return;
        };
        let mut touched_cells = Vec::new();
        let result = self.try_mutate_song(TransactionSpec::new("Apply AI proposal"), |song, _| {
            touched_cells = apply_proposal(song, &pending.proposal)?.touched_cells;
            Ok::<(), salieri_ai::AiError>(())
        });
        match result {
            Ok(_) => {
                self.pending_ai_proposal = None;
                self.notify_success(format!(
                    "AI proposal applied to {} cell(s): {}",
                    touched_cells.len(),
                    format_touched_cells(&touched_cells)
                ));
            }
            Err(error) => self.notify_warning(format!("AI apply failed: {error}")),
        }
    }

    pub(crate) fn ai_root_pitch(&self) -> u8 {
        u8::try_from((u16::from(self.octave) + 1).saturating_mul(12))
            .unwrap_or(127)
            .min(127)
    }
}
