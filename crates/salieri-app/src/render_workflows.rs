use super::*;

use crate::workflows::{
    audio_dsp_graph, export_duration_frames, load_offline_export_samples, pattern_export_events,
    sanitize_file_stem, sequence_export_events, write_bytes_atomically,
};

pub(crate) fn run_export_plan(args: &RenderPlanArgs) -> Result<()> {
    let input_path = args.input_path.as_deref().context(
        "missing render-plan input path: usage is salieri export plan INPUT [OUTPUT.json]",
    )?;
    let song = load_project(input_path)?;
    let plan = render_plan(&song, args)?;
    let json = serde_json::to_string_pretty(&plan).context("failed to encode render plan")?;
    if let Some(output_path) = &args.output_path {
        write_bytes_atomically(output_path, format!("{json}\n").as_bytes())
            .with_context(|| format!("failed to write render plan {}", output_path.display()))?;
        println!("Wrote render plan to {}", output_path.display());
    } else {
        println!("{json}");
    }
    Ok(())
}

pub(crate) fn run_export_stems(args: &RenderStemsArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing stem input path: usage is salieri export stems INPUT OUT_DIR")?;
    let output_dir = args
        .output_dir
        .as_deref()
        .context("missing stem output directory: usage is salieri export stems INPUT OUT_DIR")?;
    let song = load_project(input_path)?;
    let manifest = export_stems(&song, args, input_path.parent(), output_dir)?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to encode stem manifest")?;
    let manifest_path = output_dir.join("stems.json");
    write_bytes_atomically(&manifest_path, format!("{manifest_json}\n").as_bytes())
        .with_context(|| format!("failed to write stem manifest {}", manifest_path.display()))?;
    println!(
        "Exported {} stem(s) and manifest {}",
        manifest.stems.len(),
        manifest_path.display()
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenderPlan {
    pub(crate) schema_version: u8,
    pub(crate) target: String,
    pub(crate) pattern: Option<usize>,
    pub(crate) sequence: bool,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) tracks: Vec<RenderPlanTrack>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenderPlanTrack {
    pub(crate) track: usize,
    pub(crate) name: String,
    pub(crate) sampler_events: usize,
    pub(crate) selected: bool,
    pub(crate) internal_audio: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StemExportManifest {
    pub(crate) schema_version: u8,
    pub(crate) target: String,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) stems: Vec<StemExportEntry>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StemExportEntry {
    pub(crate) track: usize,
    pub(crate) name: String,
    pub(crate) file: PathBuf,
    pub(crate) sampler_events: usize,
    pub(crate) frames: usize,
}

pub(crate) fn render_plan(song: &Song, args: &RenderPlanArgs) -> Result<RenderPlan> {
    let selected = selected_track_numbers(song, &args.tracks)?;
    let events = render_scope_events(song, args.pattern, args.sequence, args.sample_rate)?;
    let mut event_counts = HashMap::<u32, usize>::new();
    for event in &events {
        *event_counts.entry(event.track_id).or_insert(0) += 1;
    }
    let tracks = song
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let track_number = index + 1;
            let sampler_events = event_counts.get(&track.id.0).copied().unwrap_or(0);
            RenderPlanTrack {
                track: track_number,
                name: track.name.clone(),
                sampler_events,
                selected: selected.contains(&track_number),
                internal_audio: sampler_events > 0,
            }
        })
        .collect();
    Ok(RenderPlan {
        schema_version: 1,
        target: if args.sequence {
            "sequence".to_string()
        } else {
            "pattern".to_string()
        },
        pattern: (!args.sequence).then_some(args.pattern),
        sequence: args.sequence,
        sample_rate: args.sample_rate,
        channels: args.channels,
        tracks,
        limitations: render_limitations(),
    })
}

pub(crate) fn export_stems(
    song: &Song,
    args: &RenderStemsArgs,
    sample_base_dir: Option<&Path>,
    output_dir: &Path,
) -> Result<StemExportManifest> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create stem directory {}", output_dir.display()))?;
    let selected = selected_track_numbers(song, &args.tracks)?;
    let events = render_scope_events(song, args.pattern, args.sequence, args.sample_rate)?;
    let samples =
        load_offline_export_samples(song, args.sample_rate, args.channels, sample_base_dir)?;
    let duration_args = AudioExportArgs {
        input_path: None,
        output_path: None,
        pattern: args.pattern,
        sequence: args.sequence,
        sample_rate: args.sample_rate,
        channels: args.channels,
    };
    let silence_frames = export_duration_frames(song, &duration_args, args.sample_rate)?;
    let dsp_graph = audio_dsp_graph(song);
    let mut stems = Vec::new();
    for track_number in selected {
        let track = song
            .tracks
            .get(track_number - 1)
            .with_context(|| format!("track {track_number} does not exist"))?;
        let track_events = events
            .iter()
            .copied()
            .filter(|event| event.track_id == track.id.0)
            .collect::<Vec<_>>();
        let rendered = render_sampler_events_with_dsp(
            &samples,
            &track_events,
            OfflineRenderSpec {
                sample_rate: args.sample_rate,
                channels: args.channels,
                frames: if track_events.is_empty() {
                    silence_frames
                } else {
                    0
                },
            },
            &dsp_graph,
        )
        .context("failed to render stem")?;
        let file_name = format!(
            "track-{:02}-{}.wav",
            track_number,
            sanitize_file_stem(&track.name)
        );
        let output_path = output_dir.join(&file_name);
        let bytes = encode_audio(&rendered, AudioExportFormat::WavPcm16)
            .context("failed to encode stem WAV")?;
        write_bytes_atomically(&output_path, &bytes)
            .with_context(|| format!("failed to write stem {}", output_path.display()))?;
        stems.push(StemExportEntry {
            track: track_number,
            name: track.name.clone(),
            file: PathBuf::from(file_name),
            sampler_events: track_events.len(),
            frames: rendered.frames,
        });
    }
    Ok(StemExportManifest {
        schema_version: 1,
        target: if args.sequence {
            "sequence".to_string()
        } else {
            "pattern".to_string()
        },
        sample_rate: args.sample_rate,
        channels: args.channels,
        stems,
        limitations: render_limitations(),
    })
}

fn render_scope_events(
    song: &Song,
    pattern: usize,
    sequence: bool,
    sample_rate: u32,
) -> Result<Vec<OfflineSamplerEvent>> {
    if sequence {
        Ok(sequence_export_events(song, sample_rate))
    } else {
        if pattern == 0 {
            anyhow::bail!("--pattern is 1-based and must be greater than zero");
        }
        pattern_export_events(song, pattern - 1, 0, sample_rate)
    }
}

fn selected_track_numbers(song: &Song, tracks: &[usize]) -> Result<Vec<usize>> {
    let selected = if tracks.is_empty() {
        (1..=song.tracks.len()).collect::<Vec<_>>()
    } else {
        tracks.to_vec()
    };
    for track in &selected {
        if *track == 0 || *track > song.tracks.len() {
            anyhow::bail!("track {track} does not exist");
        }
    }
    Ok(selected)
}

fn render_limitations() -> Vec<String> {
    vec![
        "Only internal sampler/native audio events are rendered.".to_string(),
        "External MIDI-only destinations are not captured.".to_string(),
    ]
}
