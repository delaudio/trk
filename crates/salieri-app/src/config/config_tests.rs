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
