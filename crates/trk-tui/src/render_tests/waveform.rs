use super::render_test_support::*;
use super::*;

#[test]
fn waveform_lines_degrade_to_narrow_widths() {
    let overview = test_waveform(vec![trk_sampler::WaveformBucket {
        min: -1.0,
        max: 1.0,
    }]);

    let lines = waveform_lines(
        &overview,
        WaveformWindow::full(&overview),
        1,
        4,
        WaveformGlyphs::Unicode,
    );

    assert_eq!(lines.len(), 4);
    assert!(lines
        .iter()
        .all(|line| line_text(line).chars().count() == 1));
}

#[test]
fn waveform_lines_support_ascii_glyphs() {
    let overview = test_waveform(vec![trk_sampler::WaveformBucket {
        min: -0.5,
        max: 0.5,
    }]);

    let lines = waveform_lines(
        &overview,
        WaveformWindow::full(&overview),
        8,
        2,
        WaveformGlyphs::Ascii,
    );
    let rendered = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains('#'));
    assert!(!rendered.contains('█'));
}

#[test]
fn waveform_lines_use_half_block_resolution() {
    let overview = test_waveform(vec![trk_sampler::WaveformBucket { min: 0.2, max: 0.2 }]);

    let lines = waveform_lines(
        &overview,
        WaveformWindow::full(&overview),
        8,
        6,
        WaveformGlyphs::Unicode,
    );
    let rendered = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains('▀') || rendered.contains('▄'));
}

#[test]
fn waveform_lines_preserve_peaks_when_downsampling() {
    let overview = test_waveform(vec![
        trk_sampler::WaveformBucket { min: 0.0, max: 0.0 },
        trk_sampler::WaveformBucket {
            min: -1.0,
            max: 1.0,
        },
        trk_sampler::WaveformBucket { min: 0.0, max: 0.0 },
        trk_sampler::WaveformBucket { min: 0.0, max: 0.0 },
    ]);

    let lines = waveform_lines(
        &overview,
        WaveformWindow::full(&overview),
        2,
        6,
        WaveformGlyphs::Unicode,
    );
    let rendered = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains('█'));
}
