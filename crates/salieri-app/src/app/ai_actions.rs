use super::*;

impl App {
    pub(crate) fn handle_ai_command(&mut self, values: &[&str]) {
        match values {
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
            _ => self
                .notify_warning("Usage: :ai propose PROMPT | :ai show | :ai accept | :ai reject"),
        }
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
        let mut result = Ok(Vec::new());
        self.mutate_song(|song, _| {
            result = apply_proposal(song, &pending.proposal).map(|preview| preview.touched_cells);
        });
        match result {
            Ok(touched_cells) => {
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
