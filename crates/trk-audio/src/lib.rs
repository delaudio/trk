//! Audio runtime, realtime sampling, DSP, rendering, and export boundaries.

pub mod backend;
#[cfg(feature = "c-dsp-boundary")]
pub mod c_dsp;
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
pub mod wasm_dsp;

pub use backend::{
    AudioBackend, AudioCommand, AudioConfig, AudioRuntime, AudioUpdate, NullAudioBackend,
};
#[cfg(feature = "c-dsp-boundary")]
pub use c_dsp::{
    CNativeGainDescriptor, CNativeGainModule, CNativeGainParameterDescriptor,
    CNativeGainParameterValue, CNativeGainSpec, CNativeGainState, C_NATIVE_GAIN_GAIN_PARAMETER_ID,
    C_NATIVE_GAIN_MODULE_ID,
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
pub use wasm_dsp::{
    render_wasm_dsp_gain_fixture, validate_wasm_dsp_render_request, WasmDspAbiError,
    WasmDspAbiSpec, WasmDspParameterValue, WASM_DSP_ABI_VERSION, WASM_DSP_MAX_BLOCK_FRAMES,
    WASM_DSP_MAX_CHANNELS,
};
