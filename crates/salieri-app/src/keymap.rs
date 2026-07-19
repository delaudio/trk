use std::collections::BTreeMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::command::SalieriCommand;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeymapConfig {
    pub profile: String,
    /// Legacy normal-mode overrides retained for config compatibility.
    pub bindings: BTreeMap<String, String>,
    pub normal: BTreeMap<String, String>,
    pub edit: BTreeMap<String, String>,
    pub command: BTreeMap<String, String>,
    pub help: BTreeMap<String, String>,
    pub dialog: BTreeMap<String, String>,
    pub midi_settings: BTreeMap<String, String>,
    pub sequence: BTreeMap<String, String>,
    pub tracks: BTreeMap<String, String>,
    pub patterns: BTreeMap<String, String>,
    pub sampler: BTreeMap<String, String>,
    pub sample_browser: BTreeMap<String, String>,
    pub project_browser: BTreeMap<String, String>,
    pub ai: BTreeMap<String, String>,
    pub clip: BTreeMap<String, String>,
}

impl Default for KeymapConfig {
    fn default() -> Self {
        Self {
            profile: "tracker".to_string(),
            bindings: BTreeMap::new(),
            normal: BTreeMap::new(),
            edit: BTreeMap::new(),
            command: BTreeMap::new(),
            help: BTreeMap::new(),
            dialog: BTreeMap::new(),
            midi_settings: BTreeMap::new(),
            sequence: BTreeMap::new(),
            tracks: BTreeMap::new(),
            patterns: BTreeMap::new(),
            sampler: BTreeMap::new(),
            sample_browser: BTreeMap::new(),
            project_browser: BTreeMap::new(),
            ai: BTreeMap::new(),
            clip: BTreeMap::new(),
        }
    }
}

impl KeymapConfig {
    pub fn layers(&self) -> [(&'static str, &'static str, &BTreeMap<String, String>); 15] {
        [
            ("normal", "keymap.bindings", &self.bindings),
            ("normal", "keymap.normal", &self.normal),
            ("edit", "keymap.edit", &self.edit),
            ("command", "keymap.command", &self.command),
            ("help", "keymap.help", &self.help),
            ("dialog", "keymap.dialog", &self.dialog),
            ("midi_settings", "keymap.midi_settings", &self.midi_settings),
            ("sequence", "keymap.sequence", &self.sequence),
            ("tracks", "keymap.tracks", &self.tracks),
            ("patterns", "keymap.patterns", &self.patterns),
            ("sampler", "keymap.sampler", &self.sampler),
            (
                "sample_browser",
                "keymap.sample_browser",
                &self.sample_browser,
            ),
            (
                "project_browser",
                "keymap.project_browser",
                &self.project_browser,
            ),
            ("ai", "keymap.ai", &self.ai),
            ("clip", "keymap.clip", &self.clip),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapMode {
    Normal,
    Edit,
    Command,
    Help,
    Dialog,
    MidiSettings,
    Sequence,
    Tracks,
    Patterns,
    Sampler,
    SampleBrowser,
    ProjectBrowser,
    Ai,
    Clip,
}

impl KeymapMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "normal" => Some(Self::Normal),
            "edit" => Some(Self::Edit),
            "command" => Some(Self::Command),
            "help" => Some(Self::Help),
            "dialog" => Some(Self::Dialog),
            "midi_settings" => Some(Self::MidiSettings),
            "sequence" => Some(Self::Sequence),
            "tracks" => Some(Self::Tracks),
            "patterns" => Some(Self::Patterns),
            "sampler" => Some(Self::Sampler),
            "sample_browser" => Some(Self::SampleBrowser),
            "project_browser" => Some(Self::ProjectBrowser),
            "ai" => Some(Self::Ai),
            "clip" => Some(Self::Clip),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyChord {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyChord {
    fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim();
        if value.is_empty() {
            return Err("key must not be empty".to_string());
        }
        let parts = value.split('+').map(str::trim).collect::<Vec<_>>();
        if parts.iter().any(|part| part.is_empty()) {
            return Err("use the name 'plus' for the + key".to_string());
        }
        let (code_name, modifier_names) = parts.split_last().expect("non-empty key parts");
        let mut modifiers = KeyModifiers::NONE;
        for modifier in modifier_names {
            let parsed = match modifier.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => KeyModifiers::CONTROL,
                "alt" | "option" => KeyModifiers::ALT,
                "shift" => KeyModifiers::SHIFT,
                "super" | "cmd" | "command" => KeyModifiers::SUPER,
                _ => return Err(format!("unknown modifier '{modifier}'")),
            };
            if modifiers.contains(parsed) {
                return Err(format!("duplicate modifier '{modifier}'"));
            }
            modifiers.insert(parsed);
        }

        let mut code = parse_key_code(code_name)?;
        if let KeyCode::Char(value) = code {
            if value.is_ascii_uppercase() {
                modifiers.insert(KeyModifiers::SHIFT);
                code = KeyCode::Char(value.to_ascii_lowercase());
            }
        }
        if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
            code = KeyCode::BackTab;
            modifiers.remove(KeyModifiers::SHIFT);
        }
        Ok(Self { code, modifiers })
    }

    fn from_event(event: &KeyEvent) -> Self {
        let mut code = event.code;
        let mut modifiers = event.modifiers;
        if let KeyCode::Char(value) = code {
            if value.is_ascii_uppercase() {
                modifiers.insert(KeyModifiers::SHIFT);
                code = KeyCode::Char(value.to_ascii_lowercase());
            }
        }
        if code == KeyCode::BackTab {
            modifiers.remove(KeyModifiers::SHIFT);
        }
        Self { code, modifiers }
    }
}

fn parse_key_code(value: &str) -> Result<KeyCode, String> {
    let normalized = value.to_ascii_lowercase();
    let named = match normalized.as_str() {
        "space" => Some(KeyCode::Char(' ')),
        "plus" => Some(KeyCode::Char('+')),
        "esc" | "escape" => Some(KeyCode::Esc),
        "enter" | "return" => Some(KeyCode::Enter),
        "backspace" => Some(KeyCode::Backspace),
        "delete" | "del" => Some(KeyCode::Delete),
        "insert" | "ins" => Some(KeyCode::Insert),
        "tab" => Some(KeyCode::Tab),
        "backtab" => Some(KeyCode::BackTab),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "pageup" | "page_up" => Some(KeyCode::PageUp),
        "pagedown" | "page_down" => Some(KeyCode::PageDown),
        _ => None,
    };
    if let Some(code) = named {
        return Ok(code);
    }
    if let Some(number) = normalized.strip_prefix('f') {
        if let Ok(number) = number.parse::<u8>() {
            if (1..=24).contains(&number) {
                return Ok(KeyCode::F(number));
            }
        }
    }
    let mut characters = value.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) => Ok(KeyCode::Char(character)),
        _ => Err(format!("unknown key '{value}'")),
    }
}

