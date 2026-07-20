use std::{
    fs,
    path::{Path, PathBuf},
};

use super::*;

const PRESET_PROFILE_SCHEMA: &str = "salieri.preset-profile.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetProfile {
    schema: String,
    title: String,
    source_project: Option<PathBuf>,
    tracks: Vec<PresetTrack>,
    instruments: Vec<PresetInstrument>,
    native_devices: Vec<PresetDevice>,
    midi: PresetMidiInventory,
    ableton_bridge: PresetBridgeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetTrack {
    id: u32,
    name: String,
    midi_channel: u8,
    assigned_instrument: Option<String>,
    assigned_sample: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetInstrument {
    id: u32,
    name: String,
    primary_sample: Option<String>,
    zone_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetDevice {
    scope: String,
    id: u32,
    name: String,
    kind: String,
    bypassed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetMidiInventory {
    output_status: String,
    input_status: String,
    output_ports: Vec<String>,
    input_ports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetBridgeStatus {
    state: String,
    note: String,
}

impl App {
    pub(crate) fn handle_preset_command(&mut self, values: &[&str]) {
        match values {
            [] | ["inventory"] | ["status"] => self.show_preset_inventory(),
            ["save", path @ ..] => self.save_preset_profile_command(path),
            ["list", path @ ..] => self.list_preset_profiles_command(path),
            ["show" | "analyze", path @ ..] => self.show_preset_profile_command(path),
            ["load" | "apply" | "guide", path @ ..] => self.apply_preset_profile_command(path),
            ["ableton", command @ ..] => self.route_ableton_preset_command(command),
            _ => self.notify_warning(
                "Usage: :preset inventory | save PATH | list DIR | show PATH | load PATH | ableton status",
            ),
        }
    }

    fn show_preset_inventory(&mut self) {
        let profile = self.current_preset_profile();
        let summary = summarize_preset_profile(&profile);
        self.push_ai_message(AiMessageRole::Assistant, summary.clone());
        self.notify_info(summary);
    }

    fn save_preset_profile_command(&mut self, values: &[&str]) {
        let Ok(path) = command_path(values, "preset profile path is required") else {
            self.notify_warning("Usage: :preset save PATH");
            return;
        };
        let profile = self.current_preset_profile();
        match save_preset_profile(&path, &profile) {
            Ok(()) => self.notify_success(format!("Preset profile saved: {}", path.display())),
            Err(error) => self.notify_warning(format!("Preset save failed: {error}")),
        }
    }

    fn list_preset_profiles_command(&mut self, values: &[&str]) {
        let Ok(path) = command_path(values, "preset profile directory is required") else {
            self.notify_warning("Usage: :preset list DIR");
            return;
        };
        match list_preset_profiles(&path) {
            Ok(profiles) if profiles.is_empty() => {
                self.notify_warning("No preset profiles found");
            }
            Ok(profiles) => {
                let summary = profiles
                    .iter()
                    .map(|(path, title)| format!("- {title}: {}", path.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.push_ai_message(
                    AiMessageRole::Assistant,
                    format!("Preset profiles:\n{summary}"),
                );
                self.notify_info(format!("Found {} preset profile(s)", profiles.len()));
            }
            Err(error) => self.notify_warning(format!("Preset list failed: {error}")),
        }
    }

    fn show_preset_profile_command(&mut self, values: &[&str]) {
        let Ok(path) = command_path(values, "preset profile path is required") else {
            self.notify_warning("Usage: :preset show PATH");
            return;
        };
        match read_preset_profile(&path) {
            Ok(profile) => {
                let summary = summarize_preset_profile(&profile);
                self.push_ai_message(AiMessageRole::Assistant, summary.clone());
                self.notify_info(summary);
            }
            Err(error) => self.notify_warning(format!("Preset read failed: {error}")),
        }
    }

    fn apply_preset_profile_command(&mut self, values: &[&str]) {
        let Ok(path) = command_path(values, "preset profile path is required") else {
            self.notify_warning("Usage: :preset load PATH");
            return;
        };
        match read_preset_profile(&path) {
            Ok(profile) => {
                let content = format_preset_profile_for_guidance(&profile);
                let label = profile.title.clone();
                self.ai_guidance = Some(AiGuidanceContext {
                    label: format!("preset:{label}"),
                    path: path.clone(),
                    content,
                });
                self.push_ai_message(
                    AiMessageRole::Progress,
                    format!("Preset profile loaded as AI guidance: {label}"),
                );
                self.notify_success(format!("Preset profile loaded: {label}"));
            }
            Err(error) => self.notify_warning(format!("Preset read failed: {error}")),
        }
    }

    fn route_ableton_preset_command(&mut self, values: &[&str]) {
        let action = values.first().copied().unwrap_or("status");
        let message = format!(
            "Ableton preset {action} requires the optional Ableton bridge; this tracker build stores local preset metadata only"
        );
        self.push_ai_message(AiMessageRole::Progress, message.clone());
        self.notify_info(message);
    }

    fn current_preset_profile(&self) -> PresetProfile {
        PresetProfile {
            schema: PRESET_PROFILE_SCHEMA.to_string(),
            title: self.song.metadata.title.clone(),
            source_project: self.project_path.clone(),
            tracks: self.preset_tracks(),
            instruments: self.preset_instruments(),
            native_devices: self.preset_devices(),
            midi: PresetMidiInventory {
                output_status: self.midi_status.clone(),
                input_status: self.midi_input_status.clone(),
                output_ports: self
                    .midi_ports
                    .iter()
                    .map(|port| port.name.clone())
                    .collect(),
                input_ports: self
                    .midi_input_ports
                    .iter()
                    .map(|port| port.name.clone())
                    .collect(),
            },
            ableton_bridge: PresetBridgeStatus {
                state: "optional_not_configured".to_string(),
                note: "Use the optional Ableton bridge for Ableton preset capture and restore."
                    .to_string(),
            },
        }
    }

    fn preset_tracks(&self) -> Vec<PresetTrack> {
        self.song
            .tracks
            .iter()
            .map(|track| {
                let instrument = self.song.instrument_for_track(track.id);
                let sample = self.song.sample_for_track(track.id);
                PresetTrack {
                    id: track.id.0,
                    name: track.name.clone(),
                    midi_channel: track.midi_channel,
                    assigned_instrument: instrument.map(|instrument| instrument.name.clone()),
                    assigned_sample: sample.map(|sample| sample.path.clone()),
                }
            })
            .collect()
    }

    fn preset_instruments(&self) -> Vec<PresetInstrument> {
        self.song
            .instruments
            .iter()
            .map(|instrument| PresetInstrument {
                id: instrument.id.0,
                name: instrument.name.clone(),
                primary_sample: instrument
                    .primary_sample()
                    .and_then(|sample| self.song.sample_for_id(sample))
                    .map(|sample| sample.path.clone()),
                zone_count: instrument.zones.len(),
            })
            .collect()
    }

    fn preset_devices(&self) -> Vec<PresetDevice> {
        let mut devices = Vec::new();
        for device in &self.song.mixer.master_effects {
            devices.push(preset_device("master", device));
        }
        for track_mixer in &self.song.mixer.tracks {
            let track_name = self
                .song
                .tracks
                .iter()
                .find(|track| track.id == track_mixer.track)
                .map_or_else(
                    || format!("track {}", track_mixer.track.0),
                    |track| track.name.clone(),
                );
            for device in &track_mixer.effects {
                devices.push(preset_device(&track_name, device));
            }
        }
        devices
    }
}

fn command_path(values: &[&str], error: &str) -> Result<PathBuf, String> {
    let path = values.join(" ");
    let path = path.trim();
    if path.is_empty() {
        Err(error.to_string())
    } else {
        Ok(PathBuf::from(path))
    }
}

fn save_preset_profile(path: &Path, profile: &PresetProfile) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(profile)
        .map_err(|error| format!("cannot encode preset profile: {error}"))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn read_preset_profile(path: &Path) -> Result<PresetProfile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let profile = serde_json::from_str::<PresetProfile>(&raw)
        .map_err(|error| format!("invalid preset profile JSON in {}: {error}", path.display()))?;
    if profile.schema != PRESET_PROFILE_SCHEMA {
        return Err(format!(
            "unsupported preset profile schema {:?}",
            profile.schema
        ));
    }
    Ok(profile)
}

fn list_preset_profiles(dir: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    let mut profiles = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot read entry in {}: {error}", dir.display()))?
            .path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("json") {
            if let Ok(profile) = read_preset_profile(&path) {
                profiles.push((path, profile.title));
            }
        }
    }
    profiles.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(profiles)
}

fn preset_device(scope: &str, device: &EffectDevice) -> PresetDevice {
    PresetDevice {
        scope: scope.to_string(),
        id: device.id,
        name: device.name.clone(),
        kind: effect_kind_label(&device.kind).to_string(),
        bypassed: device.bypassed,
    }
}

fn effect_kind_label(kind: &EffectDeviceKind) -> &'static str {
    match kind {
        EffectDeviceKind::Gain { .. } => "gain",
        EffectDeviceKind::Pan { .. } => "pan",
        EffectDeviceKind::Balance { .. } => "balance",
        EffectDeviceKind::StereoWidth { .. } => "stereo_width",
        EffectDeviceKind::PhaseInvert { .. } => "phase_invert",
        EffectDeviceKind::Filter { .. } => "filter",
        EffectDeviceKind::Delay { .. } => "delay",
        EffectDeviceKind::Reverb { .. } => "reverb",
        EffectDeviceKind::Drive { .. } => "drive",
        EffectDeviceKind::Bitcrusher { .. } => "bitcrusher",
        EffectDeviceKind::Chorus { .. } => "chorus",
        EffectDeviceKind::Flanger { .. } => "flanger",
        EffectDeviceKind::Phaser { .. } => "phaser",
        EffectDeviceKind::Compressor { .. } => "compressor",
        EffectDeviceKind::Gate { .. } => "gate",
        EffectDeviceKind::Limiter { .. } => "limiter",
    }
}

fn summarize_preset_profile(profile: &PresetProfile) -> String {
    format!(
        "Preset profile {}: {} track(s), {} instrument(s), {} native device(s), {} MIDI output(s), {} MIDI input(s)",
        profile.title,
        profile.tracks.len(),
        profile.instruments.len(),
        profile.native_devices.len(),
        profile.midi.output_ports.len(),
        profile.midi.input_ports.len()
    )
}

fn format_preset_profile_for_guidance(profile: &PresetProfile) -> String {
    let mut lines = vec![
        format!("Preset profile: {}", profile.title),
        format!(
            "Inventory: {} track(s), {} instrument(s), {} native device(s)",
            profile.tracks.len(),
            profile.instruments.len(),
            profile.native_devices.len()
        ),
        "Use these metadata to guide arrangement, sound selection, and instrument assignment proposals."
            .to_string(),
    ];
    for instrument in &profile.instruments {
        lines.push(format!(
            "Instrument {}: sample={}",
            instrument.name,
            instrument.primary_sample.as_deref().unwrap_or("none")
        ));
    }
    for device in &profile.native_devices {
        lines.push(format!(
            "Device {} on {}: kind={} bypassed={}",
            device.name, device.scope, device.kind, device.bypassed
        ));
    }
    lines.join("\n")
}
