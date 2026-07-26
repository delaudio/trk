use super::render_test_support::render_test_state;
use super::*;
use crate::{InteractionPayload, MidiSettingsAction};
use ratatui::{backend::TestBackend, Terminal};

fn midi_interactions(ports: &[MidiPortView<'_>], selected: usize) -> InteractionMap {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut map = InteractionMap::new();
    terminal
        .draw(|frame| {
            let mut state = render_test_state();
            state.midi_settings = Some(MidiSettingsState {
                ports,
                selected_port: selected,
                status: "MIDI Disconnected",
                input_status: "MIDI In Disconnected",
                routing: &song.midi,
            });
            map = render_with_interactions(frame, &song, state);
        })
        .expect("draw");
    map
}

#[test]
fn midi_settings_virtualizes_multiple_port_rows_with_absolute_indices() {
    let ports = [MidiPortView {
        index: 7,
        name: "External MIDI Port",
    }; 20];
    let map = midi_interactions(&ports, 15);
    let rows = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::MIDI_SETTINGS_PORT)
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), 10);
    assert_eq!(
        rows.first().map(|region| region.payload),
        Some(InteractionPayload::MidiPortRow { index: 6 })
    );
    assert_eq!(
        rows.last().map(|region| region.payload),
        Some(InteractionPayload::MidiPortRow { index: 15 })
    );
    assert!(rows
        .windows(2)
        .all(|pair| pair[1].area.y == pair[0].area.y.saturating_add(1)));
}

#[test]
fn midi_settings_actions_are_distinct_and_exist_with_empty_ports() {
    let map = midi_interactions(&[], 0);
    assert!(map
        .regions()
        .iter()
        .all(|region| region.id != interaction_region::MIDI_SETTINGS_PORT));

    let actions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::MIDI_SETTINGS_ACTION)
        .collect::<Vec<_>>();
    let expected = [
        MidiSettingsAction::Connect,
        MidiSettingsAction::Disconnect,
        MidiSettingsAction::Panic,
        MidiSettingsAction::Refresh,
        MidiSettingsAction::Close,
    ];
    assert_eq!(actions.len(), expected.len());
    for (region, action) in actions.iter().zip(expected) {
        assert_eq!(
            region.payload,
            InteractionPayload::MidiSettingsAction { action }
        );
        assert_eq!(region.area.height, 1);
    }
    assert!(actions
        .windows(2)
        .all(|pair| { pair[0].area.x.saturating_add(pair[0].area.width) < pair[1].area.x }));
}
