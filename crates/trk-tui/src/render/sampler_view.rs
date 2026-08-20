use super::*;
use crate::{InteractionPayload, SamplerAction};

pub(super) fn render_sampler_view(
    frame: &mut Frame<'_>,
    area: Rect,
    sampler: Option<SamplerViewState<'_>>,
    mut interactions: Option<&mut InteractionMap>,
) {
    let interactive = interactions.is_some();
    let Some(sampler) = sampler else {
        render_empty_sampler(frame, area, interactions);
        return;
    };

    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints(if interactive {
            [
                Constraint::Length(11),
                Constraint::Length(4),
                Constraint::Min(5),
            ]
        } else {
            [
                Constraint::Length(11),
                Constraint::Length(0),
                Constraint::Min(5),
            ]
        })
        .split(area);

    let overview = sampler.overview;
    let assignment = match (sampler.assigned_track, sampler.assigned_track_count) {
        (Some(track), 1) => format!("Assigned: {track}"),
        (Some(track), count) => format!("Assigned: {track} (+{})", count.saturating_sub(1)),
        (None, _) => "Assigned: none".to_string(),
    };
    let lines = vec![
        Line::from(format!("Name: {}", truncate(sampler.name, 48))),
        Line::from(format!("Path: {}", truncate(sampler.source_path, 72))),
        Line::from(format!(
            "Instrument: {}",
            sampler.instrument.unwrap_or("none")
        )),
        Line::from(assignment),
        Line::from(format!(
            "Format: {} Hz · {} ch",
            overview.sample_rate, overview.channels
        )),
        Line::from(format!(
            "Length: {} frames · {:.3} s",
            overview.frames, overview.duration_seconds
        )),
        parameter_control_from_f32(sample_gain_descriptor(), sampler.gain),
        Line::from(format!(
            "Window: {}..{}",
            format_optional_frame(sampler.start_frame),
            format_optional_frame(sampler.end_frame)
        )),
        Line::from(format!(
            "Loop: {} {}",
            sampler.playback_mode,
            format_loop_window(sampler.loop_start_frame, sampler.loop_end_frame)
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(theme::block(" Sample Metadata ")),
        sections[0],
    );
    if let Some(interactions) = interactions.as_deref_mut() {
        render_sampler_controls(frame, sections[1], Some(sampler), interactions);
    }
    if let Some(interactions) = interactions {
        interactions.register(
            interaction_region::SAMPLER_WAVEFORM,
            Rect::new(
                sections[2].x.saturating_add(1),
                sections[2].y.saturating_add(1),
                sections[2].width.saturating_sub(2),
                sections[2].height.saturating_sub(2),
            ),
        );
    }
    render_waveform_overview_with_context(
        frame,
        sections[2],
        overview,
        WaveformWindow {
            start_bucket: sampler.waveform_start_bucket,
            end_bucket: sampler.waveform_end_bucket,
            zoom: sampler.waveform_zoom,
        },
        WaveformGlyphs::Unicode,
        WaveformMarkers {
            sample_start_frame: sampler.start_frame,
            sample_end_frame: sampler.end_frame,
            loop_start_frame: sampler.loop_start_frame,
            loop_end_frame: sampler.loop_end_frame,
        },
        sampler.color_mode,
    );
}

fn render_empty_sampler(
    frame: &mut Frame<'_>,
    area: Rect,
    interactions: Option<&mut InteractionMap>,
) {
    let Some(interactions) = interactions else {
        frame.render_widget(
            Paragraph::new("No sample loaded").block(theme::block(" Sampler ")),
            area,
        );
        return;
    };
    let sections = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);
    frame.render_widget(
        Paragraph::new("No sample loaded").block(theme::block(" Sampler ")),
        sections[0],
    );
    render_sampler_controls(frame, sections[1], None, interactions);
}

pub(super) fn render_sampler_controls(
    frame: &mut Frame<'_>,
    area: Rect,
    sampler: Option<SamplerViewState<'_>>,
    interactions: &mut InteractionMap,
) {
    let block = theme::block(" Controls ");
    let inner = block.inner(area);
    let lines = if let Some(sampler) = sampler {
        vec![
            sampler_envelope_line(sampler, inner, interactions),
            sampler_action_line(inner, 1, interactions),
        ]
    } else {
        vec![browse_only_line(inner, interactions)]
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn sampler_envelope_line(
    sampler: SamplerViewState<'_>,
    inner: Rect,
    interactions: &mut InteractionMap,
) -> Line<'static> {
    let fields = [
        (
            SamplerEnvelopeField::Attack,
            format!(
                "A {}",
                sample_envelope_attack_descriptor()
                    .format_value(&ParameterValue::Seconds(sampler.envelope.0))
            ),
        ),
        (
            SamplerEnvelopeField::Decay,
            format!(
                "D {}",
                sample_envelope_decay_descriptor()
                    .format_value(&ParameterValue::Seconds(sampler.envelope.1))
            ),
        ),
        (
            SamplerEnvelopeField::Sustain,
            format!(
                "S {}",
                sample_envelope_sustain_descriptor()
                    .format_value(&ParameterValue::Percentage(sampler.envelope.2))
            ),
        ),
        (
            SamplerEnvelopeField::Release,
            format!(
                "R {}",
                sample_envelope_release_descriptor()
                    .format_value(&ParameterValue::Seconds(sampler.envelope.3))
            ),
        ),
    ];
    let mut spans = vec![Span::raw("Envelope ")];
    let mut offset = "Envelope ".len() as u16;
    for (index, (field, value)) in fields.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" "));
            offset = offset.saturating_add(1);
        }
        let text = format!("[{value}]");
        let width = text.chars().count() as u16;
        register_action(
            interactions,
            inner,
            offset,
            0,
            width,
            SamplerAction::SelectEnvelope(field),
        );
        let style = if field == sampler.selected_envelope {
            theme::active()
        } else {
            theme::base()
        };
        spans.push(Span::styled(text, style));
        offset = offset.saturating_add(width);
    }
    Line::from(spans)
}

