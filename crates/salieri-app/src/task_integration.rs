#[cfg(test)]
use std::{thread, time::Duration};

use salieri_ai::{
    preview_proposal, AiPatternRequest, AiProposalProvider, LocalDeterministicProvider,
};

use crate::{
    app_effect::AppEffect,
    app_event::{AiIntent, AppEvent, AppIntent, AppTaskResult, PreparedAiProposal, RuntimeEvent},
    command::TaskCommand,
    format_ai_proposal_summary,
    task_runtime::{TaskDiagnostic, TaskFailure, TaskId, TaskProgress, TaskSnapshot, TaskUpdate},
    App, DEFAULT_NOTE_VELOCITY,
};

impl App {
    pub(super) fn create_ai_proposal(&mut self, prompt: String) {
        self.dispatch_intent(AppIntent::Ai(AiIntent::Propose(prompt)));
    }

    pub(crate) fn prepare_ai_proposal_effect(&mut self, prompt: String) -> Option<AppEffect> {
        let Some(pattern) = self.song.pattern(self.pattern_index) else {
            self.notify_warning("Pattern out of range");
            return None;
        };
        let request = AiPatternRequest {
            prompt,
            pattern: self.pattern_index,
            track: self.cursor.track,
            rows: pattern.row_count(),
            root_pitch: self.ai_root_pitch(),
            velocity: DEFAULT_NOTE_VELOCITY,
        };
        Some(AppEffect::SubmitAiProposal {
            song: self.song.clone(),
            request,
        })
    }

    pub(crate) fn submit_ai_proposal(
        &mut self,
        song: salieri_core::Song,
        request: AiPatternRequest,
    ) {
        let id = self.task_runtime.submit(
            "AI proposal",
            Box::new(move |context| {
                context.report_progress(TaskProgress::new(0, Some(2), "generating proposal"));
                let proposal = LocalDeterministicProvider
                    .propose(&song, &request)
                    .map_err(|error| TaskFailure::error(format!("AI proposal failed: {error}")))?;
                context.check_cancelled()?;
                context.report_progress(TaskProgress::new(1, Some(2), "validating preview"));
                let touched_cells = preview_proposal(&song, &proposal)
                    .map_err(|error| TaskFailure::error(format!("AI preview failed: {error}")))?
                    .touched_cells;
                context.check_cancelled()?;
                context.report_progress(TaskProgress::new(2, Some(2), "proposal ready"));
                Ok(AppTaskResult::AiProposal(PreparedAiProposal {
                    proposal,
                    touched_cells,
                }))
            }),
        );
        self.notify_info(format!("Task #{id} queued: AI proposal"));
    }

    pub(super) fn drain_task_updates(&mut self) {
        for update in self.task_runtime.drain_updates() {
            self.dispatch_event(AppEvent::Runtime(RuntimeEvent::TaskUpdate(update)));
        }
    }

    pub(super) fn apply_task_update(&mut self, update: TaskUpdate<AppTaskResult>) {
        let id = update.id();
        let name = self.task_name(id);
        match update {
            TaskUpdate::Started { .. } => self.notify_info(format!("Task #{id} running: {name}")),
            TaskUpdate::Progress { progress, .. } => {
                let percentage = progress
                    .percentage()
                    .map_or_else(String::new, |value| format!(" {value}%"));
                let detail = progress.message.unwrap_or_else(|| "working".to_string());
                self.notify_info(format!("Task #{id}{percentage}: {detail}"));
            }
            TaskUpdate::Completed { result, .. } => match result {
                AppTaskResult::AiProposal(prepared) => {
                    let summary =
                        format_ai_proposal_summary(&prepared.proposal, &prepared.touched_cells);
                    self.pending_ai_proposal = Some(prepared);
                    self.notify_success(format!("Task #{id} completed: {summary}"));
                }
            },
            TaskUpdate::Failed { diagnostics, .. } => {
                self.notify_error(format_task_failure(id, &name, &diagnostics));
            }
            TaskUpdate::Cancelled { .. } => {
                self.notify_warning(format!("Task #{id} cancelled: {name}"));
            }
        }
    }

