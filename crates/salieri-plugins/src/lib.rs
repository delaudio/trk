use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRoot {
    pub format: PluginFormat,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PluginInventoryOptions {
    pub prompt_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInventory {
    pub schema_version: u32,
    pub prompt_safe: bool,
    pub scanned_roots: Vec<PluginScannedRoot>,
    pub entries: Vec<PluginEntry>,
    pub failures: Vec<PluginScanFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginScannedRoot {
    pub format: PluginFormat,
    pub path_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub format: PluginFormat,
    pub kind: PluginKind,
    pub role_suitability: Vec<PluginRole>,
    pub vendor_hint: Option<String>,
    pub path_hint: String,
    pub tags: Vec<String>,
    pub metadata: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginScanFailure {
    pub format: PluginFormat,
    pub path_hint: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginFormat {
    AudioUnit,
    Vst,
    Vst3,
    Clap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginKind {
    Instrument,
    Effect,
    MidiEffect,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginRole {
    Drums,
    Bass,
    Lead,
    Harmony,
    Fx,
    Mix,
    Utility,
}

pub fn default_plugin_roots() -> Vec<PluginRoot> {
    let mut roots = Vec::new();
    add_platform_roots(&mut roots);
    roots
}

pub fn scan_default_plugin_inventory(options: PluginInventoryOptions) -> PluginInventory {
    let roots = default_plugin_roots();
    scan_plugin_inventory(&roots, options)
}

pub fn scan_plugin_inventory(
    roots: &[PluginRoot],
    options: PluginInventoryOptions,
) -> PluginInventory {
    let mut inventory = PluginInventory {
        schema_version: 1,
        prompt_safe: options.prompt_safe,
        scanned_roots: roots
            .iter()
            .map(|root| PluginScannedRoot {
                format: root.format,
                path_hint: path_hint(&root.path, &root.path, options.prompt_safe),
            })
            .collect(),
        entries: Vec::new(),
        failures: Vec::new(),
    };

    for root in roots {
        collect_plugins(root, &root.path, options, &mut inventory);
    }

    inventory.entries.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.path_hint.cmp(&right.path_hint))
    });
    for (index, entry) in inventory.entries.iter_mut().enumerate() {
        entry.id = format!(
            "plugin_{index:03}_{:08x}",
            stable_hash(&format!(
                "{}:{}:{}",
                entry.format.as_str(),
                entry.name,
                entry.path_hint
            ))
        );
    }
    inventory
}

pub fn classify_plugin_kind(name_or_path: &str) -> PluginKind {
    let tokens = tokens(name_or_path);
    if contains_any(&tokens, &["midi", "arp", "arpeggiator", "sequencer"]) {
        PluginKind::MidiEffect
    } else if contains_any(
        &tokens,
        &[
            "synth",
            "instrument",
            "sampler",
            "piano",
            "organ",
            "drum",
            "drums",
            "bass",
            "moog",
            "juno",
            "dx7",
            "303",
            "808",
            "909",
        ],
    ) {
        PluginKind::Instrument
    } else if contains_any(
        &tokens,
        &[
            "reverb",
            "delay",
            "echo",
            "compressor",
            "comp",
            "limiter",
            "eq",
            "filter",
            "chorus",
            "flanger",
            "phaser",
            "distortion",
            "saturator",
            "gate",
            "utility",
        ],
    ) {
        PluginKind::Effect
    } else {
        PluginKind::Unknown
    }
}

pub fn classify_role_suitability(name_or_path: &str, kind: PluginKind) -> Vec<PluginRole> {
    let tokens = tokens(name_or_path);
    let mut roles = Vec::new();
    if contains_any(&tokens, &["drum", "drums", "kick", "snare", "808", "909"]) {
        roles.push(PluginRole::Drums);
    }
    if contains_any(&tokens, &["bass", "sub", "303", "moog"]) {
        roles.push(PluginRole::Bass);
    }
    if contains_any(&tokens, &["lead", "mono", "acid", "303"]) {
        roles.push(PluginRole::Lead);
    }
    if contains_any(&tokens, &["pad", "poly", "juno", "piano", "organ", "chord"]) {
        roles.push(PluginRole::Harmony);
    }
    if contains_any(
        &tokens,
        &["fx", "reverb", "delay", "echo", "chorus", "phaser"],
    ) {
        roles.push(PluginRole::Fx);
    }
    if contains_any(
        &tokens,
        &["compressor", "limiter", "eq", "meter", "analyzer"],
    ) {
        roles.push(PluginRole::Mix);
    }
    if roles.is_empty() {
        roles.push(match kind {
            PluginKind::Instrument => PluginRole::Lead,
            PluginKind::Effect => PluginRole::Fx,
            PluginKind::MidiEffect | PluginKind::Unknown => PluginRole::Utility,
        });
    }
    roles
}

pub fn known_plugin_tags(name_or_path: &str) -> Vec<String> {
    let normalized = name_or_path.to_ascii_lowercase();
    let mut tags = Vec::new();
    for (needle, tag_set) in [
        ("juno", &["emulation", "analog-poly", "classic-synth"][..]),
        (
            "minimoog",
            &["emulation", "analog-mono", "classic-synth"][..],
        ),
        ("moog", &["emulation", "analog-mono"][..]),
        ("tb-303", &["emulation", "acid", "bassline"][..]),
        ("303", &["emulation", "acid", "bassline"][..]),
        ("tr-808", &["emulation", "drum-machine"][..]),
        ("808", &["emulation", "drum-machine"][..]),
        ("tr-909", &["emulation", "drum-machine"][..]),
        ("909", &["emulation", "drum-machine"][..]),
        ("dx7", &["emulation", "fm-synthesis"][..]),
        ("1176", &["emulation", "fet-compressor"][..]),
        ("la-2a", &["emulation", "opto-compressor"][..]),
        ("fairchild", &["emulation", "vari-mu-compressor"][..]),
    ] {
        if normalized.contains(needle) {
            for tag in tag_set {
                push_unique(&mut tags, (*tag).to_string());
            }
        }
    }
    tags
}

fn collect_plugins(
    root: &PluginRoot,
    dir: &Path,
    options: PluginInventoryOptions,
    inventory: &mut PluginInventory,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) => {
            inventory.failures.push(PluginScanFailure {
                format: root.format,
                path_hint: path_hint(&root.path, dir, options.prompt_safe),
                message: source.to_string(),
            });
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(source) => {
                inventory.failures.push(PluginScanFailure {
                    format: root.format,
                    path_hint: path_hint(&root.path, dir, options.prompt_safe),
                    message: source.to_string(),
                });
                continue;
            }
        };
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(source) => {
                inventory.failures.push(PluginScanFailure {
                    format: root.format,
                    path_hint: path_hint(&root.path, &path, options.prompt_safe),
                    message: source.to_string(),
                });
                continue;
            }
        };

        if is_plugin_path(&path, root.format) {
            inventory.entries.push(plugin_entry(root, &path, options));
        } else if metadata.is_dir() {
            collect_plugins(root, &path, options, inventory);
        }
    }
}

fn plugin_entry(root: &PluginRoot, path: &Path, options: PluginInventoryOptions) -> PluginEntry {
    let name = plugin_name(path);
    let format = root.format;
    let kind = classify_plugin_kind(&format!("{} {}", name, path.display()));
    let role_suitability = classify_role_suitability(&name, kind);
    let mut metadata = Vec::new();
    metadata.push(format!("format:{}", format.as_str()));
    metadata.push(format!("kind:{}", kind.as_str()));
    let tags = known_plugin_tags(&name);
    for tag in &tags {
        metadata.push(format!("tag:{tag}"));
    }
    PluginEntry {
        id: String::new(),
        name: prompt_safe(&name),
        format,
        kind,
        role_suitability,
        vendor_hint: vendor_hint(&root.path, path).map(|value| prompt_safe(&value)),
        path_hint: path_hint(&root.path, path, options.prompt_safe),
        tags,
        metadata,
    }
}

fn plugin_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .or_else(|| path.file_name().and_then(|value| value.to_str()))
        .unwrap_or("unknown")
        .to_string()
}

