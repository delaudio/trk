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

#[test]
fn waveform_styles_zero_crossings_and_attack_transients() {
    let overview = test_waveform(vec![
        trk_sampler::WaveformBucket {
            min: -0.3,
            max: 0.3,
        },
        trk_sampler::WaveformBucket {
            min: -0.1,
            max: 0.9,
        },
    ]);

    let lines = waveform_lines_with_style(
        &overview,
        WaveformWindow::full(&overview),
        2,
        2,
        WaveformGlyphs::Unicode,
        WaveformMarkers::default(),
        TerminalColorMode::TrueColor,
    );
    let styles = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter(|span| span.content != " ")
        .map(|span| span.style)
        .collect::<Vec<_>>();

    assert!(styles.iter().any(|style| style
        .fg
        .is_some_and(|color| matches!(color, Color::Rgb(..)))));
    assert!(styles
        .iter()
        .any(|style| style.add_modifier.contains(Modifier::UNDERLINED)));
    assert!(styles
        .iter()
        .any(|style| style.add_modifier.contains(Modifier::BOLD)));
    assert!(!is_attack_transient(&[0.3, 0.4], 1, 0.0));
    assert!(is_attack_transient(&[0.3, 0.9], 1, 0.0));
    assert!(!is_attack_transient(&[0.9], 0, 0.9));
    assert!(is_attack_transient(&[0.9], 0, 0.2));
}

#[test]
fn panned_waveform_uses_the_preceding_bucket_for_edge_transients() {
    let overview = test_waveform(vec![
        trk_sampler::WaveformBucket {
            min: -0.9,
            max: 0.9,
        },
        trk_sampler::WaveformBucket {
            min: -0.9,
            max: 0.9,
        },
    ]);
    let lines = waveform_lines_with_style(
        &overview,
        WaveformWindow {
            start_bucket: 1,
            end_bucket: 2,
            zoom: 1,
        },
        1,
        1,
        WaveformGlyphs::Unicode,
        WaveformMarkers::default(),
        TerminalColorMode::TrueColor,
    );

    assert!(lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter(|span| span.content != " ")
        .all(|span| !span.style.add_modifier.contains(Modifier::BOLD)));
}

#[test]
fn waveform_fallback_modes_never_emit_unsupported_colors() {
    let overview = test_waveform(vec![trk_sampler::WaveformBucket {
        min: -0.8,
        max: 0.8,
    }]);

    for mode in [
        TerminalColorMode::Indexed256,
        TerminalColorMode::Ansi16,
        TerminalColorMode::Monochrome,
    ] {
        let lines = waveform_lines_with_style(
            &overview,
            WaveformWindow::full(&overview),
            2,
            2,
            WaveformGlyphs::Unicode,
            WaveformMarkers::default(),
            mode,
        );
        let colors = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .filter_map(|span| span.style.fg)
            .collect::<Vec<_>>();
        assert!(!colors.iter().any(|color| matches!(color, Color::Rgb(..))));
        match mode {
            TerminalColorMode::Indexed256 => {
                assert!(colors
                    .iter()
                    .any(|color| matches!(color, Color::Indexed(_))));
            }
            TerminalColorMode::Ansi16 => {
                assert!(!colors.is_empty());
                assert!(!colors
                    .iter()
                    .any(|color| matches!(color, Color::Indexed(_))));
            }
            TerminalColorMode::Monochrome => assert!(colors.is_empty()),
            TerminalColorMode::TrueColor => unreachable!(),
        }
    }
}

#[test]
fn waveform_marker_projection_is_bounded_and_rejects_invalid_pairs() {
    let overview = test_waveform(vec![trk_sampler::WaveformBucket { min: 0.0, max: 0.0 }; 10]);
    let projected = project_waveform_markers(
        &overview,
        WaveformWindow::full(&overview),
        11,
        WaveformMarkers {
            sample_start_frame: Some(0),
            sample_end_frame: Some(overview.frames),
            loop_start_frame: Some(overview.frames / 4),
            loop_end_frame: Some(overview.frames * 3 / 4),
        },
    );

    assert_eq!(projected[0], Some(WaveformMarkerKind::SampleStart));
    assert_eq!(projected[10], Some(WaveformMarkerKind::SampleEnd));
    assert_eq!(projected[3], Some(WaveformMarkerKind::LoopStart));
    assert_eq!(projected[8], Some(WaveformMarkerKind::LoopEnd));

    let invalid = project_waveform_markers(
        &overview,
        WaveformWindow::full(&overview),
        8,
        WaveformMarkers {
            sample_start_frame: Some(90),
            sample_end_frame: Some(20),
            loop_start_frame: Some(10),
            loop_end_frame: None,
        },
    );
    assert!(invalid.into_iter().all(|marker| marker.is_none()));
}
