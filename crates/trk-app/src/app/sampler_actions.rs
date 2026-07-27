use super::*;

impl App {
    pub(crate) fn render_selection_to_sample(
        &mut self,
        path: PathBuf,
        assign_track: Option<usize>,
    ) {
        let Some(selection) = self.selection_bounds() else {
            self.notify_warning("Select tracker rows/tracks before rendering a sample");
            return;
        };
        let Some(pattern) = self.song.pattern(self.pattern_index) else {
            self.notify_warning("No pattern to render");
            return;
        };
        let assign_track_id = if let Some(track_index) = assign_track {
            let Some(track) = self.song.tracks.get(track_index) else {
                self.notify_warning("Track out of range");
                return;
            };
            Some(track.id)
        } else {
            None
        };

        let config = AudioConfig::default();
        let sample_rate = config.sample_rate;
        let channels = config.channels;
        let render_result = render_selection_audio(
            &self.song,
            pattern,
            selection,
            self.sample_base_dir().as_deref(),
            sample_rate,
            channels,
        );
        let (bytes, sampler_event_count) = match render_result {
            Ok(rendered) => rendered,
            Err(error) => {
                self.notify_error(format!("Render selection failed: {error}"));
                return;
            }
        };
        if let Err(error) = write_bytes_atomically(&path, &bytes) {
            self.notify_error(format!("Render selection failed: {error}"));
            return;
        }

        let sample_view = match load_sample_view_data(path.clone()) {
            Ok(sample_view) => sample_view,
            Err(error) => {
                self.notify_error(format!("Rendered sample could not be loaded: {error}"));
                return;
            }
        };
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();
        self.sample_view = Some(sample_view);
        self.sample_waveform_zoom = 1;
        self.sample_waveform_offset = 0;

        self.mutate_song_with(
            TransactionSpec::new("Render selection to sample"),
            |song, _| {
                let sample_id = song.upsert_sample_reference(sample_path, sample_name.clone());
                if let Some(track_id) = assign_track_id {
                    let _ = song.assign_sample_to_track(track_id, sample_id);
                }
            },
        );
        self.selection = None;
        self.focus_panel(FocusPanel::Sampler);
        if sampler_event_count == 0 {
            self.notify_warning(
                "Rendered silent sample; external MIDI-only destinations are not captured",
            );
        } else if assign_track.is_some() {
            self.notify_success(format!(
                "Rendered selection to sample and assigned {sample_name}"
            ));
        } else {
            self.notify_success(format!("Rendered selection to sample {sample_name}"));
        }
    }