fn sampler_action_line(inner: Rect, row: u16, interactions: &mut InteractionMap) -> Line<'static> {
    let segments = [
        ("Adjust ", None),
        ("[-]", Some(SamplerAction::DecrementEnvelope)),
        (" ", None),
        ("[+]", Some(SamplerAction::IncrementEnvelope)),
        ("  Zoom ", None),
        ("[-]", Some(SamplerAction::ZoomOut)),
        (" ", None),
        ("[+]", Some(SamplerAction::ZoomIn)),
        ("  Pan ", None),
        ("[<]", Some(SamplerAction::PanLeft)),
        (" ", None),
        ("[>]", Some(SamplerAction::PanRight)),
        ("  ", None),
        ("[Browse]", Some(SamplerAction::Browse)),
    ];
    segmented_action_line(segments, inner, row, interactions)
}

fn browse_only_line(inner: Rect, interactions: &mut InteractionMap) -> Line<'static> {
    segmented_action_line(
        [
            ("No sample loaded  ", None),
            ("[Browse]", Some(SamplerAction::Browse)),
        ],
        inner,
        0,
        interactions,
    )
}

fn segmented_action_line<const N: usize>(
    segments: [(&'static str, Option<SamplerAction>); N],
    inner: Rect,
    row: u16,
    interactions: &mut InteractionMap,
) -> Line<'static> {
    let mut spans = Vec::with_capacity(N);
    let mut offset = 0_u16;
    for (text, action) in segments {
        let width = text.chars().count() as u16;
        if let Some(action) = action {
            register_action(interactions, inner, offset, row, width, action);
            spans.push(Span::styled(text, theme::label()));
        } else {
            spans.push(Span::raw(text));
        }
        offset = offset.saturating_add(width);
    }
    Line::from(spans)
}

fn register_action(
    interactions: &mut InteractionMap,
    inner: Rect,
    column: u16,
    row: u16,
    width: u16,
    action: SamplerAction,
) {
    if row >= inner.height || column >= inner.width || width > inner.width.saturating_sub(column) {
        return;
    }
    interactions.register_with_payload(
        interaction_region::SAMPLER_ACTION,
        Rect::new(
            inner.x.saturating_add(column),
            inner.y.saturating_add(row),
            width,
            1,
        ),
        InteractionPayload::SamplerAction { action },
    );
}