fn vendor_hint(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .parent()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .and_then(|value| {
            value
                .replace('\\', "/")
                .split('/')
                .next_back()
                .map(str::to_string)
        })
}

fn path_hint(root: &Path, path: &Path, prompt_safe: bool) -> String {
    if !prompt_safe {
        return path.display().to_string();
    }
    let relative = path.strip_prefix(root).unwrap_or(path);
    let value = relative
        .to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    if value.is_empty() {
        root.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("plugin-root")
            .to_string()
    } else {
        value
    }
}

fn is_plugin_path(path: &Path, format: PluginFormat) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            match format {
                PluginFormat::AudioUnit => extension == "component",
                PluginFormat::Vst => matches!(extension.as_str(), "vst" | "so" | "dll"),
                PluginFormat::Vst3 => extension == "vst3",
                PluginFormat::Clap => extension == "clap",
            }
        })
        .unwrap_or(false)
}

fn tokens(value: &str) -> Vec<String> {
    value
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn contains_any(tokens: &[String], needles: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| needles.iter().any(|needle| token == needle))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn prompt_safe(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn stable_hash(value: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

impl PluginFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::AudioUnit => "audio-unit",
            Self::Vst => "vst",
            Self::Vst3 => "vst3",
            Self::Clap => "clap",
        }
    }
}