    pub(crate) fn assign_loaded_sample_to_track(&mut self, track_index: usize) {
        let Some(sample_view) = &self.sample_view else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before assigning it");
            return;
        };

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };

        let track_id = track.id;
        let track_name = track.name.clone();
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();

        self.mutate_song(|song, _| {
            let sample_id = song.upsert_sample_reference(sample_path, sample_name);
            let _ = song.assign_sample_to_track(track_id, sample_id);
        });
        self.focus_panel(FocusPanel::Sampler);
        self.notify_success(format!("Sample assigned to {track_name}"));
    }

    pub(crate) fn assign_sample_path_to_track(&mut self, path: PathBuf, track_index: usize) {
        let sample_view = match load_sample_view_data(path) {
            Ok(sample_view) => sample_view,
            Err(error) => {
                self.focus_panel(FocusPanel::Sampler);
                self.notify_error(format!("Sample load failed: {error}"));
                return;
            }
        };
        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };

        let track_id = track.id;
        let track_name = track.name.clone();
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();

        self.sample_view = Some(sample_view);
        self.sample_waveform_zoom = 1;
        self.sample_waveform_offset = 0;
        self.mutate_song(|song, _| {
            let sample_id = song.upsert_sample_reference(sample_path, sample_name);
            let _ = song.assign_sample_to_track(track_id, sample_id);
        });
        self.focus_panel(FocusPanel::Sampler);
        self.notify_success(format!("Sample assigned to {track_name}"));
    }

    pub(crate) fn unassign_sample_from_track(&mut self, track_index: usize) {
        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };
        let track_id = track.id;
        let track_name = track.name.clone();

        self.mutate_song(|song, _| {
            song.unassign_sample_from_track(track_id);
        });
        self.notify_success(format!("Sample unassigned from {track_name}"));
    }

    pub(crate) fn replace_track_sample_with_loaded_sample(&mut self, track_index: usize) {
        let Some(sample_view) = &self.sample_view else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before replacing an assignment");
            return;
        };

        let Some(track) = self.song.tracks.get(track_index) else {
            self.notify_warning("Track out of range");
            return;
        };

        let track_id = track.id;
        let track_name = track.name.clone();
        let previous_sample = self
            .song
            .sample_assignment_for_track(track_id)
            .map(|assignment| assignment.sample);
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();

        self.mutate_song(|song, _| {
            let sample_id = song
                .replace_track_sample(track_id, sample_path, sample_name)
                .expect("track exists and sample was just upserted");
            if let Some(previous) = previous_sample {
                if previous != sample_id && !song.is_sample_assigned(previous) {
                    let _ = song.remove_sample_reference(previous);
                }
            }
        });
        self.focus_panel(FocusPanel::Sampler);
        self.notify_success(format!("Sample replaced on {track_name}"));
    }

    pub(crate) fn unload_current_sample(&mut self) {
        let Some(sample_view) = &self.sample_view else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("No sample loaded");
            return;
        };

        let sample_path = sample_view.source_path.to_string_lossy();
        let sample_id = self
            .song
            .samples
            .iter()
            .find(|sample| sample.path == sample_path)
            .map(|sample| sample.id);

        match sample_id {
            Some(sample_id) if self.song.is_sample_assigned(sample_id) => {
                self.focus_panel(FocusPanel::Sampler);
                self.notify_warning("Unassign or replace sample before unloading it");
            }
            Some(sample_id) => {
                self.mutate_song(|song, _| {
                    let _ = song.remove_sample_reference(sample_id);
                });
                self.sample_view = None;
                self.focus_panel(FocusPanel::Sampler);
                self.notify_success("Sample unloaded");
            }
            None => {
                self.sample_view = None;
                self.focus_panel(FocusPanel::Sampler);
                self.notify_info("Sample view cleared");
            }
        }
    }

    pub(crate) fn cleanup_unused_sample_references(&mut self) {
        let mut removed = 0;
        self.mutate_song(|song, _| {
            removed = song.prune_unused_sample_references();
        });

        if removed == 0 {
            self.notify_info("No unused sample references");
        } else {
            self.notify_success(format!("Removed {removed} unused sample reference(s)"));
        }
    }

    pub(crate) fn show_sample_assignments(&mut self) {
        if self.song.sample_assignments.is_empty() {
            self.notify_info("No sample assignments");
            return;
        }

        let assignments = self
            .song
            .sample_assignments
            .iter()
            .filter_map(|assignment| {
                let track = self
                    .song
                    .tracks
                    .iter()
                    .find(|track| track.id == assignment.track)?;
                let sample = self.song.sample_for_id(assignment.sample)?;
                Some(format!("{}={}", track.name, sample.name))
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.notify_info(format!("Samples: {assignments}"));
    }

    pub(crate) fn loaded_sample_playback_settings(&self) -> Option<SamplePlaybackSettings> {
        let sample_view = self.sample_view.as_ref()?;
        let sample_path = sample_view.source_path.to_string_lossy();
        Some(
            self.song
                .samples
                .iter()
                .find(|sample| sample.path == sample_path)
                .map(|sample| sample.playback)
                .unwrap_or_default(),
        )
    }

    pub(crate) fn store_loaded_sample_playback_settings(
        &mut self,
        settings: SamplePlaybackSettings,
    ) -> bool {
        self.store_loaded_sample_playback_settings_with(
            settings,
            TransactionSpec::new("Edit sampler settings"),
        )
    }

    fn store_loaded_sample_playback_settings_with(
        &mut self,
        settings: SamplePlaybackSettings,
        spec: TransactionSpec,
    ) -> bool {
        let Some(sample_view) = &self.sample_view else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return false;
        };

        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();
        self.mutate_song_with(spec, |song, _| {
            let sample_id = song.upsert_sample_reference(sample_path, sample_name);
            song.set_sample_frame_window(sample_id, settings.start_frame, settings.end_frame)
                .expect("sample frame window was prevalidated");
            song.set_sample_loop(
                sample_id,
                settings.mode,
                settings.loop_start_frame,
                settings.loop_end_frame,
            )
            .expect("sample loop was prevalidated");
            song.set_sample_envelope(sample_id, settings.envelope)
                .expect("sample envelope was prevalidated");
        });
        self.focus_panel(FocusPanel::Sampler);
        true
    }

    pub(crate) fn set_loaded_sample_frame_start(&mut self, start_frame: Option<usize>) {
        let Some(mut settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return;
        };
        settings.start_frame = start_frame;
        if let Err(message) = validate_sample_playback_settings(settings) {
            self.notify_warning(message);
            return;
        }
        if self.store_loaded_sample_playback_settings(settings) {
            self.notify_success(format!(
                "Sample start {}",
                format_optional_frame(settings.start_frame)
            ));
        }
    }

    pub(crate) fn set_loaded_sample_frame_end(&mut self, end_frame: Option<usize>) {
        let Some(mut settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return;
        };
        settings.end_frame = end_frame;
        if let Err(message) = validate_sample_playback_settings(settings) {
            self.notify_warning(message);
            return;
        }
        if self.store_loaded_sample_playback_settings(settings) {
            self.notify_success(format!(
                "Sample end {}",
                format_optional_frame(settings.end_frame)
            ));
        }
    }

    pub(crate) fn set_loaded_sample_loop(
        &mut self,
        mode: SamplePlaybackMode,
        loop_start_frame: Option<usize>,
        loop_end_frame: Option<usize>,
    ) {
        let Some(mut settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return;
        };

        settings.mode = mode;
        if mode == SamplePlaybackMode::OneShot {
            settings.loop_start_frame = None;
            settings.loop_end_frame = None;
        } else if loop_start_frame.is_some() || loop_end_frame.is_some() {
            settings.loop_start_frame = loop_start_frame;
            settings.loop_end_frame = loop_end_frame;
        }

        if let Err(message) = validate_sample_playback_settings(settings) {
            self.notify_warning(message);
            return;
        }
        if self.store_loaded_sample_playback_settings(settings) {
            self.notify_success(format!("Sample loop {}", format_sample_loop(settings)));
        }
    }

    pub(crate) fn set_loaded_sample_mode(&mut self, mode: SamplePlaybackMode) {
        let Some(mut settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return;
        };

        settings.mode = mode;
        if !sample_mode_requires_loop_window(mode) {
            settings.loop_start_frame = None;
            settings.loop_end_frame = None;
        }

        if let Err(message) = validate_sample_playback_settings(settings) {
            self.notify_warning(message);
            return;
        }
        if self.store_loaded_sample_playback_settings(settings) {
            self.notify_success(format!("Sample mode {}", format_sample_playback_mode(mode)));
        }
    }

    pub(crate) fn set_loaded_sample_envelope(&mut self, envelope: SampleEnvelope) {
        let Some(mut settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return;
        };

        settings.envelope = envelope;
        if let Err(message) = validate_sample_playback_settings(settings) {
            self.notify_warning(message);
            return;
        }
        if self.store_loaded_sample_playback_settings(settings) {
            self.notify_success(format!(
                "Sample envelope {}",
                format_sample_envelope(envelope)
            ));
        }
    }

    pub(crate) fn next_sampler_envelope_field(&mut self) {
        let field = match self.sampler_envelope_field {
            SamplerEnvelopeField::Attack => SamplerEnvelopeField::Decay,
            SamplerEnvelopeField::Decay => SamplerEnvelopeField::Sustain,
            SamplerEnvelopeField::Sustain => SamplerEnvelopeField::Release,
            SamplerEnvelopeField::Release => SamplerEnvelopeField::Attack,
        };
        self.select_sampler_envelope_field(field);
    }

    pub(crate) fn previous_sampler_envelope_field(&mut self) {
        let field = match self.sampler_envelope_field {
            SamplerEnvelopeField::Attack => SamplerEnvelopeField::Release,
            SamplerEnvelopeField::Decay => SamplerEnvelopeField::Attack,
            SamplerEnvelopeField::Sustain => SamplerEnvelopeField::Decay,
            SamplerEnvelopeField::Release => SamplerEnvelopeField::Sustain,
        };
        self.select_sampler_envelope_field(field);
    }

    pub(crate) fn select_sampler_envelope_field(&mut self, field: SamplerEnvelopeField) {
        self.sampler_envelope_field = field;
        self.notify_info(format!("Envelope {}", sampler_envelope_field_label(field)));
    }

    pub(crate) fn adjust_selected_sampler_envelope(&mut self, direction: f32, coarse: bool) {
        let Some(mut settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before editing playback settings");
            return;
        };

        let field = self.sampler_envelope_field;
        let envelope = &mut settings.envelope;
        let value = match field {
            SamplerEnvelopeField::Attack => {
                envelope.attack_seconds =
                    adjust_sampler_envelope_seconds(envelope.attack_seconds, direction, coarse);
                envelope.attack_seconds
            }
            SamplerEnvelopeField::Decay => {
                envelope.decay_seconds =
                    adjust_sampler_envelope_seconds(envelope.decay_seconds, direction, coarse);
                envelope.decay_seconds
            }
            SamplerEnvelopeField::Sustain => {
                envelope.sustain = adjust_sampler_sustain(envelope.sustain, direction, coarse);
                envelope.sustain
            }
            SamplerEnvelopeField::Release => {
                envelope.release_seconds =
                    adjust_sampler_envelope_seconds(envelope.release_seconds, direction, coarse);
                envelope.release_seconds
            }
        };

        if let Err(message) = validate_sample_playback_settings(settings) {
            self.notify_warning(message);
            return;
        }
        if self.store_loaded_sample_playback_settings_with(
            settings,
            TransactionSpec::merged(
                "Adjust sampler envelope",
                format!("sampler.envelope.{field:?}"),
            ),
        ) {
            self.notify_success(format!(
                "{} {:.3}{}",
                sampler_envelope_field_label(field),
                value,
                if field == SamplerEnvelopeField::Sustain {
                    ""
                } else {
                    "s"
                }
            ));
        }
    }

    pub(crate) fn show_loaded_sample_settings(&mut self) {
        let Some(settings) = self.loaded_sample_playback_settings() else {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Load a sample before viewing playback settings");
            return;
        };
        self.notify_info(format_sample_playback_settings(settings));
    }
}

