use std::{
    borrow::Cow,
    ffi::OsString,
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::json;
use trk_core::Song;

use crate::{
    environment_value_from, AiEdit, AiError, AiPatternRequest, AiProposal, AiSource,
    EngineDescriptor, ExternalResponseFormat,
};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_STDOUT_BYTES: u64 = 1_048_576;
const MAX_STDERR_BYTES: u64 = 16_384;

#[derive(Debug, Clone)]
pub struct ExternalEngineProvider {
    engine: EngineDescriptor,
    timeout: Duration,
}

impl ExternalEngineProvider {
    pub fn new(engine: EngineDescriptor, timeout: Duration) -> Self {
        Self { engine, timeout }
    }

    pub fn propose_with_cancel(
        &self,
        song: &Song,
        request: &AiPatternRequest,
        is_cancelled: impl Fn() -> bool,
    ) -> Result<AiProposal, AiError> {
        if request.prompt.trim().is_empty() {
            return Err(AiError::EmptyPrompt);
        }
        let command = self.engine.command.as_ref().ok_or_else(|| {
            AiError::ProviderUnavailable(
                self.engine
                    .unavailable_reason()
                    .unwrap_or("external command is unavailable")
                    .to_string(),
            )
        })?;
        let prompt = composition_prompt(song, request)?;
        let child_environment = resolve_child_environment(&self.engine)?;
        let stdin = match self.engine.response_format {
            ExternalResponseFormat::DirectProposal => prompt.into_bytes(),
            ExternalResponseFormat::OpenAiChatCompletions => {
                openai_request_body(&self.engine.model, &prompt)?.into_bytes()
            }
        };
        let stdout = run_bounded_process(
            command,
            &self.engine.arguments,
            &stdin,
            &child_environment,
            self.timeout,
            is_cancelled,
        )?;
        parse_external_proposal(
            &stdout,
            self.engine.response_format,
            self.engine.label.as_str(),
            request.prompt.as_str(),
        )
    }
}

fn resolve_child_environment(
    engine: &EngineDescriptor,
) -> Result<Vec<(String, OsString)>, AiError> {
    engine
        .required_env
        .iter()
        .map(|key| {
            environment_value_from(key, engine.environment_file.as_deref())
                .filter(|value| !value.is_empty())
                .map(|value| (key.clone(), value))
                .ok_or_else(|| AiError::ProviderUnavailable(format!("missing {key}")))
        })
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExternalProposalResponse {
    summary: String,
    edits: Vec<ExternalEdit>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum ExternalEdit {
    SetNote {
        pattern: usize,
        row: usize,
        track: usize,
        pitch: u8,
        velocity: u8,
    },
    ClearCell {
        pattern: usize,
        row: usize,
        track: usize,
    },
}

// The service-owned envelope is intentionally forward-compatible because the
// API includes metadata fields. The model-authored proposal inside `content`
// remains strict through `ExternalProposalResponse` and `ExternalEdit`.
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: String,
}

pub fn parse_external_proposal(
    stdout: &[u8],
    response_format: ExternalResponseFormat,
    provider: &str,
    prompt: &str,
) -> Result<AiProposal, AiError> {
    let raw = std::str::from_utf8(stdout)
        .map_err(|error| AiError::ProviderResponse(format!("stdout is not UTF-8: {error}")))?;
    let proposal_json = match response_format {
        ExternalResponseFormat::DirectProposal => Cow::Borrowed(raw),
        ExternalResponseFormat::OpenAiChatCompletions => {
            let response: OpenAiResponse = serde_json::from_str(raw).map_err(|error| {
                AiError::ProviderResponse(format!("invalid OpenAI response envelope: {error}"))
            })?;
            Cow::Owned(
                response
                    .choices
                    .first()
                    .map(|choice| choice.message.content.clone())
                    .ok_or_else(|| {
                        AiError::ProviderResponse("OpenAI response contains no choices".to_string())
                    })?,
            )
        }
    };
    let response: ExternalProposalResponse = serde_json::from_str(&proposal_json)
        .map_err(|error| AiError::ProviderResponse(format!("invalid proposal JSON: {error}")))?;
    if response.summary.trim().is_empty() {
        return Err(AiError::ProviderResponse(
            "proposal summary cannot be empty".to_string(),
        ));
    }
    if response.edits.is_empty() {
        return Err(AiError::EmptyProposal);
    }
    let edits = response
        .edits
        .into_iter()
        .map(|edit| match edit {
            ExternalEdit::SetNote {
                pattern,
                row,
                track,
                pitch,
                velocity,
            } => AiEdit::SetNote {
                pattern,
                row,
                track,
                pitch,
                velocity,
            },
            ExternalEdit::ClearCell {
                pattern,
                row,
                track,
            } => AiEdit::ClearCell {
                pattern,
                row,
                track,
            },
        })
        .collect();
    Ok(AiProposal {
        source: AiSource::External {
            provider: provider.to_string(),
        },
        prompt: prompt.to_string(),
        summary: response.summary,
        edits,
    })
}

fn composition_prompt(song: &Song, request: &AiPatternRequest) -> Result<String, AiError> {
    let envelope = json!({
        "instruction": "Return only one JSON object matching response_schema. Do not use markdown fences.",
        "response_schema": {
            "summary": "non-empty string",
            "edits": [
                {"op": "set_note", "pattern": 0, "row": 0, "track": 0, "pitch": 60, "velocity": 100},
                {"op": "clear_cell", "pattern": 0, "row": 1, "track": 0}
            ]
        },
        "selection": request,
        "song": song,
    });
    serde_json::to_string_pretty(&envelope)
        .map_err(|error| AiError::ProviderIo(format!("serialize composition request: {error}")))
}

fn openai_request_body(model: &str, prompt: &str) -> Result<String, AiError> {
    serde_json::to_string(&json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "response_format": {"type": "json_object"}
    }))
    .map_err(|error| AiError::ProviderIo(format!("serialize OpenAI request: {error}")))
}

