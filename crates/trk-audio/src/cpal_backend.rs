use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use trk_sampler::PreviewBuffer;

use crate::{
    backend::{AudioBackend, AudioConfig},
    calibration::CalibrationControl,
    dsp::DspGraphSpec,
    errors::AudioError,
    realtime_sampler::{RealtimeAudioCommand, RealtimeSampler, RealtimeSamplerConfig},
};

#[cfg(test)]
mod tests;

pub struct CpalAudioBackend {
    worker: Option<CpalStreamWorker>,
    calibration: CalibrationControl,
}

impl Default for CpalAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CpalAudioBackend {
    #[must_use]
    pub fn new() -> Self {
        Self::with_calibration(CalibrationControl::new())
    }

    #[must_use]
    pub fn with_calibration(calibration: CalibrationControl) -> Self {
        Self {
            worker: None,
            calibration,
        }
    }

    #[must_use]
    pub fn is_started(&self) -> bool {
        self.worker.is_some()
    }

    pub fn register_sample(&self, sample_id: u32, buffer: PreviewBuffer) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::RegisterSample { sample_id, buffer })
    }

    pub fn send_realtime_command(&self, command: RealtimeAudioCommand) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::Realtime(command))
    }

    pub fn clear_samples(&self) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::ClearSamples)
    }

    pub fn set_dsp_graph(&self, graph: DspGraphSpec) -> Result<(), AudioError> {
        self.send_stream_command(CpalStreamCommand::SetDspGraph(graph))
    }

    fn send_stream_command(&self, command: CpalStreamCommand) -> Result<(), AudioError> {
        let Some(worker) = &self.worker else {
            return Err(AudioError::Command(
                "CPAL stream is not started".to_string(),
            ));
        };
        worker
            .command_tx
            .send(command)
            .map_err(|error| AudioError::Command(format!("CPAL stream command failed: {error}")))
    }
}

impl AudioBackend for CpalAudioBackend {
    fn start(&mut self, config: AudioConfig) -> Result<(), AudioError> {
        if self.worker.is_some() {
            return Ok(());
        }

        let (command_tx, command_rx) = mpsc::channel();
        let (startup_tx, startup_rx) = mpsc::channel();
        let calibration = self.calibration.clone();
        let handle = thread::spawn(move || {
            cpal_stream_thread(config, command_rx, startup_tx, calibration);
        });

        match startup_rx.recv() {
            Ok(Ok(())) => {
                self.worker = Some(CpalStreamWorker { command_tx, handle });
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = handle.join();
                Err(error)
            }
            Err(error) => {
                let _ = handle.join();
                Err(AudioError::Start(format!(
                    "cpal stream thread failed before startup: {error}"
                )))
            }
        }
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if let Some(worker) = self.worker.take() {
            let _ = worker.command_tx.send(CpalStreamCommand::Stop);
            worker
                .handle
                .join()
                .map_err(|_| AudioError::Stop("cpal stream thread panicked".to_string()))?;
        }
        self.calibration.clear_meters();
        Ok(())
    }
}

impl Drop for CpalAudioBackend {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct CpalStreamWorker {
    command_tx: Sender<CpalStreamCommand>,
    handle: JoinHandle<()>,
}

enum CpalStreamCommand {
    RegisterSample {
        sample_id: u32,
        buffer: PreviewBuffer,
    },
    ClearSamples,
    SetDspGraph(DspGraphSpec),
    Realtime(RealtimeAudioCommand),
    Stop,
}

fn cpal_stream_thread(
    config: AudioConfig,
    command_rx: Receiver<CpalStreamCommand>,
    startup_tx: Sender<Result<(), AudioError>>,
    calibration: CalibrationControl,
) {
    let (realtime_tx, realtime_rx) = mpsc::channel();
    match start_realtime_cpal_stream(config, realtime_rx, calibration) {
        Ok(stream) => {
            let _ = startup_tx.send(Ok(()));
            while let Ok(command) = command_rx.recv() {
                if matches!(command, CpalStreamCommand::Stop) {
                    break;
                }
                let _ = realtime_tx.send(command);
            }
            let _ = stream.pause();
        }
        Err(error) => {
            let _ = startup_tx.send(Err(error));
        }
    }
}

fn start_realtime_cpal_stream(
    config: AudioConfig,
    command_rx: Receiver<CpalStreamCommand>,
    calibration: CalibrationControl,
) -> Result<Stream, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AudioError::Start("no default output device".to_string()))?;
    let default_config = device
        .default_output_config()
        .map_err(|error| AudioError::Start(format!("default output config failed: {error}")))?;
    let sample_format = default_config.sample_format();
    let stream_config = StreamConfig {
        channels: config.channels,
        sample_rate: cpal::SampleRate(config.sample_rate),
        buffer_size: cpal::BufferSize::Fixed(u32::from(config.buffer_frames)),
    };

    let stream = build_realtime_output_stream(
        &device,
        &stream_config,
        sample_format,
        command_rx,
        calibration,
    )?;
    stream
        .play()
        .map_err(|error| AudioError::Start(format!("failed to play stream: {error}")))?;
    Ok(stream)
}

