use std::{env, path::Path};
#[cfg(test)]
use std::{thread, time::Duration};

use salieri_ai::{
    preview_proposal, AiPatternRequest, AiProposalProvider, LocalDeterministicProvider,
    MockProvider,
};

use crate::AiMessageRole;
use crate::{
    app_effect::AppEffect,
    app_event::{AiIntent, AppEvent, AppIntent, AppTaskResult, PreparedAiProposal, RuntimeEvent},
    command::TaskCommand,
    config::{AiConfig, AiProviderKind},
    format_ai_proposal_summary,
    task_runtime::{TaskDiagnostic, TaskFailure, TaskId, TaskProgress, TaskSnapshot, TaskUpdate},
    App, DEFAULT_NOTE_VELOCITY,
};

impl App {
    pub(super) fn create_ai_proposal(&mut self, prompt: String) {
        self.push_ai_message(AiMessageRole::User, prompt.clone());
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
            provider: self.ai_config.clone(),
        })
    }

    pub(crate) fn submit_ai_proposal(
        &mut self,
        song: salieri_core::Song,
        request: AiPatternRequest,
        provider: AiConfig,
    ) {
        let diagnostics = ai_provider_diagnostics(&provider);
        if !diagnostics.is_empty() {
            let message = format!(
                "AI provider {} unavailable: {}",
                provider.provider,
                diagnostics.join("; ")
            );
            self.push_ai_message(AiMessageRole::Error, message.clone());
            self.notify_error(message);
            return;
        }
        let provider_label = ai_provider_label(&provider);
        let job_provider_label = provider_label.clone();
        let id = self.task_runtime.submit(
            format!("AI proposal via {provider_label}"),
            Box::new(move |context| {
                context.report_progress(
                    TaskProgress::new(0, Some(3), "generating proposal")
                        .with_phase("generate")
                        .with_tool(job_provider_label),
                );
                let proposal = propose_with_configured_provider(&provider, &song, &request)
                    .map_err(|error| TaskFailure::error(format!("AI proposal failed: {error}")))?;
                context.check_cancelled()?;
                context.report_progress(
                    TaskProgress::new(1, Some(3), "validating preview")
                        .with_phase("preview")
                        .with_tool("proposal diff"),
                );
                let touched_cells = preview_proposal(&song, &proposal)
                    .map_err(|error| TaskFailure::error(format!("AI preview failed: {error}")))?
                    .touched_cells;
                context.check_cancelled()?;
                context.report_progress(
                    TaskProgress::new(3, Some(3), "proposal ready")
                        .with_phase("ready")
                        .with_tool("proposal diff"),
                );
                Ok(AppTaskResult::AiProposal(PreparedAiProposal {
                    proposal,
                    touched_cells,
                }))
            }),
        );
        self.push_ai_message(
            AiMessageRole::Progress,
            format!("Task #{id} queued via {provider_label}"),
        );
        self.notify_info(format!(
            "Task #{id} queued: AI proposal via {provider_label}"
        ));
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
            TaskUpdate::Started { .. } => {
                self.push_ai_message(
                    AiMessageRole::Progress,
                    format!("Task #{id} running: {name}"),
                );
                self.notify_info(format!("Task #{id} running: {name}"));
            }
            TaskUpdate::Progress { progress, .. } => {
                let summary = format_task_progress(id, &progress);
                self.push_ai_message(AiMessageRole::Progress, summary.clone());
                self.notify_info(summary);
            }
            TaskUpdate::Completed { result, .. } => match result {
                AppTaskResult::AiProposal(prepared) => {
                    let summary =
                        format_ai_proposal_summary(&prepared.proposal, &prepared.touched_cells);
                    self.pending_ai_proposal = Some(prepared);
                    self.push_ai_message(AiMessageRole::Assistant, summary.clone());
                    self.notify_success(format!("Task #{id} completed: {summary}"));
                }
            },
            TaskUpdate::Failed { diagnostics, .. } => {
                let failure = format_task_failure(id, &name, &diagnostics);
                self.push_ai_message(AiMessageRole::Error, failure.clone());
                self.notify_error(failure);
            }
            TaskUpdate::Cancelled { .. } => {
                self.push_ai_message(
                    AiMessageRole::Progress,
                    format!("Task #{id} cancelled: {name}"),
                );
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
            let name = self.task_name(id);
            self.push_ai_message(
                AiMessageRole::Progress,
                format!("Task #{id} cancelling: {name}"),
            );
            self.notify_info(format!("Task #{id} cancellation requested"));
            return;
        }
        match self.task_runtime.task(id) {
            Some(task) => self.notify_info(format!("Task #{id} already {}", task.status)),
            None => self.notify_warning(format!("Task #{id} not found")),
        }
    }

    pub(super) fn cancel_active_task(&mut self) {
        let Some(id) = self
            .task_runtime
            .tasks()
            .rev()
            .find(|task| !task.status.is_terminal())
            .map(|task| task.id)
        else {
            self.notify_warning("No active task");
            return;
        };
        self.cancel_task(id);
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

pub(crate) fn format_ai_provider_status(config: &AiConfig) -> String {
    let label = ai_provider_label(config);
    let diagnostics = ai_provider_diagnostics(config);
    if diagnostics.is_empty() {
        format!("AI provider {label} available")
    } else {
        format!(
            "AI provider {label} unavailable: {}",
            diagnostics.join("; ")
        )
    }
}

fn ai_provider_label(config: &AiConfig) -> String {
    format!("{} model={}", config.provider, config.model)
}

fn propose_with_configured_provider(
    config: &AiConfig,
    song: &salieri_core::Song,
    request: &AiPatternRequest,
) -> Result<salieri_ai::AiProposal, salieri_ai::AiError> {
    match config.provider {
        AiProviderKind::LocalDeterministic => LocalDeterministicProvider.propose(song, request),
        AiProviderKind::Mock => MockProvider::new(config.model.clone()).propose(song, request),
        AiProviderKind::Command => Err(salieri_ai::AiError::ProviderUnavailable(
            "command provider adapters are not implemented yet".to_string(),
        )),
    }
}

fn ai_provider_diagnostics(config: &AiConfig) -> Vec<String> {
    let mut diagnostics = Vec::new();
    for required_env in &config.required_env {
        if env::var_os(required_env)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            diagnostics.push(format!(
                "missing required environment variable {required_env}"
            ));
        }
    }
    if config.provider == AiProviderKind::Command {
        match config.command_path.as_deref().map(str::trim) {
            Some(command) if command_is_available(command) => {}
            Some(command) if !command.is_empty() => {
                diagnostics.push(format!("command binary not found: {command}"));
            }
            _ => diagnostics.push("ai.command_path is required".to_string()),
        }
        diagnostics
            .push("command provider adapters are reserved for future integrations".to_string());
    }
    diagnostics
}

