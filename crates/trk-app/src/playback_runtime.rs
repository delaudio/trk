mod audio_dispatch;
#[cfg(test)]
mod fake_backends;
mod logging;
mod midi_dispatch;
mod sample_preload;
mod scheduler;
mod transport;

pub(crate) use sample_preload::{audio_sampler_playback_settings, resolve_sample_path};
#[allow(unused_imports)]
pub use transport::{PlaybackCursor, PlaybackRuntime, PlaybackUpdate};

#[cfg(test)]
use audio_dispatch::PlaybackAudioOutput;
#[cfg(test)]
use logging::MidiLogger;
#[cfg(test)]
use midi_dispatch::PlaybackOutput;
#[cfg(test)]
use sample_preload::load_realtime_samples;
#[cfg(test)]
use scheduler::{
    micros_to_frames, run_pattern, run_pattern_chain, run_sequence, PatternRunResult,
    PlaybackRunContext,
};
#[cfg(test)]
use transport::PlaybackCommand;

#[cfg(test)]
mod tests;