fn run_bounded_process(
    command: &std::path::Path,
    arguments: &[String],
    stdin: &[u8],
    child_environment: &[(String, OsString)],
    timeout: Duration,
    is_cancelled: impl Fn() -> bool,
) -> Result<Vec<u8>, AiError> {
    let mut process = Command::new(command);
    process
        .args(arguments)
        .envs(child_environment.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut process);
    let mut child = process
        .spawn()
        .map_err(|error| AiError::ProviderLaunch(error.to_string()))?;
    let mut child_stdin = child
        .stdin
        .take()
        .ok_or_else(|| AiError::ProviderIo("child stdin unavailable".to_string()))?;
    let stdin = stdin.to_vec();
    let mut stdin_writer = Some(thread::spawn(move || {
        child_stdin.write_all(&stdin)?;
        drop(child_stdin);
        Ok::<(), std::io::Error>(())
    }));

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AiError::ProviderIo("child stdout unavailable".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AiError::ProviderIo("child stderr unavailable".to_string()))?;
    let stdout_reader = thread::spawn(move || read_bounded(stdout, MAX_STDOUT_BYTES));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, MAX_STDERR_BYTES));
    let started = Instant::now();

    let status = loop {
        if is_cancelled() {
            terminate_child_tree(&mut child);
            drop(stdin_writer.take());
            drop(stdout_reader);
            drop(stderr_reader);
            return Err(AiError::ProviderCancelled);
        }
        if started.elapsed() >= timeout {
            terminate_child_tree(&mut child);
            drop(stdin_writer.take());
            drop(stdout_reader);
            drop(stderr_reader);
            return Err(AiError::ProviderTimeout(
                u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
            ));
        }
        match child
            .try_wait()
            .map_err(|error| AiError::ProviderIo(format!("wait for child: {error}")))?
        {
            Some(status) => break status,
            None => thread::sleep(POLL_INTERVAL),
        }
    };
    stdin_writer
        .take()
        .expect("stdin writer present")
        .join()
        .map_err(|_| AiError::ProviderIo("stdin writer panicked".to_string()))?
        .map_err(|error| AiError::ProviderIo(format!("write child stdin: {error}")))?;
    let stdout = join_reader(stdout_reader, "stdout")?;
    let stderr = join_reader(stderr_reader, "stderr")?;
    if stdout.truncated {
        return Err(AiError::ProviderResponse(format!(
            "stdout exceeded the {MAX_STDOUT_BYTES}-byte limit"
        )));
    }
    if !status.success() {
        let stderr_text = String::from_utf8_lossy(&stderr.bytes);
        let truncation = if stderr.truncated { " (truncated)" } else { "" };
        return Err(AiError::ProviderExit(format!(
            "status {status}; stderr{truncation}: {}",
            stderr_text.trim()
        )));
    }
    Ok(stdout.bytes)
}

