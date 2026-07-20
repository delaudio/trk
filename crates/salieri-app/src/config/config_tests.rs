use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::*;

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

struct TestFile(PathBuf);

impl TestFile {
    fn new(name: &str, contents: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "salieri-layout-config-{name}-{}-{}.toml",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, contents).expect("write config");
        Self(path)
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn loads_layout_preferences() {
    let file = TestFile::new(
        "layout",
        r#"
[ui.layout]
default = "studio"
show_inspector = true
left_width = 32
inspector_width = 44
track_desk_height = 12
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
    let layout = loaded.config().ui.layout;

    assert_eq!(layout.default, preferences::LayoutPreset::Studio);
    assert!(layout.show_inspector);
    assert_eq!(layout.left_width, 32);
    assert_eq!(layout.inspector_width, 44);
    assert_eq!(layout.track_desk_height, 12);
}

#[test]
fn loads_project_display_and_playback_preferences() {
    let file = TestFile::new(
        "project-display-playback",
        r#"
[ui]
row_number_format = "hex"
row_number_base = "one"
pattern_divider_interval = 8
pattern_highlight_interval = 32
show_pattern_top_info = false

[audio]
playback_headroom_db = 6
limiter_mode = "soft"
resampling_quality = "high"
send_mode = "post_fader"
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
    let config = loaded.config();

    assert_eq!(config.ui.row_number_format, RowNumberFormat::Hex);
    assert_eq!(config.ui.row_number_base, RowNumberBase::One);
    assert_eq!(config.ui.pattern_divider_interval, 8);
    assert_eq!(config.ui.pattern_highlight_interval, 32);
    assert!(!config.ui.show_pattern_top_info);
    assert_eq!(config.audio.playback_headroom_db, 6);
    assert_eq!(config.audio.limiter_mode, LimiterMode::Soft);
    assert_eq!(config.audio.resampling_quality, ResamplingQuality::High);
    assert_eq!(config.audio.send_mode, SendMode::PostFader);
}

#[test]
fn accepts_zero_edit_step_for_stationary_step_jump() {
    let file = TestFile::new(
        "zero-edit-step",
        r#"
[keyboard]
edit_step = 0
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");

    assert_eq!(loaded.config().keyboard.edit_step, 0);
}

#[test]
fn loads_ai_provider_preferences() {
    let file = TestFile::new(
        "ai-provider",
        r#"
[ai]
provider = "mock"
model = "fixture-mock"
command_path = "codex"
required_env = ["SALIERI_AI_TOKEN"]
session_file = "ai-session.json"
retention_messages = 42
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
    let ai = &loaded.config().ai;

    assert_eq!(ai.provider, AiProviderKind::Mock);
    assert_eq!(ai.model, "fixture-mock");
    assert_eq!(ai.command_path.as_deref(), Some("codex"));
    assert_eq!(ai.required_env, vec!["SALIERI_AI_TOKEN".to_string()]);
    assert_eq!(
        ai.session_file,
        Some(
            file.0
                .parent()
                .expect("config parent")
                .join("ai-session.json")
        )
    );
    assert_eq!(ai.retention_messages, 42);
}

#[test]
fn loads_workspace_library_paths_relative_to_config_file() {
    let file = TestFile::new(
        "workspace",
        r#"
[workspace]
project_library = "Projects"
sample_library = "./Samples"
recent_project_limit = 24
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
    let config_dir = file.0.parent().expect("config dir");

    assert_eq!(
        loaded.config().workspace.project_library,
        Some(config_dir.join("Projects"))
    );
    assert_eq!(
        loaded.config().workspace.sample_library,
        Some(config_dir.join("./Samples"))
    );
    assert_eq!(loaded.config().workspace.recent_project_limit, 24);
}

#[test]
fn preserves_legacy_browser_start_dirs_as_expanded_paths() {
    let file = TestFile::new(
        "browser-start-dirs",
        r#"
[sample_browser]
start_dir = "LegacySamples"

[project_browser]
start_dir = "LegacyProjects"
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
    let config_dir = file.0.parent().expect("config dir");

    assert_eq!(
        loaded.config().sample_browser.start_dir,
        Some(config_dir.join("LegacySamples"))
    );
    assert_eq!(
        loaded.config().project_browser.start_dir,
        Some(config_dir.join("LegacyProjects"))
    );
}

#[test]
fn loads_partial_config_over_defaults_and_exposes_metadata() {
    let file = TestFile::new(
        "partial",
        r#"
[keyboard]
vim_navigation = false
edit_step = 4
default_octave = 5

[keymap]
profile = "studio"
bindings = { "ctrl+p" = "play pattern" }

[ui]
follow_playhead = false
display_mode = "compact"

[theme]
name = "high-contrast"

[midi]
default_output = "IAC Driver"
default_input = "IAC Driver"
log_file = "salieri-midi.log"

[sample_browser]
chooser_command = "yazi"
start_dir = "~/Samples"

[project_browser]
start_dir = "~/Music/Salieri"
recent_file = "recent-projects.json"

[history]
undo_limit = 250
"#,
    );

    let loaded = load_config(Some(&file.0), ConfigOverrides::default()).expect("load config");
    let config = loaded.config();
    assert!(!config.keyboard.vim_navigation);
    assert_eq!(config.keyboard.edit_step, 4);
    assert!(!config.ui.follow_playhead);
    assert!(!config.ui.show_line_numbers_hex);
    assert_eq!(config.audio, AudioPreferences::default());
    assert_eq!(config.ai, AiConfig::default());
    assert_eq!(config.midi.default_output, "IAC Driver");
    assert_eq!(config.history.undo_limit, 250);
    assert_eq!(
        config.sample_browser.start_dir,
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Samples"))
    );
    assert_eq!(loaded.metadata().source, ConfigSource::File(file.0.clone()));
    assert_eq!(loaded.metadata().keymap_profile, "studio");
    assert_eq!(loaded.metadata().theme_name, "high-contrast");
    assert_eq!(loaded.metadata().display_mode, DisplayMode::Compact);
}

#[test]
fn validates_layout_preferences() {
    let file = TestFile::new(
        "invalid-layout",
        r#"
[ui.layout]
left_width = 4
inspector_width = 100
track_desk_height = 2
"#,
    );

    let error = load_config(Some(&file.0), ConfigOverrides::default()).expect_err("invalid");
    let ConfigLoadError::Validation(error) = error else {
        panic!("expected validation error");
    };

    let fields = error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.field.as_str())
        .collect::<Vec<_>>();
    assert!(fields.contains(&"ui.layout.left_width"));
    assert!(fields.contains(&"ui.layout.inspector_width"));
    assert!(fields.contains(&"ui.layout.track_desk_height"));
}

#[test]
fn validates_ai_provider_preferences() {
    let file = TestFile::new(
        "invalid-ai",
        r#"
[ai]
model = " "
command_path = " "
required_env = ["SALIERI_AI_TOKEN", ""]
retention_messages = 0
"#,
    );

    let error = load_config(Some(&file.0), ConfigOverrides::default()).expect_err("invalid");
    let ConfigLoadError::Validation(error) = error else {
        panic!("expected validation error");
    };
    let rendered = error.to_string();

    assert_eq!(error.diagnostics.len(), 4);
    assert!(rendered.contains("ai.model"));
    assert!(rendered.contains("ai.command_path"));
    assert!(rendered.contains("ai.required_env.1"));
    assert!(rendered.contains("ai.retention_messages"));
}

#[test]
fn validates_project_display_and_playback_preferences() {
    let file = TestFile::new(
        "invalid-project-display-playback",
        r#"
[ui]
pattern_divider_interval = 300
pattern_highlight_interval = 400

[audio]
playback_headroom_db = 80
"#,
    );

    let error = load_config(Some(&file.0), ConfigOverrides::default()).expect_err("invalid");
    let ConfigLoadError::Validation(error) = error else {
        panic!("expected validation error");
    };

    let fields = error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.field.as_str())
        .collect::<Vec<_>>();
    assert!(fields.contains(&"ui.pattern_divider_interval"));
    assert!(fields.contains(&"ui.pattern_highlight_interval"));
    assert!(fields.contains(&"audio.playback_headroom_db"));
}
