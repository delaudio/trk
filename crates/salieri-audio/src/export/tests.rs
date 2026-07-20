use salieri_sampler::PreviewBuffer;

use crate::{fixtures::mono_sample, *};

#[test]
fn supported_export_formats_are_explicit() {
    assert_eq!(
        supported_audio_export_formats(),
        &[AudioExportFormat::WavPcm16]
    );
}

#[test]
fn rendered_sampler_events_can_be_encoded_as_wav() {
    let samples = vec![OfflineSamplerSample {
        sample_id: 1,
        buffer: mono_sample(vec![0.5, -0.5]),
    }];
    let rendered = render_sampler_events(
        &samples,
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
            playback: AudioSamplerPlaybackSettings::default(),
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 2,
        },
    )
    .expect("render");

    let bytes = encode_audio(&rendered, AudioExportFormat::WavPcm16).expect("encode");

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
        4
    );
    assert_eq!(i16::from_le_bytes([bytes[44], bytes[45]]), 16_384);
    assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), -16_384);
}

#[test]
fn encodes_wav_pcm16_without_filesystem_side_effects() {
    let audio = RenderedAudio {
        sample_rate: 48_000,
        channels: 1,
        frames: 3,
        data: vec![-1.0, 0.0, 1.0],
    };

    let bytes = encode_audio(&audio, AudioExportFormat::WavPcm16).expect("encode");

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
        6
    );
    assert_eq!(i16::from_le_bytes([bytes[44], bytes[45]]), i16::MIN);
    assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), 0);
    assert_eq!(i16::from_le_bytes([bytes[48], bytes[49]]), i16::MAX);
}

#[test]
fn render_export_failures_are_clear() {
    let preview = PreviewBuffer {
        sample_rate: 44_100,
        channels: 2,
        frames: 1,
        data: vec![0.0, 0.0],
    };

    assert!(matches!(
        render_sampler_preview(
            &preview,
            OfflineRenderSpec {
                sample_rate: 48_000,
                channels: 2,
                frames: 1,
            }
        ),
        Err(AudioExportError::UnsupportedSampleRateConversion {
            source_sample_rate: 44_100,
            target_sample_rate: 48_000
        })
    ));
}
