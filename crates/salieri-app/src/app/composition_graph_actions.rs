use super::*;

impl App {
    pub(crate) fn handle_composition_graph_command(&mut self, values: &[&str]) {
        match values {
            ["draft" | "propose", prompt @ ..] => self.draft_composition_graph_command(prompt),
            ["show" | "preview"] => self.show_composition_graph_command(),
            ["reject" | "clear"] => self.reject_composition_graph_command(),
            ["apply" | "accept" | "compile"] => self.apply_composition_graph_command(),
            _ => self.notify_warning("Usage: :graph draft PROMPT | show | reject | apply"),
        }
    }

    fn draft_composition_graph_command(&mut self, prompt: &[&str]) {
        let graph = draft_composition_graph(&self.song, &prompt.join(" "));
        let preview = format_composition_graph_preview(&graph);
        self.pending_composition_graph = Some(graph);
        self.push_ai_message(AiMessageRole::Assistant, preview);
        self.notify_info("Composition graph draft ready; use :graph show/apply/reject");
    }

    fn show_composition_graph_command(&mut self) {
        let Some(graph) = &self.pending_composition_graph else {
            self.notify_warning("No pending composition graph");
            return;
        };
        let sections = graph.sections.len();
        let preview = format_composition_graph_preview(graph);
        self.push_ai_message(AiMessageRole::Assistant, preview);
        self.notify_info(format!("Composition graph pending: {sections} section(s)"));
    }

    fn reject_composition_graph_command(&mut self) {
        if self.pending_composition_graph.take().is_some() {
            self.push_ai_message(AiMessageRole::Progress, "Composition graph rejected");
            self.notify_info("Composition graph rejected");
        } else {
            self.notify_warning("No pending composition graph");
        }
    }

    fn apply_composition_graph_command(&mut self) {
        let Some(graph) = self.pending_composition_graph.clone() else {
            self.notify_warning("No pending composition graph");
            return;
        };
        let result = self.try_mutate_song(
            TransactionSpec::new("Apply composition graph"),
            |song, _| {
                *song = compile_composition_graph(song, &graph)?;
                Ok::<(), anyhow::Error>(())
            },
        );
        match result {
            Ok(_) => {
                self.pending_composition_graph = None;
                self.push_ai_message(AiMessageRole::Assistant, "Composition graph applied");
                self.notify_success("Composition graph applied to sequence");
            }
            Err(error) => self.notify_warning(format!("Composition graph apply failed: {error}")),
        }
    }
}
