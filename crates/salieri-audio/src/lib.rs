//! Audio runtime, realtime sampling, DSP, rendering, and export boundaries.

pub mod backend;
pub mod cpal_backend;
pub mod dsp;
pub mod errors;
pub mod export;
#[cfg(test)]
mod fixtures;
pub mod input_recorder;
pub mod levels;
pub mod native_module;
pub mod offline_render;
pub mod realtime_sampler;
mod shared;

pub use backend::{
    AudioBackend, AudioCommand, AudioConfig, AudioRuntime, AudioUpdate, NullAudioBackend,
};
pub use cpal_backend::CpalAudioBackend;
pub use dsp::{
    DspDeviceKind, DspDeviceSpec, DspDriveMode, DspDynamicsDetector, DspFilterMode, DspGraphSpec,
    SendDspBusSpec, TrackDspChainSpec, TrackSendSpec,
};
pub use errors::{AudioError, AudioExportError};
pub use export::{encode_audio, supported_audio_export_formats, AudioExportFormat};
pub use input_recorder::{
    AudioInputCapture, AudioInputDeviceInfo, AudioInputError, AudioInputSource,
    CpalAudioInputSource, SampleRecorder, SampleRecorderStatus,
};
pub use levels::{measure_levels, LevelMeter};
pub use native_module::{
    NativeEffectModule, NativeEffectModuleSpec, NativeEffectParameterValue, NativeModulePrepareSpec,
};
pub use offline_render::{
    render_sampler_events, render_sampler_events_with_dsp, render_sampler_preview,
    AudioSamplerPlaybackMode, AudioSamplerPlaybackSettings, OfflineRenderSpec, OfflineSamplerEvent,
    OfflineSamplerSample, RenderedAudio,
};
pub use realtime_sampler::{
    apply_preview_envelope, prepare_realtime_sample, slice_preview_buffer, RealtimeAudioCommand,
    RealtimeSampler, RealtimeSamplerConfig,
};
