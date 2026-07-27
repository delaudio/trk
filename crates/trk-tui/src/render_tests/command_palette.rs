use super::render_test_support::render_test_state;
use super::*;
use ratatui::{backend::TestBackend, Terminal};
use trk_core::Song;

#[test]
fn command_palette_entries_expose_absolute_scrolled_indices_on_fixed_rows() {
    let entry = CommandPaletteEntryView {
        title: "A deliberately long command palette action title",
        category: "View",
        command: "long-command",
        shortcut: Some("Ctrl+Shift+P"),
        disabled_reason: None,
        recent: false,
    };
    let entries = [entry; 20];
    let song = Song::empty();
    let backend = TestBackend::new(48, 20);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            let mut state = render_test_state();
            state.mode_label = "PALETTE";
            state.command_palette = Some(CommandPaletteViewState {
                query: "",
                entries: &entries,
                selected: 15,
            });
            interactions = render_with_interactions(frame, &song, state);
        })
        .expect("draw");

    let rows = interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::COMMAND_PALETTE_ENTRY)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 14);
    assert_eq!(
        rows.first().map(|region| region.payload),
        Some(crate::InteractionPayload::CommandPaletteEntry { index: 2 })
    );
    assert_eq!(
        rows.last().map(|region| region.payload),
        Some(crate::InteractionPayload::CommandPaletteEntry { index: 15 })
    );
    assert!(rows
        .windows(2)
        .all(|pair| pair[1].area.y == pair[0].area.y + 1));
}

#[test]
fn command_palette_non_entry_geometry_has_no_entry_payload() {
    let entry = CommandPaletteEntryView {
        title: "Open Sampler",
        category: "View",
        command: "sampler",
        shortcut: None,
        disabled_reason: None,
        recent: false,
    };
    let entries = [entry];
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut interactions = InteractionMap::new();

    terminal
        .draw(|frame| {
            let mut state = render_test_state();
            state.mode_label = "PALETTE";
            state.command_palette = Some(CommandPaletteViewState {
                query: "",
                entries: &entries,
                selected: 0,
            });
            interactions = render_with_interactions(frame, &song, state);
        })
        .expect("draw");

    let row = interactions
        .region(interaction_region::COMMAND_PALETTE_ENTRY)
        .expect("entry");
    assert_ne!(
        row.area.y,
        interactions
            .region(interaction_region::OVERLAY_COMMAND_PALETTE)
            .unwrap()
            .area
            .y
    );
    assert!(interactions
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::COMMAND_PALETTE_ENTRY)
        .all(
            |region| region.payload == crate::InteractionPayload::CommandPaletteEntry { index: 0 }
        ));
}
