use super::*;
use crate::ai_session as ai_session_store;

impl App {
    pub(crate) fn push_ai_message(&mut self, role: AiMessageRole, text: impl Into<String>) {
        let now = ai_session_store::now_unix_seconds();
        self.ai_thread.updated_at = now;
        self.ai_thread
            .messages
            .push(AiMessage::new(role, text.into(), now));
        ai_session_store::trim_ai_thread(&mut self.ai_thread, self.ai_retention_messages);
        self.save_ai_session_silent();
    }

    pub(crate) fn load_ai_session_command(&mut self) {
        let Some(path) = self.ai_session_file.clone() else {
            self.notify_warning("AI session persistence not configured");
            return;
        };
        if !path.exists() {
            self.notify_warning("AI session file does not exist");
            return;
        }
        match self.load_ai_session() {
            Ok(true) => self.notify_success("AI session loaded"),
            Ok(false) => self.notify_warning("AI session persistence not configured"),
            Err(error) => self.notify_error(format!("AI session load failed: {error}")),
        }
    }

    pub(crate) fn save_ai_session_command(&mut self) {
        match self.save_ai_session() {
            Ok(true) => self.notify_success("AI session saved"),
            Ok(false) => self.notify_warning("AI session persistence not configured"),
            Err(error) => self.notify_error(format!("AI session save failed: {error}")),
        }
    }

    pub(crate) fn delete_ai_session_command(&mut self) {
        let Some(path) = self.ai_session_file.clone() else {
            self.notify_warning("AI session persistence not configured");
            return;
        };
        match ai_session_store::delete_ai_thread(&path) {
            Ok(deleted) => {
                self.pending_ai_proposal = None;
                self.ai_thread =
                    ai_session_store::default_thread_for_project(self.project_path.as_deref());
                if deleted {
                    self.notify_success("AI session deleted");
                } else {
                    self.notify_info("AI session file did not exist");
                }
            }
            Err(error) => self.notify_error(format!("AI session delete failed: {error}")),
        }
    }

    pub(crate) fn set_ai_retention_messages(&mut self, retention_messages: usize) {
        self.ai_retention_messages = retention_messages.max(1);
        ai_session_store::trim_ai_thread(&mut self.ai_thread, self.ai_retention_messages);
        self.save_ai_session_silent();
        self.notify_info(format!(
            "AI session retention set to {} message(s)",
            self.ai_retention_messages
        ));
    }

    pub(crate) fn load_ai_session(&mut self) -> anyhow::Result<bool> {
        let Some(path) = &self.ai_session_file else {
            return Ok(false);
        };
        if !path.exists() {
            return Ok(false);
        }
        let mut thread = ai_session_store::load_ai_thread(path)?;
        thread.linked_project = self.project_path.clone().or(thread.linked_project);
        ai_session_store::trim_ai_thread(&mut thread, self.ai_retention_messages);
        self.ai_thread = thread;
        self.pending_ai_proposal = None;
        Ok(true)
    }

    fn save_ai_session(&mut self) -> anyhow::Result<bool> {
        let Some(path) = &self.ai_session_file else {
            return Ok(false);
        };
        self.ai_thread.linked_project = self.project_path.clone();
        self.ai_thread.updated_at = ai_session_store::now_unix_seconds();
        ai_session_store::trim_ai_thread(&mut self.ai_thread, self.ai_retention_messages);
        ai_session_store::save_ai_thread(path, &self.ai_thread)?;
        Ok(true)
    }

    fn save_ai_session_silent(&mut self) {
        if let Err(error) = self.save_ai_session() {
            self.notify_warning(format!("AI session autosave failed: {error}"));
        }
    }
}

impl AiMessage {
    pub(crate) fn new(role: AiMessageRole, text: String, created_at: u64) -> Self {
        Self {
            role,
            text,
            created_at,
            status: "complete".to_string(),
        }
    }
}
