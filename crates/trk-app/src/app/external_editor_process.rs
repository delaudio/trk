use std::{
    ffi::OsString,
    path::Path,
    process::{Command, ExitStatus},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExternalEditorRunError {
    Launch(String),
    Wait(String),
}

pub(crate) fn run_external_editor(path: &Path) -> Result<ExitStatus, ExternalEditorRunError> {
    let spec = resolve_editor_spec();
    let mut command =
        external_editor_command(&spec, path).map_err(ExternalEditorRunError::Launch)?;
    let mut child = command.spawn().map_err(|error| {
        ExternalEditorRunError::Launch(format!("could not launch editor: {error}"))
    })?;
    child.wait().map_err(|error| {
        ExternalEditorRunError::Wait(format!("could not wait for editor: {error}"))
    })
}

fn resolve_editor_spec() -> OsString {
    select_editor_spec(std::env::var_os("EDITOR"), std::env::var_os("VISUAL"))
}

fn select_editor_spec(editor: Option<OsString>, visual: Option<OsString>) -> OsString {
    [editor, visual]
        .into_iter()
        .flatten()
        .find(is_usable_editor_spec)
        .unwrap_or_else(|| OsString::from(default_editor()))
}

fn is_usable_editor_spec(value: &OsString) -> bool {
    value.to_str().is_some_and(|value| !value.trim().is_empty())
}

fn default_editor() -> &'static str {
    if cfg!(windows) {
        "notepad"
    } else {
        "nano"
    }
}

fn external_editor_command(spec: &OsString, path: &Path) -> Result<Command, String> {
    let spec = spec
        .to_str()
        .ok_or_else(|| "editor command is not valid Unicode".to_string())?;
    let words = parse_editor_words(spec)?;
    let mut command = Command::new(&words[0]);
    command.args(&words[1..]).arg(path);
    Ok(command)
}

fn parse_editor_words(spec: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = spec.chars().peekable();
    while let Some(character) = chars.next() {
        match (quote, character) {
            (Some(active), value) if value == active => quote = None,
            (Some(_), '\\') | (None, '\\') => {
                if let Some(next) = chars.peek().copied() {
                    if next.is_whitespace() || matches!(next, '\'' | '"') {
                        word.push(chars.next().expect("peeked character"));
                    } else {
                        word.push('\\');
                    }
                } else {
                    word.push('\\');
                }
                started = true;
            }
            (Some(_), value) => {
                word.push(value);
                started = true;
            }
            (None, '\'' | '"') => {
                quote = Some(character);
                started = true;
            }
            (None, value) if value.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (None, value) => {
                word.push(value);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err("editor command contains an unmatched quote".to_string());
    }
    if started {
        words.push(word);
    }
    if words.first().is_none_or(String::is_empty) {
        return Err("editor command is empty".to_string());
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_portable_editor_arguments_without_a_shell() {
        assert_eq!(
            parse_editor_words("code --wait --name 'trk project'").expect("parse"),
            ["code", "--wait", "--name", "trk project"]
        );
        assert_eq!(
            parse_editor_words(r#""C:\Program Files\Editor.exe" --wait"#).expect("parse"),
            [r"C:\Program Files\Editor.exe", "--wait"]
        );
        assert_eq!(
            parse_editor_words(r#""\\server\share\editor.exe" --wait"#).expect("parse UNC"),
            [r"\\server\share\editor.exe", "--wait"]
        );
        assert!(parse_editor_words("code 'unfinished").is_err());
        assert!(parse_editor_words("   ").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_unicode_editor_specs_explicitly() {
        use std::os::unix::ffi::OsStringExt;

        let spec = OsString::from_vec(vec![0xff]);
        let error = external_editor_command(&spec, Path::new("project.trk"))
            .expect_err("non-Unicode editor command");

        assert!(error.contains("not valid Unicode"));
        assert_eq!(
            select_editor_spec(Some(spec), Some("code --wait".into())),
            OsString::from("code --wait")
        );
    }

    #[test]
    fn appends_project_path_as_a_distinct_process_argument() {
        let path = Path::new("project with $(metacharacters).trk");
        let command =
            external_editor_command(&OsString::from("code --wait"), path).expect("editor command");
        assert_eq!(command.get_program(), "code");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [std::ffi::OsStr::new("--wait"), path.as_os_str()]
        );
    }

    #[test]
    fn editor_selection_prefers_editor_then_visual_then_platform_default() {
        assert_eq!(
            select_editor_spec(Some("nvim".into()), Some("code".into())),
            OsString::from("nvim")
        );
        assert_eq!(
            select_editor_spec(Some("".into()), Some("code --wait".into())),
            OsString::from("code --wait")
        );
        assert_eq!(
            select_editor_spec(Some("   ".into()), Some("code --wait".into())),
            OsString::from("code --wait")
        );
        assert_eq!(
            select_editor_spec(None, None),
            OsString::from(default_editor())
        );
    }
}
