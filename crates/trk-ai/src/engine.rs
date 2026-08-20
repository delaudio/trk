use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
};

const DISCOVERY_ENV_KEYS: [&str; 3] = ["OPENAI_API_KEY", "TRK_AI_PROVIDER", "TRK_AI_MODEL"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineId {
    LocalDeterministic,
    Claude,
    Codex,
    OpenAi,
    Ollama,
}

impl EngineId {
    pub const ALL: [Self; 5] = [
        Self::LocalDeterministic,
        Self::Claude,
        Self::Codex,
        Self::OpenAi,
        Self::Ollama,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::LocalDeterministic => "local_deterministic",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenAi => "openai",
            Self::Ollama => "ollama",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::LocalDeterministic => "Built-in",
            Self::Claude => "Claude CLI",
            Self::Codex => "Codex CLI",
            Self::OpenAi => "OpenAI API",
            Self::Ollama => "Ollama",
        }
    }

    pub const fn default_model(self) -> &'static str {
        match self {
            Self::LocalDeterministic => "local-deterministic",
            Self::Claude => "default",
            Self::Codex => "default",
            Self::OpenAi => "gpt-5-mini",
            Self::Ollama => "llama3.2",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" | "local_deterministic" | "built-in" | "builtin" => {
                Some(Self::LocalDeterministic)
            }
            "claude" | "claude-cli" => Some(Self::Claude),
            "codex" | "codex-cli" => Some(Self::Codex),
            "openai" | "openai-api" => Some(Self::OpenAi),
            "ollama" => Some(Self::Ollama),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalResponseFormat {
    DirectProposal,
    OpenAiChatCompletions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineDescriptor {
    pub id: EngineId,
    pub label: String,
    pub model: String,
    pub command: Option<PathBuf>,
    pub arguments: Vec<String>,
    pub required_env: Vec<String>,
    pub response_format: ExternalResponseFormat,
    /// Dotenv source resolved during discovery. This carries only the path,
    /// never secret values, so execution can use the same source safely.
    pub environment_file: Option<PathBuf>,
    pub(crate) unavailable_reason: Option<String>,
}

impl EngineDescriptor {
    pub fn external(
        id: EngineId,
        label: impl Into<String>,
        model: impl Into<String>,
        command: PathBuf,
        arguments: Vec<String>,
        required_env: Vec<String>,
        response_format: ExternalResponseFormat,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            model: model.into(),
            command: Some(command),
            arguments,
            required_env,
            response_format,
            environment_file: Some(PathBuf::from(".env")),
            unavailable_reason: None,
        }
    }

    pub fn with_environment_file(mut self, environment_file: Option<PathBuf>) -> Self {
        self.environment_file = environment_file;
        self
    }

    pub fn is_available(&self) -> bool {
        self.unavailable_reason.is_none()
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineDiscoveryInput {
    pub path: Option<OsString>,
    pub environment: HashMap<String, OsString>,
    pub environment_file: Option<PathBuf>,
}

impl EngineDiscoveryInput {
    pub fn from_process() -> Self {
        Self::from_process_in(Path::new("."))
    }

    pub fn from_process_in(directory: &Path) -> Self {
        let dotenv = read_dotenv(directory.join(".env"));
        let environment = DISCOVERY_ENV_KEYS
            .into_iter()
            .filter_map(|key| {
                env::var_os(key)
                    .or_else(|| dotenv.get(key).cloned())
                    .map(|value| (key.to_string(), value))
            })
            .collect();
        Self {
            path: env::var_os("PATH"),
            environment,
            environment_file: Some(directory.join(".env")),
        }
    }

    pub fn value(&self, key: &str) -> Option<&OsStr> {
        self.environment.get(key).map(OsString::as_os_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSelectionState {
    engines: Vec<EngineDescriptor>,
    selected: usize,
    active: EngineId,
    preferred: Option<EngineId>,
}

impl EngineSelectionState {
    pub fn discover() -> Self {
        discover_engines()
    }

    pub fn engines(&self) -> &[EngineDescriptor] {
        &self.engines
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected(&self) -> &EngineDescriptor {
        &self.engines[self.selected]
    }

    pub fn active_id(&self) -> EngineId {
        self.active
    }

    pub fn active(&self) -> &EngineDescriptor {
        self.engines
            .iter()
            .find(|engine| engine.id == self.active)
            .unwrap_or(&self.engines[0])
    }

    pub fn preferred(&self) -> Option<EngineId> {
        self.preferred
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % self.engines.len();
    }

    pub fn select_previous(&mut self) {
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.engines.len() - 1);
    }

    pub fn select(&mut self, id: EngineId) -> bool {
        let Some(index) = self.engines.iter().position(|engine| engine.id == id) else {
            return false;
        };
        self.selected = index;
        true
    }

    pub fn set_active(&mut self, id: EngineId) -> Result<(), EngineSelectionError> {
        self.select(id);
        self.activate_selected().map(|_| ())
    }

    pub fn set_configured_active(&mut self, id: EngineId, model: &str) -> bool {
        if !self.select(id) {
            return false;
        }
        self.engines[self.selected].model = model.to_string();
        self.active = id;
        true
    }

    pub fn activate_selected(&mut self) -> Result<EngineDescriptor, EngineSelectionError> {
        let engine = self.selected().clone();
        if let Some(reason) = engine.unavailable_reason() {
            return Err(EngineSelectionError {
                engine: engine.id,
                reason: reason.to_string(),
            });
        }
        self.active = engine.id;
        Ok(engine)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{engine:?} is unavailable: {reason}")]
pub struct EngineSelectionError {
    pub engine: EngineId,
    pub reason: String,
}

pub fn discover_engines() -> EngineSelectionState {
    discover_engines_with(&EngineDiscoveryInput::from_process())
}

pub fn environment_value(key: &str) -> Option<OsString> {
    environment_value_from(key, Some(Path::new(".env")))
}

pub fn environment_value_from(key: &str, environment_file: Option<&Path>) -> Option<OsString> {
    env::var_os(key)
        .or_else(|| environment_file.and_then(|path| read_dotenv(path.to_path_buf()).remove(key)))
}

pub fn discover_engines_with(input: &EngineDiscoveryInput) -> EngineSelectionState {
    let preferred = input
        .value("TRK_AI_PROVIDER")
        .and_then(OsStr::to_str)
        .and_then(EngineId::parse);
    let model_override = input
        .value("TRK_AI_MODEL")
        .and_then(OsStr::to_str)
        .filter(|value| !value.trim().is_empty());
    let mut engines = EngineId::ALL
        .into_iter()
        .map(|id| discover_engine(id, input))
        .collect::<Vec<_>>();
    if let (Some(preferred), Some(model)) = (preferred, model_override) {
        if let Some(engine) = engines.iter_mut().find(|engine| engine.id == preferred) {
            engine.model = model.to_string();
            if engine.id == EngineId::Ollama {
                engine.arguments = vec!["run".to_string(), model.to_string()];
            }
        }
    }
    let selected = preferred
        .and_then(|id| engines.iter().position(|engine| engine.id == id))
        .unwrap_or(0);
    let active = preferred
        .filter(|id| {
            engines
                .iter()
                .find(|engine| engine.id == *id)
                .is_some_and(EngineDescriptor::is_available)
        })
        .unwrap_or(EngineId::LocalDeterministic);
    EngineSelectionState {
        engines,
        selected,
        active,
        preferred,
    }
}

fn discover_engine(id: EngineId, input: &EngineDiscoveryInput) -> EngineDescriptor {
    let (binary, arguments, required_env, response_format) = match id {
        EngineId::LocalDeterministic => (
            None,
            Vec::new(),
            Vec::new(),
            ExternalResponseFormat::DirectProposal,
        ),
        EngineId::Claude => (
            Some("claude"),
            vec![
                "--print".to_string(),
                "--output-format".to_string(),
                "text".to_string(),
            ],
            Vec::new(),
            ExternalResponseFormat::DirectProposal,
        ),
        EngineId::Codex => (
            Some("codex"),
            vec![
                "exec".to_string(),
                "--skip-git-repo-check".to_string(),
                "-".to_string(),
            ],
            Vec::new(),
            ExternalResponseFormat::DirectProposal,
        ),
        EngineId::OpenAi => (
            Some("curl"),
            vec![
                "--silent".to_string(),
                "--show-error".to_string(),
                "--fail-with-body".to_string(),
                "--variable".to_string(),
                "%OPENAI_API_KEY".to_string(),
                "--expand-header".to_string(),
                "Authorization: Bearer {{OPENAI_API_KEY}}".to_string(),
                "--json".to_string(),
                "@-".to_string(),
                "https://api.openai.com/v1/chat/completions".to_string(),
            ],
            vec!["OPENAI_API_KEY".to_string()],
            ExternalResponseFormat::OpenAiChatCompletions,
        ),
        EngineId::Ollama => (
            Some("ollama"),
            vec!["run".to_string(), id.default_model().to_string()],
            Vec::new(),
            ExternalResponseFormat::DirectProposal,
        ),
    };
    let command = binary.and_then(|binary| find_in_path(binary, input.path.as_deref()));
    let mut missing = Vec::new();
    if let Some(binary) = binary {
        if command.is_none() {
            missing.push(format!("missing {binary} executable in PATH"));
        }
    }
    for key in &required_env {
        if input.value(key).is_none_or(OsStr::is_empty) {
            missing.push(format!("missing {key}"));
        }
    }
    EngineDescriptor {
        id,
        label: id.label().to_string(),
        model: id.default_model().to_string(),
        command,
        arguments,
        required_env,
        response_format,
        environment_file: input.environment_file.clone(),
        unavailable_reason: (!missing.is_empty()).then(|| missing.join("; ")),
    }
}

fn find_in_path(binary: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    env::split_paths(path?).find_map(|directory| {
        binary_candidates(binary).into_iter().find_map(|candidate| {
            let candidate = directory.join(candidate);
            is_executable_file(&candidate).then_some(candidate)
        })
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn binary_candidates(binary: &str) -> Vec<OsString> {
    #[cfg(windows)]
    {
        vec![
            OsString::from(binary),
            OsString::from(format!("{binary}.exe")),
        ]
    }
    #[cfg(not(windows))]
    {
        vec![OsString::from(binary)]
    }
}

fn read_dotenv(path: PathBuf) -> HashMap<String, OsString> {
    let Ok(contents) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if !DISCOVERY_ENV_KEYS.contains(&key) {
                return None;
            }
            let value = parse_dotenv_value(value.trim())?;
            Some((key.to_string(), OsString::from(value)))
        })
        .collect()
}

fn parse_dotenv_value(value: &str) -> Option<String> {
    let Some(first) = value.chars().next() else {
        return Some(String::new());
    };
    if matches!(first, '\'' | '"') {
        let mut escaped = false;
        let closing = value.char_indices().skip(1).find_map(|(index, ch)| {
            if first == '"' && !escaped && ch == '\\' {
                escaped = true;
                return None;
            }
            if escaped {
                escaped = false;
                return None;
            }
            (ch == first).then_some(index)
        })?;
        let trailing = value[closing + first.len_utf8()..].trim();
        if !trailing.is_empty() && !trailing.starts_with('#') {
            return None;
        }
        let inner = &value[first.len_utf8()..closing];
        if first == '\'' {
            return Some(inner.to_string());
        }
        let mut parsed = String::new();
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                parsed.push(ch);
                continue;
            }
            match chars.next()? {
                'n' => parsed.push('\n'),
                'r' => parsed.push('\r'),
                't' => parsed.push('\t'),
                '\\' => parsed.push('\\'),
                '"' => parsed.push('"'),
                other => {
                    parsed.push('\\');
                    parsed.push(other);
                }
            }
        }
        return Some(parsed);
    }
    let comment = value
        .char_indices()
        .find(|(index, ch)| {
            *ch == '#'
                && value[..*index]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
        })
        .map_or(value.len(), |(index, _)| index);
    Some(value[..comment].trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn discovery_reports_present_and_missing_engines_without_secret_values() {
        let directory = test_dir("discovery");
        fs::create_dir_all(&directory).expect("create test PATH");
        for binary in ["claude", "codex", "curl"] {
            let path = directory.join(binary);
            fs::write(&path, "fixture").expect("write binary fixture");
            make_executable(&path);
        }
        let secret = "secret-must-not-appear";
        let input = EngineDiscoveryInput {
            path: Some(directory.as_os_str().to_owned()),
            environment: HashMap::from([("OPENAI_API_KEY".to_string(), OsString::from(secret))]),
            environment_file: None,
        };

        let state = discover_engines_with(&input);

        assert!(state.engines()[0].is_available());
        assert!(engine(&state, EngineId::Claude).is_available());
        assert!(engine(&state, EngineId::Codex).is_available());
        assert!(engine(&state, EngineId::OpenAi).is_available());
        assert!(engine(&state, EngineId::OpenAi)
            .arguments
            .iter()
            .any(|argument| argument == "@-"));
        assert!(engine(&state, EngineId::OpenAi)
            .arguments
            .iter()
            .any(|argument| argument.contains("{{OPENAI_API_KEY}}")));
        assert_eq!(
            engine(&state, EngineId::Ollama).unavailable_reason(),
            Some("missing ollama executable in PATH")
        );
        let diagnostics = format!("{state:?}");
        assert!(!diagnostics.contains(secret));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn preferred_engine_and_model_drive_selection_only_when_available() {
        let directory = test_dir("preferred");
        fs::create_dir_all(&directory).expect("create test PATH");
        let ollama = directory.join("ollama");
        fs::write(&ollama, "fixture").expect("write binary fixture");
        make_executable(&ollama);
        let input = EngineDiscoveryInput {
            path: Some(directory.as_os_str().to_owned()),
            environment: HashMap::from([
                ("TRK_AI_PROVIDER".to_string(), OsString::from("ollama")),
                ("TRK_AI_MODEL".to_string(), OsString::from("qwen2.5")),
            ]),
            environment_file: None,
        };

        let state = discover_engines_with(&input);

        assert_eq!(state.preferred(), Some(EngineId::Ollama));
        assert_eq!(state.active_id(), EngineId::Ollama);
        assert_eq!(state.selected().model, "qwen2.5");
        assert_eq!(state.selected().arguments, ["run", "qwen2.5"]);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn selection_wraps_and_rejects_unavailable_engine() {
        let mut state = discover_engines_with(&EngineDiscoveryInput {
            path: None,
            environment: HashMap::new(),
            environment_file: None,
        });

        state.select_previous();
        assert_eq!(state.selected().id, EngineId::Ollama);
        assert!(state.activate_selected().is_err());
        assert_eq!(state.active_id(), EngineId::LocalDeterministic);
        state.select_next();
        assert_eq!(state.selected().id, EngineId::LocalDeterministic);
        assert!(state.activate_selected().is_ok());
    }

    #[test]
    fn dotenv_is_used_without_overriding_process_style_values() {
        let directory = test_dir("dotenv");
        fs::create_dir_all(&directory).expect("create dotenv dir");
        fs::write(
            directory.join(".env"),
            "OPENAI_API_KEY='from-file'\nTRK_AI_PROVIDER=codex\nIGNORED=value\n",
        )
        .expect("write dotenv");

        let input = EngineDiscoveryInput::from_process_in(&directory);

        assert_eq!(
            input.value("TRK_AI_PROVIDER"),
            env::var_os("TRK_AI_PROVIDER")
                .as_deref()
                .or(Some(OsStr::new("codex")))
        );
        assert!(input.value("IGNORED").is_none());
        assert_eq!(input.environment_file, Some(directory.join(".env")));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn dotenv_values_remove_one_matching_quote_pair_and_decode_double_quote_escapes() {
        assert_eq!(
            parse_dotenv_value(r#""quoted \"model\"""#),
            Some("quoted \"model\"".to_string())
        );
        assert_eq!(
            parse_dotenv_value("token # local comment"),
            Some("token".to_string())
        );
        assert_eq!(
            parse_dotenv_value("'literal # value'"),
            Some("literal # value".to_string())
        );
        assert_eq!(
            parse_dotenv_value("\"quoted\" # local comment"),
            Some("quoted".to_string())
        );
        assert_eq!(parse_dotenv_value("\"unterminated"), None);
    }

    #[cfg(unix)]
    #[test]
    fn discovery_rejects_non_executable_path_entries() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = test_dir("non-executable");
        fs::create_dir_all(&directory).expect("create test PATH");
        let codex = directory.join("codex");
        fs::write(&codex, "fixture").expect("write binary fixture");
        let mut permissions = fs::metadata(&codex).expect("metadata").permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&codex, permissions).expect("set permissions");
        let state = discover_engines_with(&EngineDiscoveryInput {
            path: Some(directory.as_os_str().to_owned()),
            environment: HashMap::new(),
            environment_file: None,
        });

        assert_eq!(
            engine(&state, EngineId::Codex).unavailable_reason(),
            Some("missing codex executable in PATH")
        );

        let _ = fs::remove_dir_all(directory);
    }

    fn engine(state: &EngineSelectionState, id: EngineId) -> &EngineDescriptor {
        state
            .engines()
            .iter()
            .find(|engine| engine.id == id)
            .expect("engine")
    }

    fn test_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "trk-ai-{name}-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn make_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut permissions = fs::metadata(path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).expect("set permissions");
        }
    }
}
