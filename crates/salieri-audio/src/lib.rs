use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            buffer_frames: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCommand {
    Start,
    Stop,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioUpdate {
    Started(AudioConfig),
    Stopped,
    Shutdown,
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio backend failed to start: {0}")]
    Start(String),
    #[error("audio backend failed to stop: {0}")]
    Stop(String),
}

pub trait AudioBackend: Send + 'static {
    fn start(&mut self, config: AudioConfig) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
}

#[derive(Debug, Default)]
pub struct NullAudioBackend {
    started: bool,
}

impl NullAudioBackend {
    #[must_use]
    pub const fn is_started(&self) -> bool {
        self.started
    }
}

impl AudioBackend for NullAudioBackend {
    fn start(&mut self, _config: AudioConfig) -> Result<(), AudioError> {
        self.started = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.started = false;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AudioRuntime {
    command_tx: Sender<AudioCommand>,
    update_rx: Receiver<AudioUpdate>,
    handle: Option<JoinHandle<()>>,
}

impl AudioRuntime {
    #[must_use]
    pub fn spawn<B>(config: AudioConfig, backend: B) -> Self
    where
        B: AudioBackend,
    {
        let (command_tx, command_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let handle = thread::spawn(move || audio_thread(config, backend, command_rx, update_tx));

        Self {
            command_tx,
            update_rx,
            handle: Some(handle),
        }
    }

    pub fn start(&self) {
        let _ = self.command_tx.send(AudioCommand::Start);
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(AudioCommand::Stop);
    }

    pub fn shutdown(&self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }

    #[must_use]
    pub fn try_recv(&self) -> Option<AudioUpdate> {
        self.update_rx.try_recv().ok()
    }
}

impl Drop for AudioRuntime {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn audio_thread<B>(
    config: AudioConfig,
    mut backend: B,
    command_rx: Receiver<AudioCommand>,
    update_tx: Sender<AudioUpdate>,
) where
    B: AudioBackend,
{
    let mut running = false;
    while let Ok(command) = command_rx.recv() {
        match command {
            AudioCommand::Start => match backend.start(config) {
                Ok(()) => {
                    running = true;
                    let _ = update_tx.send(AudioUpdate::Started(config));
                }
                Err(error) => {
                    let _ = update_tx.send(AudioUpdate::Error(error.to_string()));
                }
            },
            AudioCommand::Stop => {
                if running {
                    match backend.stop() {
                        Ok(()) => {
                            running = false;
                            let _ = update_tx.send(AudioUpdate::Stopped);
                        }
                        Err(error) => {
                            let _ = update_tx.send(AudioUpdate::Error(error.to_string()));
                        }
                    }
                } else {
                    let _ = update_tx.send(AudioUpdate::Stopped);
                }
            }
            AudioCommand::Shutdown => {
                if running {
                    let _ = backend.stop();
                }
                let _ = update_tx.send(AudioUpdate::Shutdown);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RealtimeAudioCommand {
    TriggerSample {
        sample_id: u32,
        frame: u64,
        gain: f32,
        pitch_ratio: f32,
    },
    StopVoice {
        voice_id: u64,
        frame: u64,
    },
    AllNotesOff {
        frame: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn audio_runtime_starts_stops_and_shuts_down_backend() {
        let runtime = AudioRuntime::spawn(AudioConfig::default(), NullAudioBackend::default());

        runtime.start();
        assert_eq!(
            recv_update(&runtime),
            Some(AudioUpdate::Started(AudioConfig::default()))
        );

        runtime.stop();
        assert_eq!(recv_update(&runtime), Some(AudioUpdate::Stopped));

        runtime.shutdown();
        assert_eq!(recv_update(&runtime), Some(AudioUpdate::Shutdown));
    }

    #[test]
    fn stop_is_idempotent_when_audio_is_not_running() {
        let runtime = AudioRuntime::spawn(AudioConfig::default(), NullAudioBackend::default());

        runtime.stop();

        assert_eq!(recv_update(&runtime), Some(AudioUpdate::Stopped));
    }

    #[test]
    fn realtime_commands_are_plain_data_messages() {
        let command = RealtimeAudioCommand::TriggerSample {
            sample_id: 1,
            frame: 128,
            gain: 0.5,
            pitch_ratio: 2.0,
        };

        assert_eq!(
            command,
            RealtimeAudioCommand::TriggerSample {
                sample_id: 1,
                frame: 128,
                gain: 0.5,
                pitch_ratio: 2.0,
            }
        );
    }

    fn recv_update(runtime: &AudioRuntime) -> Option<AudioUpdate> {
        let deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            if let Some(update) = runtime.try_recv() {
                return Some(update);
            }
            thread::sleep(Duration::from_millis(1));
        }
        None
    }
}
