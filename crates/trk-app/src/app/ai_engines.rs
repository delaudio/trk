use super::*;
use crate::config::{AiConfig, AiProviderKind};

pub(super) fn resolve_initial_ai_engines(
    configured: &AiConfig,
) -> (EngineSelectionState, AiConfig) {
    resolve_initial_ai_engines_with(configured, EngineSelectionState::discover())
}

fn resolve_initial_ai_engines_with(
    configured: &AiConfig,
    mut engines: EngineSelectionState,
) -> (EngineSelectionState, AiConfig) {
    let mut config = configured.clone();
    if let Some(preferred) = engines.preferred() {
        if engines.active_id() == preferred {
            let descriptor = engines.active().clone();
            apply_engine_config(&mut config, &descriptor);
            return (engines, config);
        }
    }
    if let Some(configured_id) = configured_engine_id(configured.provider) {
        engines.set_configured_active(configured_id, &configured.model);
    }
    (engines, config)
}

impl App {
    pub(crate) fn open_ai_engine_selector(&mut self) {
        self.ai_engine_selector_open = true;
        let active = self.ai_engines.active_id();
        self.ai_engines.select(active);
        self.notify_info("Select an available AI engine; Enter activates, Esc cancels");
    }

    pub(crate) fn close_ai_engine_selector(&mut self) {
        self.ai_engine_selector_open = false;
    }

    pub(crate) fn select_next_ai_engine(&mut self) {
        self.ai_engines.select_next();
    }

    pub(crate) fn select_previous_ai_engine(&mut self) {
        self.ai_engines.select_previous();
    }

    pub(crate) fn activate_selected_ai_engine(&mut self) {
        match self.ai_engines.activate_selected() {
            Ok(engine) => {
                apply_engine_config(&mut self.ai_config, &engine);
                self.ai_engine_selector_open = false;
                self.push_ai_message(
                    AiMessageRole::System,
                    format!("Active AI engine: {} ({})", engine.label, engine.model),
                );
                self.notify_success(format!(
                    "AI engine {} model={} active",
                    engine.label, engine.model
                ));
            }
            Err(error) => self.notify_warning(error.to_string()),
        }
    }

    pub(crate) fn active_ai_engine_label(&self) -> String {
        let engine = self.ai_engines.active();
        if configured_engine_id(self.ai_config.provider) == Some(engine.id) {
            format!("{} model={}", engine.label, self.ai_config.model)
        } else {
            format!("{} model={}", self.ai_config.provider, self.ai_config.model)
        }
    }
}

fn configured_engine_id(provider: AiProviderKind) -> Option<EngineId> {
    match provider {
        AiProviderKind::LocalDeterministic => Some(EngineId::LocalDeterministic),
        AiProviderKind::Claude => Some(EngineId::Claude),
        AiProviderKind::Codex => Some(EngineId::Codex),
        AiProviderKind::OpenAi => Some(EngineId::OpenAi),
        AiProviderKind::Ollama => Some(EngineId::Ollama),
        AiProviderKind::Mock | AiProviderKind::Command => None,
    }
}

fn apply_engine_config(config: &mut AiConfig, engine: &EngineDescriptor) {
    config.provider = match engine.id {
        EngineId::LocalDeterministic => AiProviderKind::LocalDeterministic,
        EngineId::Claude => AiProviderKind::Claude,
        EngineId::Codex => AiProviderKind::Codex,
        EngineId::OpenAi => AiProviderKind::OpenAi,
        EngineId::Ollama => AiProviderKind::Ollama,
    };
    config.model = engine.model.clone();
    config.command_path = engine
        .command
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    config.command_args = engine.arguments.clone();
    config.required_env = engine.required_env.clone();
    config.environment_file = engine.environment_file.clone();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, ffi::OsString, fs};
    use trk_ai::{discover_engines_with, EngineDiscoveryInput};

    #[test]
    fn applying_discovered_engine_updates_runtime_provider_without_restart() {
        let directory =
            std::env::temp_dir().join(format!("trk-app-ai-engine-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create test PATH");
        let codex = directory.join("codex");
        fs::write(&codex, "fixture").expect("write codex fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mut permissions = fs::metadata(&codex).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&codex, permissions).expect("set permissions");
        }
        let mut app = App {
            ai_engines: discover_engines_with(&EngineDiscoveryInput {
                path: Some(OsString::from(directory.as_os_str())),
                environment: HashMap::new(),
                environment_file: None,
                curl_supports_header_expansion: None,
            }),
            ..App::default()
        };
        app.ai_engines.select(EngineId::Codex);

        app.activate_selected_ai_engine();

        assert_eq!(
            configured_engine_id(app.ai_config.provider),
            Some(EngineId::Codex)
        );
        assert_eq!(app.ai_engines.active_id(), EngineId::Codex);
        assert_eq!(
            app.ai_config.command_path.as_deref(),
            Some(directory.join("codex").to_string_lossy().as_ref())
        );
        assert!(!app.ai_engine_selector_open);

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn unavailable_configured_engine_remains_the_active_unavailable_choice() {
        let configured = AiConfig {
            provider: AiProviderKind::Codex,
            model: "configured-model".to_string(),
            command_path: Some("missing-codex".to_string()),
            ..AiConfig::default()
        };
        let discovered = discover_engines_with(&EngineDiscoveryInput {
            path: None,
            environment: HashMap::new(),
            environment_file: None,
            curl_supports_header_expansion: None,
        });

        let (engines, resolved) = resolve_initial_ai_engines_with(&configured, discovered);

        assert_eq!(engines.active_id(), EngineId::Codex);
        assert!(!engines.active().is_available());
        assert_eq!(resolved.provider, AiProviderKind::Codex);
        assert_eq!(resolved.model, "configured-model");
        assert_eq!(engines.active().model, "configured-model");
    }
}