#[derive(Debug, Clone)]
struct Binding {
    mode: KeymapMode,
    chord: KeyChord,
    command: SalieriCommand,
    field: String,
    command_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapDiagnostic {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct Keymap {
    bindings: Vec<Binding>,
}

impl Keymap {
    pub fn from_config(config: &KeymapConfig) -> Result<Self, Vec<KeymapDiagnostic>> {
        let mut bindings: Vec<Binding> = Vec::new();
        let mut diagnostics = Vec::new();
        for (mode_name, field_prefix, configured) in config.layers() {
            let mode = KeymapMode::parse(mode_name).expect("known keymap config layer");
            for (key, configured_command) in configured {
                let field = format!("{field_prefix}.{key}");
                let chord = match KeyChord::parse(key) {
                    Ok(chord) => chord,
                    Err(message) => {
                        diagnostics.push(KeymapDiagnostic { field, message });
                        continue;
                    }
                };
                if let Some(existing) = bindings
                    .iter()
                    .find(|binding| binding.mode == mode && binding.chord == chord)
                {
                    diagnostics.push(KeymapDiagnostic {
                        field,
                        message: format!("conflicts with {}", existing.field),
                    });
                    continue;
                }
                let command = match parse_command(configured_command) {
                    Ok(command) => command,
                    Err(message) => {
                        diagnostics.push(KeymapDiagnostic { field, message });
                        continue;
                    }
                };
                let command_text = command_text(configured_command);
                bindings.push(Binding {
                    mode,
                    chord,
                    command,
                    field,
                    command_text,
                });
            }
        }
        if diagnostics.is_empty() {
            Ok(Self { bindings })
        } else {
            Err(diagnostics)
        }
    }

    pub fn command_for(&self, mode: KeymapMode, event: &KeyEvent) -> Option<SalieriCommand> {
        let chord = KeyChord::from_event(event);
        self.bindings
            .iter()
            .find(|binding| binding.mode == mode && binding.chord == chord)
            .map(|binding| binding.command.clone())
    }

    pub fn help_summary(&self) -> Option<String> {
        if self.bindings.is_empty() {
            return None;
        }
        let visible = self
            .bindings
            .iter()
            .take(3)
            .map(|binding| {
                let field = binding
                    .field
                    .strip_prefix("keymap.")
                    .unwrap_or(&binding.field);
                format!("{field} -> :{}", binding.command_text)
            })
            .collect::<Vec<_>>()
            .join(" | ");
        let remaining = self.bindings.len().saturating_sub(3);
        let suffix = if remaining == 0 {
            String::new()
        } else {
            format!(" | +{remaining} more")
        };
        Some(format!("Custom keys: {visible}{suffix}"))
    }
}

pub fn validate_config(config: &KeymapConfig) -> Vec<KeymapDiagnostic> {
    Keymap::from_config(config).err().unwrap_or_default()
}

fn parse_command(value: &str) -> Result<SalieriCommand, String> {
    let value = command_text(value);
    match SalieriCommand::parse(&value) {
        Ok(Some(command)) => {
            command.validate().map_err(|error| error.to_string())?;
            Ok(command)
        }
        Ok(None) => Err("command must not be empty".to_string()),
        Err(error) => Err(error.to_string()),
    }
}

fn command_text(value: &str) -> String {
    value
        .trim()
        .strip_prefix(':')
        .unwrap_or(value.trim())
        .trim()
        .to_string()
}

#[cfg(test)]
#[path = "keymap_tests.rs"]
mod tests;
