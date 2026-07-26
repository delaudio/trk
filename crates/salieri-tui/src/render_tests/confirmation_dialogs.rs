use super::render_test_support::render_test_state;
use super::*;
use crate::{ConfirmationAction, InteractionPayload};
use ratatui::{backend::TestBackend, Terminal};

fn confirmation_interactions(quit: bool, message: Option<&str>) -> InteractionMap {
    let song = Song::empty();
    let backend = TestBackend::new(100, 32);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut map = InteractionMap::new();
    terminal
        .draw(|frame| {
            let mut state = render_test_state();
            state.quit_confirmation = quit;
            state.delete_confirmation = message;
            map = render_with_interactions(frame, &song, state);
        })
        .expect("draw");
    map
}

#[test]
fn quit_confirmation_exposes_save_dont_save_and_cancel_targets() {
    let map = confirmation_interactions(true, None);
    let actions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::CONFIRMATION_ACTION)
        .collect::<Vec<_>>();
    let expected = [
        ConfirmationAction::Save,
        ConfirmationAction::DontSave,
        ConfirmationAction::Cancel,
    ];
    assert_eq!(actions.len(), expected.len());
    for (region, action) in actions.iter().zip(expected) {
        assert_eq!(
            region.payload,
            InteractionPayload::ConfirmationAction { action }
        );
        assert_eq!(region.area.height, 1);
    }
    assert!(actions
        .windows(2)
        .all(|pair| { pair[0].area.x.saturating_add(pair[0].area.width) < pair[1].area.x }));
}

#[test]
fn destructive_confirmation_exposes_confirm_and_cancel_targets() {
    let map = confirmation_interactions(false, Some("Delete track 02 Bass?"));
    let actions = map
        .regions()
        .iter()
        .filter(|region| region.id == interaction_region::CONFIRMATION_ACTION)
        .collect::<Vec<_>>();

    assert_eq!(actions.len(), 2);
    assert_eq!(
        actions[0].payload,
        InteractionPayload::ConfirmationAction {
            action: ConfirmationAction::Confirm
        }
    );
    assert_eq!(
        actions[1].payload,
        InteractionPayload::ConfirmationAction {
            action: ConfirmationAction::Cancel
        }
    );
}
