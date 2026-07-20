use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use super::*;

const SUPPORTED_GUIDANCE_EXTENSIONS: &[&str] = &["md", "txt", "json"];

impl App {
    pub(crate) fn handle_ai_guidance_command(&mut self, values: &[&str]) {
        match values {
            [] | ["status"] => self.show_ai_guidance_status(),
            ["list"] => self.list_ai_guidance_command(),
            ["show" | "inspect", selector @ ..] => self.show_ai_guidance_command(selector),
            ["apply" | "use", selector @ ..] => self.apply_ai_guidance_command(selector),
            ["clear"] => self.clear_ai_guidance_command(),
            _ => self.notify_warning(
                "Usage: :ai guidance list | show FILE | apply FILE | clear | status",
            ),
        }
    }

    pub(crate) fn ai_prompt_with_guidance(&self, prompt: &str) -> String {
        let Some(guidance) = &self.ai_guidance else {
            return prompt.to_string();
        };
        format!(
            "Local guidance: {}\nSource: {}\n---\n{}\n---\nUser prompt:\n{}",
            guidance.label,
            guidance.path.display(),
            guidance.content,
            prompt
        )
    }

    fn show_ai_guidance_status(&mut self) {
        match &self.ai_guidance {
            Some(guidance) => self.notify_info(format!(
                "AI guidance active: {} ({})",
                guidance.label,
                guidance.path.display()
            )),
            None => self.notify_info("No AI guidance active"),
        }
    }

    fn list_ai_guidance_command(&mut self) {
        match list_guidance_files(&self.ai_config.guidance_dirs) {
            Ok(files) if files.is_empty() => {
                self.notify_warning("No AI guidance files found");
            }
            Ok(files) => {
                let summary = files
                    .iter()
                    .map(|path| format!("- {}", path.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.push_ai_message(
                    AiMessageRole::Assistant,
                    format!("AI guidance files:\n{summary}"),
                );
                self.notify_info(format!("Found {} AI guidance file(s)", files.len()));
            }
            Err(error) => self.report_ai_guidance_error(error),
        }
    }

    fn show_ai_guidance_command(&mut self, selector: &[&str]) {
        match self.load_ai_guidance(selector) {
            Ok(guidance) => {
                self.push_ai_message(
                    AiMessageRole::Assistant,
                    format!(
                        "AI guidance: {}\nSource: {}\n---\n{}",
                        guidance.label,
                        guidance.path.display(),
                        guidance.content
                    ),
                );
                self.notify_info(format!("AI guidance loaded: {}", guidance.label));
            }
            Err(error) => self.report_ai_guidance_error(error),
        }
    }

    fn apply_ai_guidance_command(&mut self, selector: &[&str]) {
        match self.load_ai_guidance(selector) {
            Ok(guidance) => {
                let label = guidance.label.clone();
                self.ai_guidance = Some(guidance);
                self.push_ai_message(
                    AiMessageRole::Progress,
                    format!("AI guidance applied: {label}"),
                );
                self.notify_success(format!("AI guidance applied: {label}"));
            }
            Err(error) => self.report_ai_guidance_error(error),
        }
    }

    fn clear_ai_guidance_command(&mut self) {
        if let Some(guidance) = self.ai_guidance.take() {
            self.push_ai_message(
                AiMessageRole::Progress,
                format!("AI guidance cleared: {}", guidance.label),
            );
            self.notify_info("AI guidance cleared");
        } else {
            self.notify_info("No AI guidance active");
        }
    }

    fn load_ai_guidance(&self, selector: &[&str]) -> Result<AiGuidanceContext, String> {
        let selector = selector.join(" ");
        let selector = selector.trim();
        if selector.is_empty() {
            return Err("AI guidance file is required".to_string());
        }
        let path = resolve_guidance_path(selector, &self.ai_config.guidance_dirs)?;
        read_guidance_file(&path)
    }

    fn report_ai_guidance_error(&mut self, error: String) {
        let message = format!("AI guidance error: {error}");
        self.push_ai_message(AiMessageRole::Error, message.clone());
        self.notify_warning(message);
    }
}

fn resolve_guidance_path(selector: &str, dirs: &[PathBuf]) -> Result<PathBuf, String> {
    let direct = PathBuf::from(selector);
    if direct.exists() {
        return Ok(direct);
    }
    if selector.contains(std::path::MAIN_SEPARATOR) {
        return Err(format!("file not found: {selector}"));
    }
    if dirs.is_empty() {
        return Err("no [ai].guidance_dirs configured".to_string());
    }

    let matches = list_guidance_files(dirs)?
        .into_iter()
        .filter(|path| guidance_selector_matches(selector, path))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!("file not found in [ai].guidance_dirs: {selector}")),
        many => Err(format!(
            "selector {selector:?} is ambiguous: {}",
            many.iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn list_guidance_files(dirs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    if dirs.is_empty() {
        return Err("no [ai].guidance_dirs configured".to_string());
    }
    let mut files = Vec::new();
    for dir in dirs {
        collect_guidance_files(dir, &mut files)
            .map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    }
    files.sort();
    Ok(files)
}

fn collect_guidance_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "directory not found",
        ));
    }
    if path.is_file() {
        if is_supported_guidance_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_guidance_files(&path, files)?;
        } else if is_supported_guidance_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn read_guidance_file(path: &Path) -> Result<AiGuidanceContext, String> {
    if !is_supported_guidance_file(path) {
        return Err(format!(
            "unsupported file type for {}; use .md, .txt, or .json",
            path.display()
        ));
    }
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let content = if extension_is(path, "json") {
        let value = serde_json::from_str::<serde_json::Value>(&raw)
            .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))?;
        serde_json::to_string_pretty(&value)
            .map_err(|error| format!("cannot render JSON in {}: {error}", path.display()))?
    } else {
        raw
    };
    Ok(AiGuidanceContext {
        label: path
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("guidance")
            .to_string(),
        path: path.to_path_buf(),
        content,
    })
}

fn guidance_selector_matches(selector: &str, path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name == selector)
        || path
            .file_stem()
            .and_then(OsStr::to_str)
            .is_some_and(|stem| stem == selector)
}

fn is_supported_guidance_file(path: &Path) -> bool {
    SUPPORTED_GUIDANCE_EXTENSIONS
        .iter()
        .any(|extension| extension_is(path, extension))
}

fn extension_is(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}
