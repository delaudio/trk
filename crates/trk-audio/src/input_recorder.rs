use crate::offline_render::RenderedAudio;

mod cpal_source;
#[cfg(test)]
mod tests;

pub use cpal_source::CpalAudioInputSource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInputDeviceInfo {
    pub id: String,
    pub name: String,
    pub channels: u16,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioInputCapture {
    pub device: AudioInputDeviceInfo,
    pub audio: RenderedAudio,
}

pub trait AudioInputSource {
    fn available_inputs(&self) -> Result<Vec<AudioInputDeviceInfo>, AudioInputError>;

    fn capture_bounded(
        &self,
        device_id: Option<&str>,
        max_frames: usize,
        gain: f32,
    ) -> Result<AudioInputCapture, AudioInputError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleRecorderStatus {
    #[default]
    Idle,
    Armed,
    Recording,
    Recorded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleRecorder {
    status: SampleRecorderStatus,
    selected_input: Option<AudioInputDeviceInfo>,
    sample_rate: u32,
    channels: u16,
    max_frames: usize,
    gain: f32,
    peak: f32,
    trim_start_frame: usize,
    trim_end_frame: usize,
    data: Vec<f32>,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioInputError {
    #[error("audio input is not available: {0}")]
    Unavailable(String),
    #[error("audio input device is not found: {0}")]
    DeviceNotFound(String),
    #[error("audio recorder cannot {action} while {status:?}")]
    InvalidState {
        action: &'static str,
        status: SampleRecorderStatus,
    },
    #[error("recording must contain at least one frame")]
    EmptyRecording,
    #[error("invalid audio input channel count: {0}")]
    InvalidChannels(u16),
    #[error("trim range must satisfy 0 <= start < end <= recorded frames")]
    InvalidTrimRange,
    #[error("unsupported input sample format: {0}")]
    UnsupportedSampleFormat(String),
}

impl Default for SampleRecorder {
    fn default() -> Self {
        Self {
            status: SampleRecorderStatus::Idle,
            selected_input: None,
            sample_rate: 48_000,
            channels: 2,
            max_frames: 0,
            gain: 1.0,
            peak: 0.0,
            trim_start_frame: 0,
            trim_end_frame: 0,
            data: Vec::new(),
        }
    }
}

impl SampleRecorder {
    #[must_use]
    pub fn status(&self) -> SampleRecorderStatus {
        self.status
    }

    #[must_use]
    pub fn selected_input(&self) -> Option<&AudioInputDeviceInfo> {
        self.selected_input.as_ref()
    }

    #[must_use]
    pub fn gain(&self) -> f32 {
        self.gain
    }

    pub fn set_gain(&mut self, gain: f32) -> Result<(), AudioInputError> {
        if !matches!(
            self.status,
            SampleRecorderStatus::Idle
                | SampleRecorderStatus::Armed
                | SampleRecorderStatus::Recorded
        ) {
            return Err(AudioInputError::InvalidState {
                action: "change gain",
                status: self.status,
            });
        }
        self.gain = sanitize_gain(gain);
        Ok(())
    }

    #[must_use]
    pub fn peak(&self) -> f32 {
        self.peak
    }

    #[must_use]
    pub fn recorded_frames(&self) -> usize {
        let channels = usize::from(self.channels);
        if channels == 0 {
            return 0;
        }
        self.data.len() / channels
    }

    #[must_use]
    pub fn trim_range(&self) -> (usize, usize) {
        (self.trim_start_frame, self.trim_end_frame)
    }

    pub fn select_input(&mut self, input: AudioInputDeviceInfo) -> Result<(), AudioInputError> {
        if matches!(self.status, SampleRecorderStatus::Recording) {
            return Err(AudioInputError::InvalidState {
                action: "select input",
                status: self.status,
            });
        }
        validate_channels(input.channels)?;
        self.sample_rate = input.sample_rate;
        self.channels = input.channels;
        self.selected_input = Some(input);
        self.status = SampleRecorderStatus::Armed;
        self.clear_recording();
        Ok(())
    }

    pub fn start(&mut self, max_frames: usize) -> Result<(), AudioInputError> {
        if !matches!(
            self.status,
            SampleRecorderStatus::Armed | SampleRecorderStatus::Recorded
        ) {
            return Err(AudioInputError::InvalidState {
                action: "start recording",
                status: self.status,
            });
        }
        if self.selected_input.is_none() {
            return Err(AudioInputError::Unavailable(
                "select an input before recording".to_string(),
            ));
        }
        if max_frames == 0 {
            return Err(AudioInputError::EmptyRecording);
        }
        self.max_frames = max_frames;
        self.peak = 0.0;
        self.trim_start_frame = 0;
        self.trim_end_frame = 0;
        self.data.clear();
        self.status = SampleRecorderStatus::Recording;
        Ok(())
    }

    pub fn push_input_interleaved(&mut self, samples: &[f32]) -> Result<usize, AudioInputError> {
        if !matches!(self.status, SampleRecorderStatus::Recording) {
            return Err(AudioInputError::InvalidState {
                action: "receive input",
                status: self.status,
            });
        }
        validate_channels(self.channels)?;
        let channels = usize::from(self.channels);
        let remaining = self.max_frames.saturating_sub(self.recorded_frames());
        let incoming_frames = samples.len() / channels;
        let accepted_frames = incoming_frames.min(remaining);
        let accepted_samples = accepted_frames.saturating_mul(channels);
        for sample in samples.iter().take(accepted_samples) {
            let value = sanitize_sample(*sample * self.gain);
            self.peak = self.peak.max(value.abs());
            self.data.push(value);
        }
        if self.recorded_frames() >= self.max_frames {
            self.finish_recording();
        }
        Ok(accepted_frames)
    }

    pub fn stop(&mut self) -> Result<(), AudioInputError> {
        if !matches!(
            self.status,
            SampleRecorderStatus::Recording | SampleRecorderStatus::Recorded
        ) {
            return Err(AudioInputError::InvalidState {
                action: "stop recording",
                status: self.status,
            });
        }
        self.finish_recording();
        Ok(())
    }

    pub fn load_recorded_audio(
        &mut self,
        audio: RenderedAudio,
        input: Option<AudioInputDeviceInfo>,
    ) -> Result<(), AudioInputError> {
        validate_rendered_audio(&audio)?;
        self.selected_input = input;
        self.sample_rate = audio.sample_rate;
        self.channels = audio.channels;
        self.max_frames = audio.frames;
        self.data = audio.data.into_iter().map(sanitize_sample).collect();
        self.peak = self.data.iter().fold(0.0_f32, |peak, sample| {
            peak.max(sanitize_sample(*sample).abs())
        });
        self.finish_recording();
        Ok(())
    }

    pub fn trim(&mut self, start_frame: usize, end_frame: usize) -> Result<(), AudioInputError> {
        if !matches!(self.status, SampleRecorderStatus::Recorded) {
            return Err(AudioInputError::InvalidState {
                action: "trim recording",
                status: self.status,
            });
        }
        if start_frame >= end_frame || end_frame > self.recorded_frames() {
            return Err(AudioInputError::InvalidTrimRange);
        }
        self.trim_start_frame = start_frame;
        self.trim_end_frame = end_frame;
        Ok(())
    }

    pub fn rendered_audio(&self) -> Result<RenderedAudio, AudioInputError> {
        if !matches!(self.status, SampleRecorderStatus::Recorded) {
            return Err(AudioInputError::InvalidState {
                action: "render recording",
                status: self.status,
            });
        }
        let frames = self.trim_end_frame.saturating_sub(self.trim_start_frame);
        if frames == 0 {
            return Err(AudioInputError::EmptyRecording);
        }
        let channels = usize::from(self.channels);
        let start = self.trim_start_frame.saturating_mul(channels);
        let end = self.trim_end_frame.saturating_mul(channels);
        Ok(RenderedAudio {
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames,
            data: self.data[start..end].to_vec(),
        })
    }

    fn finish_recording(&mut self) {
        self.status = SampleRecorderStatus::Recorded;
        self.trim_start_frame = 0;
        self.trim_end_frame = self.recorded_frames();
    }

    fn clear_recording(&mut self) {
        self.max_frames = 0;
        self.peak = 0.0;
        self.trim_start_frame = 0;
        self.trim_end_frame = 0;
        self.data.clear();
    }
}

fn validate_rendered_audio(audio: &RenderedAudio) -> Result<(), AudioInputError> {
    validate_channels(audio.channels)?;
    let expected = audio.frames.saturating_mul(usize::from(audio.channels));
    if audio.frames == 0 || audio.data.len() != expected {
        return Err(AudioInputError::EmptyRecording);
    }
    Ok(())
}

fn validate_channels(channels: u16) -> Result<(), AudioInputError> {
    if channels == 0 {
        return Err(AudioInputError::InvalidChannels(channels));
    }
    Ok(())
}

fn sanitize_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.max(0.0)
    } else {
        1.0
    }
}

fn sanitize_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