impl PluginKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Instrument => "instrument",
            Self::Effect => "effect",
            Self::MidiEffect => "midi-effect",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(target_os = "macos")]
fn add_platform_roots(roots: &mut Vec<PluginRoot>) {
    roots.extend([
        PluginRoot {
            format: PluginFormat::AudioUnit,
            path: PathBuf::from("/Library/Audio/Plug-Ins/Components"),
        },
        PluginRoot {
            format: PluginFormat::Vst,
            path: PathBuf::from("/Library/Audio/Plug-Ins/VST"),
        },
        PluginRoot {
            format: PluginFormat::Vst3,
            path: PathBuf::from("/Library/Audio/Plug-Ins/VST3"),
        },
        PluginRoot {
            format: PluginFormat::Clap,
            path: PathBuf::from("/Library/Audio/Plug-Ins/CLAP"),
        },
    ]);
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.extend([
            PluginRoot {
                format: PluginFormat::AudioUnit,
                path: home.join("Library/Audio/Plug-Ins/Components"),
            },
            PluginRoot {
                format: PluginFormat::Vst,
                path: home.join("Library/Audio/Plug-Ins/VST"),
            },
            PluginRoot {
                format: PluginFormat::Vst3,
                path: home.join("Library/Audio/Plug-Ins/VST3"),
            },
            PluginRoot {
                format: PluginFormat::Clap,
                path: home.join("Library/Audio/Plug-Ins/CLAP"),
            },
        ]);
    }
}

#[cfg(target_os = "linux")]
fn add_platform_roots(roots: &mut Vec<PluginRoot>) {
    roots.extend([
        PluginRoot {
            format: PluginFormat::Vst,
            path: PathBuf::from("/usr/lib/vst"),
        },
        PluginRoot {
            format: PluginFormat::Vst3,
            path: PathBuf::from("/usr/lib/vst3"),
        },
        PluginRoot {
            format: PluginFormat::Clap,
            path: PathBuf::from("/usr/lib/clap"),
        },
        PluginRoot {
            format: PluginFormat::Vst,
            path: PathBuf::from("/usr/local/lib/vst"),
        },
        PluginRoot {
            format: PluginFormat::Vst3,
            path: PathBuf::from("/usr/local/lib/vst3"),
        },
        PluginRoot {
            format: PluginFormat::Clap,
            path: PathBuf::from("/usr/local/lib/clap"),
        },
    ]);
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.extend([
            PluginRoot {
                format: PluginFormat::Vst,
                path: home.join(".vst"),
            },
            PluginRoot {
                format: PluginFormat::Vst3,
                path: home.join(".vst3"),
            },
            PluginRoot {
                format: PluginFormat::Clap,
                path: home.join(".clap"),
            },
        ]);
    }
}

