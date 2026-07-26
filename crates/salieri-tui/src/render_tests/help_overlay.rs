use super::render_test_support::terminal_buffer_text;
use super::*;
use crate::InteractionPayload;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;

fn interaction_map_with_state(width: u16, height: u16, state: TuiState<'_>) -> InteractionMap {
    let song = Song::empty();
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut map = InteractionMap::new();
    terminal
        .draw(|frame| map = render_with_interactions(frame, &song, state))
        .expect("draw");
    map
}

#[test]
fn help_overlay_exposes_fixed_tabs_content_and_close_targets() {
    let mut state = render_test_support::render_test_state();
    state.show_help = true;
    state.help_tab = HelpTab::Sampler;

    let map = interaction_map_with_state(100, 32, state);
    let tabs = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::HELP_TAB)
        .collect::<Vec<_>>();

    assert_eq!(tabs.len(), HelpTab::ALL.len());
    for (index, region) in tabs.iter().enumerate() {
        assert_eq!(region.area.height, 1);
        assert_eq!(region.payload, InteractionPayload::HelpTab { index });
    }
    assert!(tabs.windows(2).all(|pair| pair[0].area.y == pair[1].area.y
        && pair[0].area.x.saturating_add(pair[0].area.width) < pair[1].area.x));

    let content = map
        .region(interaction_region::HELP_CONTENT)
        .expect("help content");
    let close = map
        .region(interaction_region::HELP_CLOSE)
        .expect("help close");
    assert!(content.area.y > tabs[0].area.y);
    assert_eq!(close.area.height, 1);
    assert_eq!(
        map.hit_test(close.area.x, close.area.y)
            .map(|region| region.id),
        Some(interaction_region::HELP_CLOSE)
    );
}

#[test]
fn help_overlay_renders_visible_close_control() {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut state = render_test_support::render_test_state();
    state.show_help = true;

    terminal
        .draw(|frame| render(frame, &song, state))
        .expect("draw");

    let rendered = terminal_buffer_text(&terminal);
    assert!(rendered.contains("[ Close ]"));
    assert!(rendered.contains("Basics"));
    assert!(rendered.contains("Commands"));
}
