use super::*;

fn viewport() -> MouseViewport {
    MouseViewport {
        terminal_width: 100,
        terminal_height: 32,
    }
}

fn register_action(app: &mut App, action: ConfirmationAction) {
    app.interaction_map.register_with_payload(
        interaction_region::CONFIRMATION_ACTION,
        ratatui::layout::Rect::new(30, 15, 14, 1),
        InteractionPayload::ConfirmationAction { action },
    );
}

fn click(app: &mut App, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_mouse(
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        viewport(),
    );
}

#[test]
fn quit_save_and_dont_save_clicks_use_existing_dialog_paths() {
    let path =
        std::env::temp_dir().join(format!("trk-mouse-dialog-save-{}.trk", std::process::id()));
    let mut save = App {
        project_path: Some(path.clone()),
        dirty: true,
        mode: AppMode::Dialog,
        dialog: Some(Dialog::QuitDirty),
        ..App::default()
    };
    register_action(&mut save, ConfirmationAction::Save);
    click(&mut save, MouseEventKind::Down(MouseButton::Left), 32, 15);
    let saved = load_project(&path).expect("clicked Save writes project");
    let _ = std::fs::remove_file(path);
    assert_eq!(saved, save.song);
    assert!(save.should_quit);

    let mut dont_save = App {
        dirty: true,
        mode: AppMode::Dialog,
        dialog: Some(Dialog::QuitDirty),
        ..App::default()
    };
    register_action(&mut dont_save, ConfirmationAction::DontSave);
    click(
        &mut dont_save,
        MouseEventKind::Down(MouseButton::Left),
        32,
        15,
    );
    assert!(dont_save.should_quit);
    assert!(dont_save.dialog.is_none());
}

#[test]
fn destructive_confirm_and_cancel_clicks_use_existing_dialog_paths() {
    let mut confirm = App {
        mode: AppMode::Dialog,
        dialog: Some(Dialog::DeleteTrack {
            track_index: 1,
            message: "Delete track 02 Bass?".to_string(),
        }),
        ..App::default()
    };
    let original_tracks = confirm.song.tracks.len();
    register_action(&mut confirm, ConfirmationAction::Confirm);
    click(
        &mut confirm,
        MouseEventKind::Down(MouseButton::Left),
        32,
        15,
    );
    assert_eq!(confirm.song.tracks.len(), original_tracks - 1);
    assert_eq!(confirm.mode, AppMode::Normal);

    let mut cancel = App {
        mode: AppMode::Dialog,
        dialog: Some(Dialog::DeleteTrack {
            track_index: 1,
            message: "Delete track 02 Bass?".to_string(),
        }),
        ..App::default()
    };
    register_action(&mut cancel, ConfirmationAction::Cancel);
    click(&mut cancel, MouseEventKind::Down(MouseButton::Left), 32, 15);
    assert_eq!(cancel.song.tracks.len(), original_tracks);
    assert!(cancel.dialog.is_none());
    assert_eq!(cancel.mode, AppMode::Normal);
}

#[test]
fn open_project_confirm_and_cancel_clicks_use_existing_dialog_paths() {
    let dir = std::env::temp_dir().join(format!("trk-mouse-dialog-open-{}", std::process::id()));
    let project_path = dir.join("target.trk");
    std::fs::create_dir_all(&dir).expect("create project dir");
    let mut target = Song::empty();
    target.transport.bpm = 155;
    save_song_project(&project_path, &target).expect("save target project");

    let mut confirm = App {
        mode: AppMode::Dialog,
        dialog: Some(Dialog::OpenProjectDirty {
            path: project_path.clone(),
            message: "Discard changes and open target?".to_string(),
        }),
        ..App::default()
    };
    register_action(&mut confirm, ConfirmationAction::Confirm);
    click(
        &mut confirm,
        MouseEventKind::Down(MouseButton::Left),
        32,
        15,
    );
    assert_eq!(confirm.mode, AppMode::Normal);
    assert_eq!(confirm.song.transport.bpm, 155);

    let mut cancel = App {
        mode: AppMode::Dialog,
        dialog: Some(Dialog::OpenProjectDirty {
            path: project_path,
            message: "Discard changes and open target?".to_string(),
        }),
        project_browser_view: Some(AppProjectBrowserView {
            current_dir: dir.clone(),
            entries: Vec::new(),
            cursor: 0,
            message: None,
        }),
        ..App::default()
    };
    register_action(&mut cancel, ConfirmationAction::Cancel);
    click(&mut cancel, MouseEventKind::Down(MouseButton::Left), 32, 15);
    assert_eq!(cancel.mode, AppMode::ProjectBrowser);
    assert!(cancel.dialog.is_none());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn confirmation_dialog_ignores_outside_secondary_drag_and_invalid_targets() {
    let mut app = App {
        mode: AppMode::Dialog,
        dialog: Some(Dialog::QuitDirty),
        ..App::default()
    };
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 1);
    assert_eq!(app.mode, AppMode::Dialog);
    assert!(!app.should_quit);

    for kind in [
        MouseEventKind::Down(MouseButton::Right),
        MouseEventKind::Drag(MouseButton::Left),
    ] {
        register_action(&mut app, ConfirmationAction::DontSave);
        click(&mut app, kind, 32, 15);
        assert_eq!(app.mode, AppMode::Dialog);
        assert!(!app.should_quit);
    }

    let mut mismatched = App {
        mode: AppMode::Dialog,
        dialog: Some(Dialog::DeleteTrack {
            track_index: 1,
            message: "Delete track 02 Bass?".to_string(),
        }),
        ..App::default()
    };
    let original_tracks = mismatched.song.tracks.len();
    register_action(&mut mismatched, ConfirmationAction::Save);
    click(
        &mut mismatched,
        MouseEventKind::Down(MouseButton::Left),
        32,
        15,
    );
    assert_eq!(mismatched.song.tracks.len(), original_tracks);
    assert!(matches!(
        mismatched.dialog,
        Some(Dialog::DeleteTrack { .. })
    ));

    app.interaction_map.register_with_payload(
        interaction_region::CONFIRMATION_ACTION,
        ratatui::layout::Rect::new(30, 15, 14, 1),
        InteractionPayload::None,
    );
    click(&mut app, MouseEventKind::Down(MouseButton::Left), 32, 15);
    assert_eq!(app.mode, AppMode::Dialog);
    assert!(!app.should_quit);
}
