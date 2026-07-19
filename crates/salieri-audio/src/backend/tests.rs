use crate::{fixtures::recv_update, *};

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
