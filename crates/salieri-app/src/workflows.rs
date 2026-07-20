use super::*;

pub(crate) fn run_transform_euclidean(args: &TransformEuclideanArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing transform input path")?;
    let output_path = args
        .output_path
        .as_deref()
        .context("missing transform output path")?;
    if args.pattern == 0 {
        anyhow::bail!("--pattern is 1-based and must be greater than zero");
    }
    if args.track == 0 {
        anyhow::bail!("--track is 1-based and must be greater than zero");
    }

    let mut song = load_project(input_path)?;
    let report = apply_euclidean(
        &mut song,
        args.pattern - 1,
        EuclideanRhythm {
            steps: args.steps,
            pulses: args.pulses,
            rotation: args.rotation,
            track: args.track - 1,
            pitch: args.pitch,
            velocity: args.velocity,
        },
    )?;
    save_project(output_path, &song)?;

    println!(
        "Applied Euclidean transform to {} cells and wrote {}",
        report.touched_cells.len(),
        output_path.display()
    );
    Ok(())
}

pub(crate) fn run_sample_inspect(args: &SampleInspectArgs) -> Result<()> {
    let inspection = inspect_sample(args)?;
    match args.format {
        SampleInspectFormat::Text => print!("{}", format_sample_inspection_text(&inspection)),
        SampleInspectFormat::Json => println!("{}", format_sample_inspection_json(&inspection)?),
    }
    Ok(())
}

pub(crate) fn run_export_audio(args: &AudioExportArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing export input path: usage is salieri export audio INPUT OUTPUT")?;
    let output_path = args
        .output_path
        .as_deref()
        .context("missing export output path: usage is salieri export audio INPUT OUTPUT")?;
    let song = load_project(input_path)?;
    let rendered = render_audio_export(&song, args, input_path.parent())?;
    let bytes = encode_audio(&rendered, AudioExportFormat::WavPcm16)
        .context("failed to encode WAV PCM16 audio")?;
    write_bytes_atomically(output_path, &bytes)
        .with_context(|| format!("failed to write audio export {}", output_path.display()))?;

    println!(
        "Exported {} frames at {} Hz to {}",
        rendered.frames,
        rendered.sample_rate,
        output_path.display()
    );
    Ok(())
}

pub(crate) fn run_import_xrns(args: &ImportXrnsArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing import input path: usage is salieri import xrns INPUT OUTPUT")?;
    let output_path = args
        .output_path
        .as_deref()
        .context("missing import output path: usage is salieri import xrns INPUT OUTPUT")?;
    let bytes = fs::read(input_path)
        .with_context(|| format!("failed to read XRNS import {}", input_path.display()))?;
    let exported_sample_paths = if let Some(sample_dir) = &args.sample_dir {
        let prefix = args
            .sample_path_prefix
            .as_deref()
            .map(str::to_string)
            .unwrap_or_else(|| sample_dir.to_string_lossy().to_string());
        extract_xrns_samples_for_project(&bytes, sample_dir, &prefix, args.convert_samples_to_wav)?
    } else {
        HashMap::new()
    };
    let report = if exported_sample_paths.is_empty() {
        import_xrns(&bytes)
    } else {
        import_xrns_with_sample_paths(&bytes, &exported_sample_paths)
    };
    for diagnostic in &report.diagnostics {
        eprintln!(
            "XRNS {:?}: {}{}",
            diagnostic.severity,
            diagnostic.message,
            diagnostic
                .location
                .as_deref()
                .map_or_else(String::new, |location| format!(" ({location})"))
        );
    }
    if report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == XrnsDiagnosticSeverity::Error)
    {
        anyhow::bail!("XRNS import failed; project was not written");
    }
    let song = report
        .song
        .context("XRNS import produced no project; project was not written")?;
    let extracted_sample_count = exported_sample_paths.len();
    let track_count = song.tracks.len();
    let pattern_count = song.patterns.len();
    let sequence_len = song.sequence.len();
    let sample_count = song.samples.len();
    save_project(output_path, &song)?;

    println!(
        "Imported {} to {}: {} tracks, {} patterns, {} sequence entries, {} samples, {} extracted sample files",
        input_path.display(),
        output_path.display(),
        track_count,
        pattern_count,
        sequence_len,
        sample_count,
        extracted_sample_count
    );
    Ok(())
}

