use super::*;

fn context() -> CommandPaletteContext {
    CommandPaletteContext {
        active_view: TuiView::Pattern,
        dirty: false,
        is_playing: false,
        has_selection: false,
        has_loaded_sample: false,
    }
}

#[test]
fn palette_ranks_titles_aliases_and_fuzzy_matches() {
    let results = command_palette_results("sam brow", context(), &[]);

    assert_eq!(results[0].action.id, "view.sample-browser");
}

#[test]
fn palette_includes_disabled_explanations() {
    let results = command_palette_results("stop", context(), &[]);

    assert_eq!(results[0].action.id, "stop");
    assert_eq!(results[0].disabled_reason, Some("Playback is stopped"));
}

#[test]
fn palette_boosts_recent_actions_for_empty_queries() {
    let recent = vec!["view.sampler".to_string()];
    let results = command_palette_results("", context(), &recent);

    assert_eq!(results[0].action.id, "view.sampler");
}
