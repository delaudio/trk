use crate::*;

#[test]
fn cpal_backend_starts_unopened() {
    let backend = CpalAudioBackend::new();

    assert!(!backend.is_started());
}