#[cfg(target_os = "windows")]
fn add_platform_roots(roots: &mut Vec<PluginRoot>) {
    if let Some(common) = std::env::var_os("COMMONPROGRAMFILES") {
        let common = PathBuf::from(common);
        roots.extend([
            PluginRoot {
                format: PluginFormat::Vst3,
                path: common.join("VST3"),
            },
            PluginRoot {
                format: PluginFormat::Clap,
                path: common.join("CLAP"),
            },
        ]);
    }
    if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
        roots.push(PluginRoot {
            format: PluginFormat::Vst,
            path: PathBuf::from(program_files).join("VstPlugins"),
        });
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn add_platform_roots(_roots: &mut Vec<PluginRoot>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_plugin_kind_and_roles_from_names() {
        let kind = classify_plugin_kind("TAL-Juno-LX.vst3");
        assert_eq!(kind, PluginKind::Instrument);
        let roles = classify_role_suitability("TAL-Juno-LX.vst3", kind);
        assert!(roles.contains(&PluginRole::Harmony));

        let kind = classify_plugin_kind("1176 Compressor.component");
        assert_eq!(kind, PluginKind::Effect);
        let roles = classify_role_suitability("1176 Compressor.component", kind);
        assert!(roles.contains(&PluginRole::Mix));

        assert_eq!(
            classify_plugin_kind("MIDI Arpeggiator.vst3"),
            PluginKind::MidiEffect
        );
    }

    #[test]
    fn enriches_known_historical_and_emulation_tags() {
        let tags = known_plugin_tags("Juno 106 Chorus.vst3");
        assert!(tags.contains(&"emulation".to_string()));
        assert!(tags.contains(&"analog-poly".to_string()));

        let tags = known_plugin_tags("TB-303 Bassline.clap");
        assert!(tags.contains(&"acid".to_string()));
        assert!(tags.contains(&"bassline".to_string()));
    }

    #[test]
    fn scans_inventory_and_keeps_failures_per_root() {
        let root = std::env::temp_dir().join(format!("salieri-plugins-{}", std::process::id()));
        let vendor = root.join("Acme");
        fs::create_dir_all(&vendor).expect("mkdir");
        fs::write(vendor.join("Acme Juno.vst3"), b"plugin").expect("write plugin");
        fs::write(root.join("notes.txt"), b"ignore").expect("write notes");
        let missing = root.join("missing");

        let inventory = scan_plugin_inventory(
            &[
                PluginRoot {
                    format: PluginFormat::Vst3,
                    path: root.clone(),
                },
                PluginRoot {
                    format: PluginFormat::Clap,
                    path: missing,
                },
            ],
            PluginInventoryOptions { prompt_safe: true },
        );

        let _ = fs::remove_dir_all(&root);

        assert_eq!(inventory.schema_version, 1);
        assert_eq!(inventory.entries.len(), 1);
        assert_eq!(inventory.entries[0].name, "Acme Juno");
        assert_eq!(inventory.entries[0].vendor_hint.as_deref(), Some("Acme"));
        assert_eq!(inventory.entries[0].path_hint, "Acme/Acme Juno.vst3");
        assert!(!inventory.entries[0]
            .path_hint
            .contains(std::env::temp_dir().to_string_lossy().as_ref()));
        assert_eq!(inventory.failures.len(), 1);
    }
}
