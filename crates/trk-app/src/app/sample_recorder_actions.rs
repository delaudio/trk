use super::*;

impl App {
    pub(crate) fn list_sample_recorder_inputs(&mut self) {
        match CpalAudioInputSource::new().available_inputs() {
            Ok(inputs) if inputs.is_empty() => {
                self.notify_info("No audio input devices available");
            }
            Ok(inputs) => {
                let summary = inputs
                    .iter()
                    .map(|input| {
                        format!(
                            "{}={} ({} ch @ {} Hz)",
                            input.id, input.name, input.channels, input.sample_rate
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                self.notify_info(format!("Audio inputs: {summary}"));
            }
            Err(error) => self.notify_warning(format!("Audio input unavailable: {error}")),
        }
    }

    pub(crate) fn select_sample_recorder_input(&mut self, device_id: &str) {
        match CpalAudioInputSource::new().available_inputs() {
            Ok(inputs) => {
                let Some(input) = inputs.into_iter().find(|input| input.id == device_id) else {
                    self.notify_warning(format!("Audio input {device_id} not found"));
                    return;
                };
                let name = input.name.clone();
                match self.sample_recorder.select_input(input) {
                    Ok(()) => self.notify_success(format!("Recorder input selected: {name}")),
                    Err(error) => self.notify_warning(format!("Recorder input failed: {error}")),
                }
            }
            Err(error) => self.notify_warning(format!("Audio input unavailable: {error}")),
        }
    }

    pub(crate) fn set_sample_recorder_gain(&mut self, gain: f32) {
        match self.sample_recorder.set_gain(gain) {
            Ok(()) => {
                self.notify_success(format!("Recorder gain {:.3}", self.sample_recorder.gain()))
            }
            Err(error) => self.notify_warning(format!("Recorder gain failed: {error}")),
        }
    }

    pub(crate) fn start_sample_recorder(&mut self, max_frames: usize) {
        match self.sample_recorder.start(max_frames) {
            Ok(()) => self.notify_success(format!("Recorder armed for {max_frames} frames")),
            Err(error) => self.notify_warning(format!("Recorder start failed: {error}")),
        }
    }

    pub(crate) fn stop_sample_recorder(&mut self) {
        match self.sample_recorder.stop() {
            Ok(()) => self.notify_success(format!(
                "Recorder stopped: {} frames, peak {:.3}",
                self.sample_recorder.recorded_frames(),
                self.sample_recorder.peak()
            )),
            Err(error) => self.notify_warning(format!("Recorder stop failed: {error}")),
        }
    }

    pub(crate) fn capture_sample_recording(&mut self, max_frames: usize, device_id: Option<&str>) {
        match CpalAudioInputSource::new().capture_bounded(
            device_id,
            max_frames,
            self.sample_recorder.gain(),
        ) {
            Ok(capture) => {
                let device = capture.device.clone();
                let frames = capture.audio.frames;
                let device_name = device.name.clone();
                match self
                    .sample_recorder
                    .load_recorded_audio(capture.audio, Some(device))
                {
                    Ok(()) => self.notify_success(format!(
                        "Recorded {frames} frames from {device_name}; peak {:.3}",
                        self.sample_recorder.peak()
                    )),
                    Err(error) => self.notify_warning(format!("Recorder capture failed: {error}")),
                }
            }
            Err(error) => self.notify_warning(format!("Recorder capture failed: {error}")),
        }
    }

    pub(crate) fn trim_sample_recording(&mut self, start_frame: usize, end_frame: usize) {
        match self.sample_recorder.trim(start_frame, end_frame) {
            Ok(()) => self.notify_success(format!(
                "Recorder trim {start_frame}..{end_frame} ({} frames)",
                end_frame.saturating_sub(start_frame)
            )),
            Err(error) => self.notify_warning(format!("Recorder trim failed: {error}")),
        }
    }

    pub(crate) fn save_sample_recording(&mut self, path: PathBuf) {
        match self.encode_sample_recording() {
            Ok(bytes) => match write_bytes_atomically(&path, &bytes) {
                Ok(()) => {
                    self.notify_success(format!("Recorded sample saved to {}", path.display()));
                }
                Err(error) => self.notify_error(format!("Recorder save failed: {error}")),
            },
            Err(error) => self.notify_warning(format!("Recorder save failed: {error}")),
        }
    }

    pub(crate) fn save_sample_recording_and_load(
        &mut self,
        path: PathBuf,
        assign_track: Option<usize>,
    ) {
        let assign_track_id = if let Some(track_index) = assign_track {
            let Some(track) = self.song.tracks.get(track_index) else {
                self.notify_warning("Track out of range");
                return;
            };
            Some(track.id)
        } else {
            None
        };
        let bytes = match self.encode_sample_recording() {
            Ok(bytes) => bytes,
            Err(error) => {
                self.notify_warning(format!("Recorder save failed: {error}"));
                return;
            }
        };
        if let Err(error) = write_bytes_atomically(&path, &bytes) {
            self.notify_error(format!("Recorder save failed: {error}"));
            return;
        }

        let sample_view = match load_sample_view_data(path.clone()) {
            Ok(sample_view) => sample_view,
            Err(error) => {
                self.notify_error(format!("Recorded sample could not be loaded: {error}"));
                return;
            }
        };
        let sample_name = sample_view.sample.name.clone();
        let sample_path = sample_view.source_path.to_string_lossy().to_string();
        self.sample_view = Some(sample_view);
        self.sample_waveform_zoom = 1;
        self.sample_waveform_offset = 0;

        self.mutate_song_with(TransactionSpec::new("Save recorded sample"), |song, _| {
            let sample_id = song.upsert_sample_reference(sample_path, sample_name.clone());
            if let Some(track_id) = assign_track_id {
                let _ = song.assign_sample_to_track(track_id, sample_id);
            }
        });
        self.focus_panel(FocusPanel::Sampler);
        if assign_track.is_some() && assign_track_id.is_some() {
            self.notify_success(format!(
                "Recorded sample saved, loaded, and assigned {sample_name}"
            ));
        } else {
            self.notify_success(format!("Recorded sample saved and loaded {sample_name}"));
        }
    }

    fn encode_sample_recording(&self) -> Result<Vec<u8>> {
        let rendered = self.sample_recorder.rendered_audio()?;
        encode_audio(&rendered, AudioExportFormat::WavPcm16)
            .context("failed to encode recorded sample")
    }

    #[cfg(test)]
    pub(crate) fn load_fake_sample_recording_for_test(&mut self, audio: trk_audio::RenderedAudio) {
        self.sample_recorder
            .load_recorded_audio(audio, None)
            .expect("fake sample recording");
    }
}
