use std::{
    thread,
    time::{Duration, Instant},
};

use salieri_sampler::PreviewBuffer;

use crate::{AudioRuntime, AudioUpdate};

pub(crate) fn recv_update(runtime: &AudioRuntime) -> Option<AudioUpdate> {
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        if let Some(update) = runtime.try_recv() {
            return Some(update);
        }
        thread::sleep(Duration::from_millis(1));
    }
    None
}

pub(crate) fn mono_sample(data: Vec<f32>) -> PreviewBuffer {
    PreviewBuffer {
        sample_rate: 48_000,
        channels: 1,
        frames: data.len(),
        data,
    }
}

pub(crate) fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}