fn build_realtime_output_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    command_rx: Receiver<CpalStreamCommand>,
    calibration: CalibrationControl,
) -> Result<Stream, AudioError> {
    match sample_format {
        SampleFormat::F32 => {
            build_realtime_output_stream_for::<f32>(device, config, command_rx, calibration)
        }
        SampleFormat::I16 => {
            build_realtime_output_stream_for::<i16>(device, config, command_rx, calibration)
        }
        SampleFormat::U16 => {
            build_realtime_output_stream_for::<u16>(device, config, command_rx, calibration)
        }
        sample_format => Err(AudioError::Start(format!(
            "unsupported output sample format {sample_format}"
        ))),
    }
}

fn build_realtime_output_stream_for<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    command_rx: Receiver<CpalStreamCommand>,
    calibration: CalibrationControl,
) -> Result<Stream, AudioError>
where
    T: Sample + SizedSample + FromSample<f32> + 'static,
{
    let sampler_config = RealtimeSamplerConfig {
        sample_rate: config.sample_rate.0,
        channels: config.channels,
        max_voices: RealtimeSamplerConfig::default().max_voices,
    };
    let mut sampler = RealtimeSampler::with_calibration(sampler_config, calibration);
    let mut scratch = vec![0.0; usize::from(config.channels) * config.buffer_size_frame_hint()];

    device
        .build_output_stream(
            config,
            move |output, _| {
                write_realtime_output::<T>(output, &mut scratch, &mut sampler, &command_rx);
            },
            |error| {
                let _ = error;
            },
            None,
        )
        .map_err(|error| AudioError::Start(format!("failed to build output stream: {error}")))
}

trait StreamConfigFrameHint {
    fn buffer_size_frame_hint(&self) -> usize;
}

impl StreamConfigFrameHint for StreamConfig {
    fn buffer_size_frame_hint(&self) -> usize {
        match self.buffer_size {
            cpal::BufferSize::Fixed(frames) => frames as usize,
            cpal::BufferSize::Default => 512,
        }
    }
}

fn write_realtime_output<T>(
    output: &mut [T],
    scratch: &mut Vec<f32>,
    sampler: &mut RealtimeSampler,
    command_rx: &Receiver<CpalStreamCommand>,
) where
    T: Sample + FromSample<f32>,
{
    while let Ok(command) = command_rx.try_recv() {
        match command {
            CpalStreamCommand::RegisterSample { sample_id, buffer } => {
                let _ = sampler.register_sample(sample_id, buffer);
            }
            CpalStreamCommand::ClearSamples => {
                sampler.clear_samples();
            }
            CpalStreamCommand::SetDspGraph(graph) => {
                sampler.set_dsp_graph(graph);
            }
            CpalStreamCommand::Realtime(command) => {
                let _ = sampler.handle_command_now(command);
            }
            CpalStreamCommand::Stop => {}
        }
    }

    if scratch.len() != output.len() {
        scratch.resize(output.len(), 0.0);
    }
    sampler.render_into(scratch);

    for (output, sample) in output.iter_mut().zip(scratch.iter()) {
        *output = T::from_sample((*sample).clamp(-1.0, 1.0));
    }
}