    pub(super) fn handle_task_command(&mut self, command: TaskCommand) {
        match command {
            TaskCommand::List => self.show_tasks(),
            TaskCommand::Cancel(raw_id) => self.cancel_task(TaskId::from_raw(raw_id)),
        }
    }

    pub(super) fn active_task_status(&self) -> Option<String> {
        if self.task_runtime.is_idle() {
            return None;
        }
        self.task_runtime
            .tasks()
            .rev()
            .find(|task| !task.status.is_terminal())
            .map(format_task_snapshot)
    }

    fn show_tasks(&mut self) {
        let summary = self
            .task_runtime
            .tasks()
            .rev()
            .take(4)
            .map(format_task_snapshot)
            .collect::<Vec<_>>();
        if summary.is_empty() {
            self.notify_info("No tasks");
        } else {
            self.notify_info(summary.join(" | "));
        }
    }

    fn cancel_task(&mut self, id: TaskId) {
        if self.task_runtime.cancel(id) {
            self.notify_info(format!("Task #{id} cancellation requested"));
            return;
        }
        match self.task_runtime.task(id) {
            Some(task) => self.notify_info(format!("Task #{id} already {}", task.status)),
            None => self.notify_warning(format!("Task #{id} not found")),
        }
    }

    fn task_name(&self, id: TaskId) -> String {
        self.task_runtime
            .task(id)
            .map_or_else(|| "unknown task".to_string(), |task| task.name.clone())
    }

    #[cfg(test)]
    pub(super) fn wait_for_tasks(&mut self) {
        for _ in 0..1_000 {
            self.drain_task_updates();
            if self.task_runtime.is_idle() {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("tasks did not finish");
    }
}

fn format_task_snapshot(task: &TaskSnapshot) -> String {
    let progress = task
        .progress
        .as_ref()
        .and_then(TaskProgress::percentage)
        .map_or_else(String::new, |value| format!(" {value}%"));
    format!("#{} {} {}{progress}", task.id, task.name, task.status)
}

fn format_task_failure(id: TaskId, name: &str, diagnostics: &[TaskDiagnostic]) -> String {
    let details = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    format!("Task #{id} failed ({name}): {details}")
}

#[cfg(test)]
mod tests {
    use crate::task_runtime::{FakeTaskBackend, TaskRuntime, TaskStatus};

    use super::*;

    #[test]
    fn task_updates_flow_through_app_dispatcher_and_unified_status() {
        let (backend, controller) = FakeTaskBackend::new();
        let mut app = App {
            task_runtime: TaskRuntime::with_backend(backend),
            ..App::default()
        };
        let id = app.task_runtime.submit(
            "index samples",
            Box::new(|_| panic!("fake backend must not run jobs")),
        );
        controller.push(TaskUpdate::Started { id });
        controller.push(TaskUpdate::Progress {
            id,
            progress: TaskProgress::new(3, Some(4), "indexing"),
        });

        app.drain_task_updates();

        assert_eq!(
            app.task_runtime.task(id).expect("task").status,
            TaskStatus::Running
        );
        assert_eq!(
            app.active_task_status(),
            Some(format!("#{id} index samples running 75%"))
        );
        assert!(app
            .tui_midi_status()
            .ends_with(&format!("Task #{id} index samples running 75%")));
        assert_eq!(
            app.notification
                .as_ref()
                .map(|notification| notification.message.clone()),
            Some(format!("Task #{id} 75%: indexing"))
        );

        assert!(app.task_runtime.cancel(id));
        controller.push(TaskUpdate::Cancelled { id });
        app.drain_task_updates();
        app.show_tasks();
        assert!(app
            .notification
            .as_ref()
            .expect("task list notification")
            .message
            .contains("cancelled"));
    }

    #[test]
    fn ai_proposal_runs_as_background_task() {
        let mut app = App::default();

        app.create_ai_proposal("four on the floor".to_string());
        assert!(app.pending_ai_proposal.is_none());
        app.wait_for_tasks();

        assert!(app.pending_ai_proposal.is_some());
        assert!(app.task_runtime.is_idle());
    }
}
