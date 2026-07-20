use super::*;

use crate::app::workspace_actions::workspace_report_artifact_path;
use crate::workflows::write_bytes_atomically;

impl App {
    pub(crate) fn handle_report_command(&mut self, values: &[&str]) {
        match values {
            [] | ["project"] | ["summary"] => self.show_project_report(),
            ["project" | "summary", "save", path @ ..] => self.save_report_file(
                path,
                "project report",
                "project-report.md",
                format_project_report,
            ),
            ["project" | "summary", "workspace", root @ ..] => self.save_workspace_report(
                root,
                "project report",
                "project-report.md",
                format_project_report,
            ),
            ["critique" | "review"] => self.show_critique_report(),
            ["critique" | "review", "save", path @ ..] => self.save_report_file(
                path,
                "critique report",
                "critique-report.md",
                format_critique_report,
            ),
            ["critique" | "review", "workspace", root @ ..] => self.save_workspace_report(
                root,
                "critique report",
                "critique-report.md",
                format_critique_report,
            ),
            ["revise" | "revision", prompt @ ..] => self.create_revision_proposal(prompt),
            _ => self.notify_warning(
                "Usage: :report project|critique [save PATH|workspace ROOT] | :revise PROMPT",
            ),
        }
    }

    fn show_project_report(&mut self) {
        let report = format_project_report(&self.song);
        let summary = project_report_summary(&self.song);
        self.push_ai_message(AiMessageRole::Assistant, report);
        self.notify_info(summary);
    }

    fn show_critique_report(&mut self) {
        let report = format_critique_report(&self.song);
        let summary = critique_report_summary(&self.song);
        self.push_ai_message(AiMessageRole::Assistant, report);
        self.notify_info(summary);
    }

    fn save_report_file(
        &mut self,
        path: &[&str],
        label: &str,
        default_file_name: &str,
        formatter: fn(&Song) -> String,
    ) {
        let Some(path) = report_command_path(path) else {
            self.notify_warning(format!("Usage: :report {label} save PATH"));
            return;
        };
        let output_path = if path.is_dir() {
            path.join(default_file_name)
        } else {
            path
        };
        self.write_report(&output_path, label, formatter);
    }

    fn save_workspace_report(
        &mut self,
        root: &[&str],
        label: &str,
        file_name: &str,
        formatter: fn(&Song) -> String,
    ) {
        let Some(root) = report_command_path(root) else {
            self.notify_warning(format!("Usage: :report {label} workspace ROOT"));
            return;
        };
        match workspace_report_artifact_path(&root, file_name) {
            Ok(path) => self.write_report(&path, label, formatter),
            Err(error) => self.notify_warning(format!("Report workspace save failed: {error}")),
        }
    }

    fn write_report(&mut self, output_path: &Path, label: &str, formatter: fn(&Song) -> String) {
        let report = formatter(&self.song);
        match write_bytes_atomically(output_path, report.as_bytes()) {
            Ok(()) => self.notify_success(format!("Saved {label}: {}", output_path.display())),
            Err(error) => self.notify_warning(format!("Report save failed: {error}")),
        }
    }

    fn create_revision_proposal(&mut self, prompt: &[&str]) {
        let request = if prompt.is_empty() {
            "add a focused revision based on the critique".to_string()
        } else {
            prompt.join(" ")
        };
        let critique = format_critique_report(&self.song);
        let prompt = format!(
            "Revision workflow. Keep changes reviewable and do not mutate directly.\n\n{critique}\nRevision request: {request}"
        );
        self.create_ai_proposal(prompt);
    }
}

fn report_command_path(values: &[&str]) -> Option<PathBuf> {
    let value = values.join(" ");
    let value = value.trim();
    (!value.is_empty()).then(|| PathBuf::from(value))
}