fn command_is_available(command: &str) -> bool {
    let path = Path::new(command);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file();
    }
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|entry| entry.join(command).is_file()))
}

fn format_task_snapshot(task: &TaskSnapshot) -> String {
    let progress = task
        .progress
        .as_ref()
        .and_then(TaskProgress::percentage)
        .map_or_else(String::new, |value| format!(" {value}%"));
    format!("#{} {} {}{progress}", task.id, task.name, task.status)
}

fn format_task_progress(id: TaskId, progress: &TaskProgress) -> String {
    let percentage = progress
        .percentage()
        .map_or_else(String::new, |value| format!(" {value}%"));
    let phase = progress
        .phase
        .as_deref()
        .map_or_else(String::new, |value| format!(" [{value}]"));
    let tool = progress
        .tool
        .as_deref()
        .map_or_else(String::new, |value| format!(" via {value}"));
    let detail = progress.message.as_deref().unwrap_or("working");
    format!("Task #{id}{percentage}{phase}{tool}: {detail}")
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

    #[test]
    fn ai_task_progress_streams_into_chat_thread() {
        let (backend, controller) = FakeTaskBackend::new();
        let mut app = App {
            task_runtime: TaskRuntime::with_backend(backend),
            ..App::default()
        };

        app.create_ai_proposal("syncopated bass".to_string());
        let id = app
            .task_runtime
            .tasks()
            .next_back()
            .expect("queued task")
            .id;
        controller.push(TaskUpdate::Started { id });
        controller.push(TaskUpdate::Progress {
            id,
            progress: TaskProgress::new(1, Some(4), "streaming stderr diagnostics")
                .with_phase("diagnostics")
                .with_tool("mock provider"),
        });

        app.drain_task_updates();

        assert!(app.ai_thread.messages.iter().any(|message| {
            message.role == AiMessageRole::Progress
                && message.text.starts_with(&format!(
                    "Task #{id} running: AI proposal via local_deterministic"
                ))
        }));
        assert!(app.ai_thread.messages.iter().any(|message| {
            message.role == AiMessageRole::Progress
                && message.text
                    == format!(
                        "Task #{id} 25% [diagnostics] via mock provider: streaming stderr diagnostics"
                    )
        }));
        assert_eq!(
            app.active_task_status(),
            Some(format!(
                "#{id} AI proposal via local_deterministic model=local-deterministic running 25%"
            ))
        );
    }

    #[test]
    fn cancelling_ai_task_leaves_project_unchanged_and_no_pending_proposal() {
        let (backend, controller) = FakeTaskBackend::new();
        let mut app = App {
            task_runtime: TaskRuntime::with_backend(backend),
            ..App::default()
        };
        let before = app.song.clone();

        app.create_ai_proposal("cancel this sketch".to_string());
        let id = app
            .task_runtime
            .tasks()
            .next_back()
            .expect("queued task")
            .id;
        app.cancel_active_task();
        assert!(controller.was_cancelled(id));
        controller.push(TaskUpdate::Cancelled { id });
        app.drain_task_updates();

        assert_eq!(app.song, before);
        assert!(app.pending_ai_proposal.is_none());
        assert!(app.ai_thread.messages.iter().any(|message| {
            message.role == AiMessageRole::Progress
                && message.text.starts_with(&format!(
                    "Task #{id} cancelling: AI proposal via local_deterministic"
                ))
        }));
        assert!(app.ai_thread.messages.iter().any(|message| {
            message.role == AiMessageRole::Progress
                && message.text.starts_with(&format!(
                    "Task #{id} cancelled: AI proposal via local_deterministic"
                ))
        }));
    }
}
