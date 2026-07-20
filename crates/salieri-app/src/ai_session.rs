#[cfg(test)]
use std::path::PathBuf;
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{AiMessage, AiMessageRole, AiThread};

const AI_SESSION_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedAiSession {
    version: u32,
    thread: AiThread,
}

pub(crate) fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub(crate) fn load_ai_thread(path: &Path) -> Result<AiThread> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read AI session {}", path.display()))?;
    let session: PersistedAiSession = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse AI session {}", path.display()))?;
    if session.version != AI_SESSION_FORMAT_VERSION {
        anyhow::bail!("unsupported AI session version {}", session.version);
    }
    Ok(session.thread)
}

pub(crate) fn save_ai_thread(path: &Path, thread: &AiThread) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create AI session directory {}", parent.display())
        })?;
    }
    let session = PersistedAiSession {
        version: AI_SESSION_FORMAT_VERSION,
        thread: thread.clone(),
    };
    let contents =
        serde_json::to_string_pretty(&session).context("failed to serialize AI session history")?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write AI session {}", path.display()))
}

pub(crate) fn delete_ai_thread(path: &Path) -> Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            Err(error).with_context(|| format!("failed to delete AI session {}", path.display()))
        }
    }
}

pub(crate) fn default_thread_for_project(project_path: Option<&Path>) -> AiThread {
    let now = now_unix_seconds();
    AiThread {
        id: format!("local-{now}"),
        created_at: now,
        updated_at: now,
        status: "active".to_string(),
        linked_project: project_path.map(Path::to_path_buf),
        messages: vec![AiMessage {
            role: AiMessageRole::System,
            text: "Local AI chat is ready. Prompts become reviewable proposals.".to_string(),
            created_at: now,
            status: "complete".to_string(),
        }],
        composer: String::new(),
    }
}

pub(crate) fn trim_ai_thread(thread: &mut AiThread, retention_messages: usize) {
    let retention_messages = retention_messages.max(1);
    if thread.messages.len() <= retention_messages {
        return;
    }
    let system = thread
        .messages
        .iter()
        .find(|message| message.role == AiMessageRole::System)
        .cloned();
    let keep_from = thread.messages.len().saturating_sub(retention_messages);
    let mut retained = thread.messages.split_off(keep_from);
    if let Some(system) = system {
        if !retained
            .iter()
            .any(|message| message.role == AiMessageRole::System)
        {
            retained.insert(0, system);
            while retained.len() > retention_messages {
                retained.remove(1);
            }
        }
    }
    thread.messages = retained;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "salieri-ai-session-{label}-{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn ai_thread_round_trips_without_pending_proposals() {
        let path = session_path("roundtrip");
        let mut thread = default_thread_for_project(Some(Path::new("/tmp/song.salieri")));
        thread.messages.push(AiMessage {
            role: AiMessageRole::User,
            text: "make a bassline".to_string(),
            created_at: thread.created_at + 1,
            status: "complete".to_string(),
        });

        save_ai_thread(&path, &thread).expect("save session");
        let loaded = load_ai_thread(&path).expect("load session");

        assert_eq!(
            loaded.linked_project,
            Some(PathBuf::from("/tmp/song.salieri"))
        );
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[1].text, "make a bassline");

        let _ = delete_ai_thread(&path);
    }

    #[test]
    fn malformed_ai_history_is_rejected() {
        let path = session_path("malformed");
        fs::write(&path, "{not json").expect("write malformed");

        let error = load_ai_thread(&path).expect_err("malformed rejected");

        assert!(error.to_string().contains("failed to parse AI session"));
        let _ = delete_ai_thread(&path);
    }
}