pub(crate) fn extract_xrns_samples_for_project(
    xrns_bytes: &[u8],
    sample_dir: &Path,
    sample_path_prefix: &str,
    convert_to_wav: bool,
) -> Result<HashMap<String, String>> {
    let samples = extract_xrns_sample_payloads(xrns_bytes).map_err(anyhow::Error::msg)?;
    fs::create_dir_all(sample_dir)
        .with_context(|| format!("failed to create sample directory {}", sample_dir.display()))?;

    let mut used_names = HashSet::new();
    let mut exported_paths = HashMap::new();
    for sample in samples {
        if !sample.supported && !convert_to_wav {
            continue;
        }
        let file_name = unique_sample_file_name(&sample.source_path, &mut used_names);
        let output_path = sample_dir.join(&file_name);
        if sample.supported {
            fs::write(&output_path, &sample.bytes).with_context(|| {
                format!("failed to write extracted sample {}", output_path.display())
            })?;
        } else {
            convert_sample_payload_to_wav(
                &sample.source_path,
                &sample.format,
                &sample.bytes,
                &output_path,
            )?;
        }
        exported_paths.insert(
            sample.source_path,
            prefixed_sample_path(sample_path_prefix, &file_name),
        );
    }

    Ok(exported_paths)
}

pub(crate) fn convert_sample_payload_to_wav(
    source_path: &str,
    format: &str,
    bytes: &[u8],
    output_path: &Path,
) -> Result<()> {
    let temp_input = output_path.with_extension(format!("salieri-import.{format}"));
    fs::write(&temp_input, bytes)
        .with_context(|| format!("failed to write temporary sample {}", temp_input.display()))?;
    let conversion = ProcessCommand::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&temp_input)
        .arg(output_path)
        .output()
        .with_context(|| "failed to run ffmpeg for XRNS sample conversion")?;
    let _ = fs::remove_file(&temp_input);
    if !conversion.status.success() {
        let stderr = String::from_utf8_lossy(&conversion.stderr);
        anyhow::bail!("failed to convert XRNS sample {source_path} to WAV: {stderr}");
    }
    Ok(())
}

pub(crate) fn prefixed_sample_path(prefix: &str, file_name: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() {
        file_name.to_string()
    } else {
        format!("{prefix}/{file_name}")
    }
}

pub(crate) fn unique_sample_file_name(
    source_path: &str,
    used_names: &mut HashSet<String>,
) -> String {
    let mut base = sample_file_stem(source_path);
    if base.is_empty() {
        base = "sample".to_string();
    }

    let mut candidate = format!("{base}.wav");
    let mut suffix = 2_usize;
    while used_names.contains(&candidate) {
        candidate = format!("{base}-{suffix}.wav");
        suffix += 1;
    }
    used_names.insert(candidate.clone());
    candidate
}

pub(crate) fn sample_file_stem(source_path: &str) -> String {
    let mut parts = source_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let file_stem = parts
        .pop()
        .and_then(|file| file.rsplit_once('.').map(|(stem, _)| stem).or(Some(file)))
        .unwrap_or("sample");
    let instrument = parts
        .iter()
        .rev()
        .find(|part| part.to_ascii_lowercase().starts_with("instrument"));
    let raw = instrument.map_or_else(
        || file_stem.to_string(),
        |instrument| format!("{instrument}-{file_stem}"),
    );
    sanitize_file_stem(&raw)
}

pub(crate) fn sanitize_file_stem(value: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

pub(crate) fn inspect_sample(args: &SampleInspectArgs) -> Result<SampleInspection> {
    let path = args
        .path
        .as_deref()
        .context("missing sample path: usage is salieri sample inspect FILE")?;
    let sample = Sample::load_wav(path)
        .with_context(|| format!("failed to load sample {}", path.display()))?;
    let overview = sample.waveform_overview(args.buckets.max(1));

    Ok(SampleInspection { sample, overview })
}

pub(crate) fn render_audio_export(
    song: &Song,
    args: &AudioExportArgs,
    sample_base_dir: Option<&Path>,
) -> Result<salieri_audio::RenderedAudio> {
    let spec_base = OfflineRenderSpec {
        sample_rate: args.sample_rate,
        channels: args.channels,
        frames: 0,
    };
    let events = if args.sequence {
        sequence_export_events(song, args.sample_rate)
    } else {
        if args.pattern == 0 {
            anyhow::bail!("--pattern is 1-based and must be greater than zero");
        }
        pattern_export_events(song, args.pattern - 1, 0, args.sample_rate)?
    };
    let samples =
        load_offline_export_samples(song, args.sample_rate, args.channels, sample_base_dir)?;
    let frames = if events.is_empty() {
        export_duration_frames(song, args, args.sample_rate)?
    } else {
        0
    };

    render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            frames,
            ..spec_base
        },
        &audio_dsp_graph(song),
    )
    .context("failed to render sampler audio events")
}

