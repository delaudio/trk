#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio backend failed to start: {0}")]
    Start(String),
    #[error("audio backend failed to stop: {0}")]
    Stop(String),
    #[error("audio backend command failed: {0}")]
    Command(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AudioExportError {
    #[error("sample {sample_id} is missing from the offline render set")]
    MissingSample { sample_id: u32 },
    #[error("unsupported sample-rate conversion from {source_sample_rate} Hz to {target_sample_rate} Hz")]
    UnsupportedSampleRateConversion {
        source_sample_rate: u32,
        target_sample_rate: u32,
    },
    #[error("unsupported channel conversion from {source_channels} to {target_channels}")]
    UnsupportedChannelConversion {
        source_channels: u16,
        target_channels: u16,
    },
    #[error("invalid sampler pitch ratio {pitch_ratio}")]
    InvalidPitchRatio { pitch_ratio: f32 },
    #[error("invalid DSP parameter")]
    InvalidDspParameter,
    #[error("rendered audio has {actual} samples, expected {expected}")]
    InvalidBufferLength { expected: usize, actual: usize },
    #[error("rendered audio is too large for a RIFF/WAV file")]
    WavTooLarge,
}
