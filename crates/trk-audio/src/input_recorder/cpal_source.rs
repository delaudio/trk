use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, StreamConfig,
};

use crate::offline_render::RenderedAudio;

use super::{
    sanitize_gain, sanitize_sample, validate_channels, AudioInputCapture, AudioInputDeviceInfo,
    AudioInputError, AudioInputSource,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct CpalAudioInputSource;

impl CpalAudioInputSource {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl AudioInputSource for CpalAudioInputSource {
    fn available_inputs(&self) -> Result<Vec<AudioInputDeviceInfo>, AudioInputError> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|error| AudioInputError::Unavailable(error.to_string()))?;
        let inputs = devices
            .enumerate()
            .filter_map(|(index, device)| {
                let config = device.default_input_config().ok()?;
                let name = device.name().unwrap_or_else(|_| format!("Input {index}"));
                Some(AudioInputDeviceInfo {
                    id: index.to_string(),
                    name,
                    channels: config.channels(),
                    sample_rate: config.sample_rate().0,
                })
            })
            .collect::<Vec<_>>();
        Ok(inputs)
    }

    fn capture_bounded(
        &self,
        device_id: Option<&str>,
        max_frames: usize,
        gain: f32,
    ) -> Result<AudioInputCapture, AudioInputError> {
        if max_frames == 0 {
            return Err(AudioInputError::EmptyRecording);
        }

        let (device, input) = resolve_cpal_input_device(device_id)?;
        validate_channels(input.channels)?;
        let config = StreamConfig {
            channels: input.channels,
            sample_rate: cpal::SampleRate(input.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };
        let sample_format = device
            .default_input_config()
            .map_err(|error| AudioInputError::Unavailable(error.to_string()))?
            .sample_format();
        let channels = usize::from(input.channels);
        let target_samples = max_frames.saturating_mul(channels);
        let captured = Arc::new(Mutex::new(Vec::with_capacity(target_samples)));
        let completed = Arc::new(AtomicBool::new(false));
        let (done_tx, done_rx) = mpsc::channel();
        let stream = build_bounded_input_stream(
            &device,
            &config,
            sample_format,
            BoundedInputCapture {
                target_samples,
                gain: sanitize_gain(gain),
                captured: Arc::clone(&captured),
                completed: Arc::clone(&completed),
                done_tx,
            },
        )?;
        stream
            .play()
            .map_err(|error| AudioInputError::Unavailable(error.to_string()))?;

        let wait = capture_timeout(max_frames, input.sample_rate);
        if done_rx.recv_timeout(wait).is_err() && !completed.load(Ordering::SeqCst) {
            let captured_len = captured
                .lock()
                .map_err(|_| AudioInputError::Unavailable("capture buffer failed".to_string()))?
                .len();
            if captured_len == 0 {
                return Err(AudioInputError::Unavailable(
                    "timed out before audio input delivered samples".to_string(),
                ));
            }
        }
        drop(stream);

        let mut data = captured
            .lock()
            .map_err(|_| AudioInputError::Unavailable("capture buffer failed".to_string()))?
            .clone();
        data.truncate(target_samples);
        let frames = data.len() / channels;
        if frames == 0 {
            return Err(AudioInputError::EmptyRecording);
        }
        data.truncate(frames.saturating_mul(channels));
        Ok(AudioInputCapture {
            device: input,
            audio: RenderedAudio {
                sample_rate: config.sample_rate.0,
                channels: config.channels,
                frames,
                data,
            },
        })
    }
}

fn resolve_cpal_input_device(
    device_id: Option<&str>,
) -> Result<(cpal::Device, AudioInputDeviceInfo), AudioInputError> {
    let host = cpal::default_host();
    if let Some(device_id) = device_id {
        let target_index = device_id
            .parse::<usize>()
            .map_err(|_| AudioInputError::DeviceNotFound(device_id.to_string()))?;
        let devices = host
            .input_devices()
            .map_err(|error| AudioInputError::Unavailable(error.to_string()))?;
        for (index, device) in devices.enumerate() {
            if index == target_index {
                let input = cpal_device_info(index, &device)?;
                return Ok((device, input));
            }
        }
        return Err(AudioInputError::DeviceNotFound(device_id.to_string()));
    }

    let device = host
        .default_input_device()
        .ok_or_else(|| AudioInputError::Unavailable("no default audio input".to_string()))?;
    let input = cpal_device_info(0, &device)?;
    Ok((device, input))
}

fn cpal_device_info(
    index: usize,
    device: &cpal::Device,
) -> Result<AudioInputDeviceInfo, AudioInputError> {
    let config = device
        .default_input_config()
        .map_err(|error| AudioInputError::Unavailable(error.to_string()))?;
    Ok(AudioInputDeviceInfo {
        id: index.to_string(),
        name: device.name().unwrap_or_else(|_| format!("Input {index}")),
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
    })
}

fn build_bounded_input_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    capture: BoundedInputCapture,
) -> Result<cpal::Stream, AudioInputError> {
    match sample_format {
        SampleFormat::F32 => build_bounded_input_stream_for::<f32>(device, config, capture),
        SampleFormat::I16 => build_bounded_input_stream_for::<i16>(device, config, capture),
        SampleFormat::U16 => build_bounded_input_stream_for::<u16>(device, config, capture),
        sample_format => Err(AudioInputError::UnsupportedSampleFormat(
            sample_format.to_string(),
        )),
    }
}

struct BoundedInputCapture {
    target_samples: usize,
    gain: f32,
    captured: Arc<Mutex<Vec<f32>>>,
    completed: Arc<AtomicBool>,
    done_tx: mpsc::Sender<()>,
}

fn build_bounded_input_stream_for<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    capture: BoundedInputCapture,
) -> Result<cpal::Stream, AudioInputError>
where
    T: Sample + SizedSample + 'static,
    f32: FromSample<T>,
{
    let BoundedInputCapture {
        target_samples,
        gain,
        captured,
        completed,
        done_tx,
    } = capture;
    device
        .build_input_stream(
            config,
            move |input: &[T], _| {
                if completed.load(Ordering::Relaxed) {
                    return;
                }
                let Ok(mut captured) = captured.lock() else {
                    return;
                };
                let remaining = target_samples.saturating_sub(captured.len());
                for sample in input.iter().take(remaining) {
                    captured.push(sanitize_sample(f32::from_sample(*sample) * gain));
                }
                if captured.len() >= target_samples && !completed.swap(true, Ordering::SeqCst) {
                    let _ = done_tx.send(());
                }
            },
            |_error| {},
            None,
        )
        .map_err(|error| AudioInputError::Unavailable(error.to_string()))
}

fn capture_timeout(frames: usize, sample_rate: u32) -> Duration {
    let seconds = if sample_rate == 0 {
        1.0
    } else {
        frames as f64 / f64::from(sample_rate)
    };
    Duration::from_secs_f64(seconds + 3.0).max(Duration::from_secs(3))
}