pub(crate) fn pattern_export_events(
    song: &Song,
    pattern_index: usize,
    base_micros: u64,
    sample_rate: u32,
) -> Result<Vec<OfflineSamplerEvent>> {
    let pattern = song
        .pattern(pattern_index)
        .with_context(|| format!("pattern {} does not exist", pattern_index + 1))?;
    Ok(sampler_events(song, pattern)
        .into_iter()
        .map(|event| OfflineSamplerEvent {
            track_id: event.track.0,
            sample_id: event.sample.0,
            frame: micros_to_frames(
                base_micros.saturating_add(event.position.offset_micros),
                sample_rate,
            ),
            gain: event.gain,
            pan: event.pan,
            pitch_ratio: event.pitch_ratio,
            velocity: event.velocity,
        })
        .collect())
}

pub(crate) fn audio_dsp_graph(song: &Song) -> DspGraphSpec {
    DspGraphSpec {
        track_chains: song
            .mixer
            .tracks
            .iter()
            .filter(|track| !track.effects.is_empty())
            .map(|track| TrackDspChainSpec {
                track_id: track.track.0,
                devices: track.effects.iter().map(audio_dsp_device).collect(),
            })
            .collect(),
        master: song
            .mixer
            .master_effects
            .iter()
            .map(audio_dsp_device)
            .collect(),
    }
}

pub(crate) fn audio_dsp_device(device: &EffectDevice) -> DspDeviceSpec {
    DspDeviceSpec {
        bypassed: device.bypassed,
        kind: match device.kind {
            EffectDeviceKind::Gain { gain } => AudioDspDeviceKind::Gain { gain },
            EffectDeviceKind::Pan { pan } => AudioDspDeviceKind::Pan { pan },
            EffectDeviceKind::Balance { balance } => AudioDspDeviceKind::Balance { balance },
            EffectDeviceKind::StereoWidth { width } => AudioDspDeviceKind::StereoWidth { width },
            EffectDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            } => AudioDspDeviceKind::PhaseInvert {
                invert_left,
                invert_right,
            },
            EffectDeviceKind::Filter {
                mode,
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
            } => AudioDspDeviceKind::Filter {
                mode: audio_filter_mode(mode),
                cutoff_hz,
                resonance,
                drive_db,
                key_track,
                env_amount,
                mix,
            },
            EffectDeviceKind::Delay {
                sync,
                time_left_ms,
                time_right_ms,
                link_times,
                feedback,
                ping_pong,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
            } => AudioDspDeviceKind::Delay {
                sync,
                time_left_ms,
                time_right_ms,
                link_times,
                feedback,
                ping_pong,
                filter_low_cut_hz,
                filter_high_cut_hz,
                mod_rate_hz,
                mod_depth,
                mix,
                output_db,
            },
        },
    }
}

fn audio_filter_mode(mode: FilterMode) -> AudioDspFilterMode {
    match mode {
        FilterMode::LowPass => AudioDspFilterMode::LowPass,
        FilterMode::HighPass => AudioDspFilterMode::HighPass,
        FilterMode::BandPass => AudioDspFilterMode::BandPass,
        FilterMode::Notch => AudioDspFilterMode::Notch,
    }
}

pub(crate) fn sequence_export_events(song: &Song, sample_rate: u32) -> Vec<OfflineSamplerEvent> {
    let mut base_micros = 0_u64;
    let mut events = Vec::new();
    for pattern_id in &song.sequence {
        let Some(pattern_index) = song
            .patterns
            .iter()
            .position(|pattern| pattern.id == *pattern_id)
        else {
            continue;
        };
        if let Ok(mut pattern_events) =
            pattern_export_events(song, pattern_index, base_micros, sample_rate)
        {
            events.append(&mut pattern_events);
        }
        if let Some(pattern) = song.pattern(pattern_index) {
            base_micros = base_micros.saturating_add(
                row_duration_micros(&song.transport).saturating_mul(pattern.row_count() as u64),
            );
        }
    }
    events
}