fn render_selection_audio(
    song: &Song,
    pattern: &Pattern,
    selection: SelectionBounds,
    sample_base_dir: Option<&Path>,
    sample_rate: u32,
    channels: u16,
) -> Result<(Vec<u8>, usize)> {
    let track_ids = (selection.track_start..=selection.track_end)
        .filter_map(|track_index| song.tracks.get(track_index).map(|track| track.id.0))
        .collect::<HashSet<_>>();
    let row_duration = row_duration_micros(&song.transport);
    let start_micros = row_duration.saturating_mul(selection.row_start as u64);
    let selection_rows = selection
        .row_end
        .saturating_sub(selection.row_start)
        .saturating_add(1);
    let frames = micros_to_selection_frames(
        row_duration.saturating_mul(selection_rows as u64),
        sample_rate,
    );
    let events = sampler_events(song, pattern)
        .into_iter()
        .filter(|event| {
            event.position.row >= selection.row_start
                && event.position.row <= selection.row_end
                && track_ids.contains(&event.track.0)
        })
        .map(|event| OfflineSamplerEvent {
            track_id: event.track.0,
            sample_id: event.sample.0,
            frame: micros_to_selection_frames(
                event.position.offset_micros.saturating_sub(start_micros),
                sample_rate,
            ) as u64,
            gain: event.gain,
            pan: event.pan,
            pitch_ratio: event.pitch_ratio,
            velocity: event.velocity,
            playback: audio_sampler_playback_settings(event.playback),
        })
        .collect::<Vec<_>>();
    let samples = load_offline_export_samples(song, sample_rate, channels, sample_base_dir)?;
    let rendered = render_sampler_events_with_dsp(
        &samples,
        &events,
        OfflineRenderSpec {
            sample_rate,
            channels,
            frames,
        },
        &audio_dsp_graph(song),
    )
    .context("failed to render selected sampler events")?;
    let bytes = encode_audio(&rendered, AudioExportFormat::WavPcm16)
        .context("failed to encode rendered selection")?;
    Ok((bytes, events.len()))
}

fn micros_to_selection_frames(micros: u64, sample_rate: u32) -> usize {
    let frames = u128::from(micros).saturating_mul(u128::from(sample_rate)) / 1_000_000;
    usize::try_from(frames).unwrap_or(usize::MAX)
}