struct CapturedStream {
    bytes: Vec<u8>,
    truncated: bool,
}

fn read_bounded(mut reader: impl Read, limit: u64) -> std::io::Result<CapturedStream> {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut buffer = [0_u8; 8_192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        let captured = remaining.min(count);
        bytes.extend_from_slice(&buffer[..captured]);
        truncated |= captured < count;
    }
    Ok(CapturedStream { bytes, truncated })
}

fn join_reader(
    reader: thread::JoinHandle<std::io::Result<CapturedStream>>,
    stream: &str,
) -> Result<CapturedStream, AiError> {
    reader
        .join()
        .map_err(|_| AiError::ProviderIo(format!("{stream} reader panicked")))?
        .map_err(|error| AiError::ProviderIo(format!("read child {stream}: {error}")))
}

#[cfg(unix)]
fn configure_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_tree(_command: &mut Command) {}

fn terminate_child_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let process_group = i32::try_from(child.id()).unwrap_or(i32::MAX);
        // SAFETY: the child was placed in a new process group whose id equals
        // its pid; a negative pid targets exactly that group.
        unsafe {
            libc::kill(-process_group, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EngineId, ExternalResponseFormat};
    use std::{
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    #[test]
    fn parses_direct_and_openai_wrapped_proposals() {
        let proposal = br#"{"summary":"Bass fill","edits":[{"op":"set_note","pattern":0,"row":4,"track":1,"pitch":48,"velocity":96},{"op":"clear_cell","pattern":0,"row":5,"track":1}]}"#;
        let direct = parse_external_proposal(
            proposal,
            ExternalResponseFormat::DirectProposal,
            "Codex CLI",
            "bass fill",
        )
        .expect("direct proposal");
        let wrapped = serde_json::to_vec(&json!({
            "id": "response-metadata-is-forward-compatible",
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "role": "assistant",
                    "content": std::str::from_utf8(proposal).expect("utf8")
                }
            }],
            "usage": {"total_tokens": 42}
        }))
        .expect("OpenAI fixture");
        let openai = parse_external_proposal(
            &wrapped,
            ExternalResponseFormat::OpenAiChatCompletions,
            "OpenAI API",
            "bass fill",
        )
        .expect("OpenAI proposal");

        assert_eq!(direct.edits, openai.edits);
        assert_eq!(direct.summary, "Bass fill");
        assert_eq!(
            direct.source,
            AiSource::External {
                provider: "Codex CLI".to_string()
            }
        );
    }

    #[test]
    fn openai_body_preserves_prompt_escapes_as_json_data() {
        let prompt = "tabs\tquotes\"literal-backslash\\t";
        let body = openai_request_body("fixture-model", prompt).expect("request body");
        let value: serde_json::Value = serde_json::from_str(&body).expect("request JSON");

        assert_eq!(value["messages"][0]["content"], prompt);
        assert_eq!(value["model"], "fixture-model");
    }

    #[cfg(unix)]
    #[test]
    fn external_process_receives_stdin_and_returns_a_proposal() {
        let environment_file =
            std::env::temp_dir().join(format!("trk-ai-external-env-{}", std::process::id()));
        std::fs::write(&environment_file, "TRK_TEST_AI_TOKEN=from-dotenv\n")
            .expect("write dotenv fixture");
        let engine = EngineDescriptor::external(
            EngineId::Codex,
            "fixture",
            "fixture",
            PathBuf::from("/bin/sh"),
            vec![
                "-c".to_string(),
                "test \"$TRK_TEST_AI_TOKEN\" = from-dotenv || exit 17; cat >/dev/null; printf '%s' '{\"summary\":\"Generated\",\"edits\":[{\"op\":\"set_note\",\"pattern\":0,\"row\":0,\"track\":0,\"pitch\":60,\"velocity\":100}]}'".to_string(),
            ],
            vec!["TRK_TEST_AI_TOKEN".to_string()],
            ExternalResponseFormat::DirectProposal,
        )
        .with_environment_file(Some(environment_file.clone()));
        let proposal = ExternalEngineProvider::new(engine, Duration::from_secs(1))
            .propose_with_cancel(&Song::empty(), &fixture_request(), || false)
            .expect("external proposal");

        assert_eq!(proposal.summary, "Generated");
        assert_eq!(proposal.edits.len(), 1);
        let _ = std::fs::remove_file(environment_file);
    }

    #[test]
    fn parser_rejects_unknown_fields_and_empty_edits() {
        let unknown = br#"{"summary":"bad","edits":[],"extra":true}"#;
        assert!(matches!(
            parse_external_proposal(
                unknown,
                ExternalResponseFormat::DirectProposal,
                "fixture",
                "prompt"
            ),
            Err(AiError::ProviderResponse(_))
        ));
        let empty = br#"{"summary":"bad","edits":[]}"#;
        assert!(matches!(
            parse_external_proposal(
                empty,
                ExternalResponseFormat::DirectProposal,
                "fixture",
                "prompt"
            ),
            Err(AiError::EmptyProposal)
        ));
    }

    #[test]
    fn bounded_reader_drains_and_marks_oversized_streams() {
        let captured = read_bounded(std::io::Cursor::new(b"abcdefgh"), 4).expect("capture");

        assert_eq!(captured.bytes, b"abcd");
        assert!(captured.truncated);
    }

    #[cfg(unix)]
    #[test]
    fn external_process_timeout_and_cancellation_are_bounded() {
        let provider = fixture_provider(Duration::from_millis(30));
        let request = fixture_request();
        let started = Instant::now();
        let error = provider
            .propose_with_cancel(&Song::empty(), &request, || false)
            .expect_err("timeout");
        assert!(matches!(error, AiError::ProviderTimeout(_)));
        assert!(started.elapsed() < Duration::from_secs(2));

        let provider = fixture_provider(Duration::from_secs(5));
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = Arc::clone(&cancelled);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            cancellation.store(true, Ordering::Release);
        });
        let error = provider
            .propose_with_cancel(&Song::empty(), &request, || {
                cancelled.load(Ordering::Acquire)
            })
            .expect_err("cancelled");
        assert!(matches!(error, AiError::ProviderCancelled));
    }

    #[cfg(unix)]
    fn fixture_provider(timeout: Duration) -> ExternalEngineProvider {
        ExternalEngineProvider::new(
            EngineDescriptor {
                id: EngineId::Codex,
                label: "fixture".to_string(),
                model: "fixture".to_string(),
                command: Some(PathBuf::from("/bin/sh")),
                arguments: vec!["-c".to_string(), "sleep 5 & wait".to_string()],
                required_env: Vec::new(),
                response_format: ExternalResponseFormat::DirectProposal,
                environment_file: None,
                unavailable_reason: None,
            },
            timeout,
        )
    }

    #[cfg(unix)]
    fn fixture_request() -> AiPatternRequest {
        AiPatternRequest {
            prompt: "fixture".to_string(),
            pattern: 0,
            track: 0,
            rows: 4,
            root_pitch: 60,
            velocity: 100,
        }
    }
}