pub(crate) fn load_offline_export_samples(
    song: &Song,
    sample_rate: u32,
    channels: u16,
    sample_base_dir: Option<&Path>,
) -> Result<Vec<OfflineSamplerSample>> {
    song.samples
        .iter()
        .map(|reference| {
            let path = resolve_sample_path(&reference.path, sample_base_dir);
            let sample = Sample::load_wav(&path)
                .with_context(|| format!("failed to load sample {}", path.display()))?;
            let preview = apply_sample_playback_settings(
                &sample.preview(Default::default()),
                reference.playback,
            );
            Ok(OfflineSamplerSample {
                sample_id: reference.id.0,
                buffer: prepare_realtime_sample(&preview, sample_rate, channels),
            })
        })
        .collect()
}

pub(crate) fn export_duration_frames(
    song: &Song,
    args: &AudioExportArgs,
    sample_rate: u32,
) -> Result<usize> {
    let duration_micros = if args.sequence {
        song.sequence.iter().fold(0_u64, |duration, pattern_id| {
            let Some(pattern) = song
                .patterns
                .iter()
                .find(|pattern| pattern.id == *pattern_id)
            else {
                return duration;
            };
            duration.saturating_add(
                row_duration_micros(&song.transport).saturating_mul(pattern.row_count() as u64),
            )
        })
    } else {
        let pattern = song
            .pattern(args.pattern - 1)
            .with_context(|| format!("pattern {} does not exist", args.pattern))?;
        row_duration_micros(&song.transport).saturating_mul(pattern.row_count() as u64)
    };
    usize::try_from(micros_to_frames(duration_micros, sample_rate))
        .context("audio export duration is too large")
}

pub(crate) fn micros_to_frames(offset_micros: u64, sample_rate: u32) -> u64 {
    u64::from(sample_rate).saturating_mul(offset_micros) / 1_000_000
}

pub(crate) fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp_path = export_temp_path_for(path);
    {
        let mut file = fs::File::create(&temp_path)
            .with_context(|| format!("failed to create temp file {}", temp_path.display()))?;
        file.write_all(bytes)
            .with_context(|| format!("failed to write temp file {}", temp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync temp file {}", temp_path.display()))?;
    }
    fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to replace export {} with {}",
            path.display(),
            temp_path.display()
        )
    })?;
    Ok(())
}

pub(crate) fn export_temp_path_for(path: &Path) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map_or_else(|| "tmp".to_string(), |value| format!("{value}.tmp"));
    temp_path.set_extension(extension);
    temp_path
}

pub(crate) fn format_sample_inspection_text(inspection: &SampleInspection) -> String {
    let sample = &inspection.sample;
    let overview = &inspection.overview;
    let waveform = compact_waveform_text(&overview.buckets);

    format!(
        "sample: {}\nsample_rate: {}\nchannels: {}\nframes: {}\nduration_seconds: {:.6}\nwaveform_buckets: {}\nwaveform: {}\n",
        sample.name,
        overview.sample_rate,
        overview.channels,
        overview.frames,
        overview.duration_seconds,
        overview.buckets.len(),
        waveform
    )
}

pub(crate) fn format_sample_inspection_json(inspection: &SampleInspection) -> Result<String> {
    let overview = &inspection.overview;
    let buckets = overview
        .buckets
        .iter()
        .map(|bucket| serde_json::json!({ "min": bucket.min, "max": bucket.max }))
        .collect::<Vec<_>>();
    let output = serde_json::json!({
        "schema_version": 1,
        "sample": {
            "name": inspection.sample.name,
            "sample_rate": overview.sample_rate,
            "channels": overview.channels,
            "frames": overview.frames,
            "duration_seconds": overview.duration_seconds,
        },
        "waveform": {
            "bucket_count": overview.buckets.len(),
            "buckets": buckets,
        }
    });

    serde_json::to_string_pretty(&output).context("failed to encode sample inspection JSON")
}

pub(crate) fn compact_waveform_text(buckets: &[WaveformBucket]) -> String {
    const GLYPHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if buckets.is_empty() {
        return "<empty>".to_string();
    }

    buckets
        .iter()
        .map(|bucket| {
            let amplitude = bucket.min.abs().max(bucket.max.abs()).clamp(0.0, 1.0);
            let index = (amplitude * (GLYPHS.len() - 1) as f32).round() as usize;
            GLYPHS[index]
        })
        .collect()
}
