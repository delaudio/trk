use std::{
    env,
    path::{Path, PathBuf},
};

use super::AppConfig;

pub(super) fn expand_config_paths(config: &mut AppConfig, config_path: Option<&Path>) {
    let base_dir = config_path.and_then(Path::parent);
    expand_optional_path(&mut config.ai.session_file, base_dir);
    expand_paths(&mut config.ai.guidance_dirs, base_dir);
    expand_optional_path(&mut config.midi.log_file, base_dir);
    expand_optional_path(&mut config.sample_browser.start_dir, base_dir);
    expand_optional_path(&mut config.project_browser.start_dir, base_dir);
    expand_optional_path(&mut config.project_browser.recent_file, base_dir);
    expand_optional_path(&mut config.workspace.project_library, base_dir);
    expand_optional_path(&mut config.workspace.sample_library, base_dir);
}

fn expand_paths(paths: &mut [PathBuf], base_dir: Option<&Path>) {
    for path in paths {
        *path = expand_path(path, base_dir);
    }
}

fn expand_optional_path(path: &mut Option<PathBuf>, base_dir: Option<&Path>) {
    if let Some(value) = path {
        *value = expand_path(value, base_dir);
    }
}

fn expand_path(path: &Path, base_dir: Option<&Path>) -> PathBuf {
    let raw = expand_home(path);
    if raw.trim().is_empty() {
        return PathBuf::from(raw);
    }
    let expanded = expand_environment_variables(&raw);
    let expanded = PathBuf::from(expanded);
    if expanded.is_relative() {
        base_dir.map_or(expanded.clone(), |base| base.join(expanded))
    } else {
        expanded
    }
}

fn expand_home(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if raw == "~" {
        home_dir_string().unwrap_or_else(|| raw.into_owned())
    } else if let Some(rest) = raw.strip_prefix("~/") {
        home_dir_string()
            .map(|home| format!("{home}/{rest}"))
            .unwrap_or_else(|| raw.into_owned())
    } else {
        raw.into_owned()
    }
}

fn home_dir_string() -> Option<String> {
    env::var_os("HOME").map(|home| home.to_string_lossy().into_owned())
}

fn expand_environment_variables(path: &str) -> String {
    let mut output = String::new();
    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        if matches!(chars.peek(), Some('{')) {
            let _ = chars.next();
            let mut name = String::new();
            for candidate in chars.by_ref() {
                if candidate == '}' {
                    break;
                }
                name.push(candidate);
            }
            output.push_str(&env::var(&name).unwrap_or_default());
            continue;
        }

        let mut name = String::new();
        while let Some(candidate) = chars.peek().copied() {
            if candidate == '_' || candidate.is_ascii_alphanumeric() {
                name.push(candidate);
                let _ = chars.next();
            } else {
                break;
            }
        }

        if name.is_empty() {
            output.push('$');
        } else {
            output.push_str(&env::var(&name).unwrap_or_default());
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_relative_workspace_paths_against_config_file_dir() {
        let mut config = AppConfig::default();
        let config_path = PathBuf::from("/tmp/trk/config.toml");
        config.ai.guidance_dirs = vec![PathBuf::from("Guidance")];
        config.workspace.project_library = Some(PathBuf::from("Projects"));
        config.workspace.sample_library = Some(PathBuf::from("./Samples"));

        expand_config_paths(&mut config, Some(&config_path));

        assert_eq!(
            config.ai.guidance_dirs,
            vec![PathBuf::from("/tmp/trk/Guidance")]
        );
        assert_eq!(
            config.workspace.project_library,
            Some(PathBuf::from("/tmp/trk/Projects"))
        );
        assert_eq!(
            config.workspace.sample_library,
            Some(PathBuf::from("/tmp/trk/./Samples"))
        );
    }

    #[test]
    fn expands_home_and_environment_variables_in_configured_paths() {
        let Some(home) = env::var_os("HOME") else {
            return;
        };
        let mut config = AppConfig::default();
        config.workspace.sample_library = Some(PathBuf::from("~/Samples"));
        config.project_browser.recent_file = Some(PathBuf::from("$HOME/.config/trk/recent.json"));

        expand_config_paths(&mut config, None);

        assert_eq!(
            config.workspace.sample_library,
            Some(PathBuf::from(home.clone()).join("Samples"))
        );
        assert_eq!(
            config.project_browser.recent_file,
            Some(PathBuf::from(home).join(".config/trk/recent.json"))
        );
    }
}
