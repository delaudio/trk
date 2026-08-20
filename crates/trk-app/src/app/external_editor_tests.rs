use super::*;
use crate::persistence::ProjectFile;
use trk_core::{NoteEvent, PlaybackPosition};

use crate::{
    app_event::{AppEvent, RuntimeEvent},
    playback_runtime::{PlaybackCursor, PlaybackUpdate},
};

#[test]
fn lowercase_e_requests_editor_only_from_normal_pattern_mode() {
    let mut normal = App::default();
    normal.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    assert!(normal.external_editor_requested);

    let mut edit = App {
        mode: AppMode::Edit,
        ..App::default()
    };
    edit.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    assert!(!edit.external_editor_requested);
    assert!(edit.song.patterns[0].rows[0].cells[0].note.is_some());
}

#[test]
fn unnamed_project_round_trips_through_scratch_without_becoming_named() {
    let mut app = App {
        external_editor_requested: true,
        ..App::default()
    };
    let request = app.take_external_editor_request().expect("request");
    assert!(request.scratch);
    assert!(request.path.is_file());
    let mut edited = ProjectFile::with_history(app.song.clone(), app.variation_history.clone());
    edited.song.transport.bpm = 173;
    save_project_file(&request.path, &edited).expect("external save");

    app.finish_external_editor(request.clone(), Ok(success_status()));

    assert_eq!(app.song.transport.bpm, 173);
    assert!(app.project_path.is_none());
    assert!(app.dirty);
    assert!(!request.path.exists());
}

#[test]
fn invalid_scratch_is_preserved_without_replacing_live_state() {
    let mut app = App {
        external_editor_requested: true,
        ..App::default()
    };
    let before = app.song.clone();
    let request = app.take_external_editor_request().expect("request");
    fs::write(&request.path, "{not json").expect("invalid edit");

    app.finish_external_editor(request.clone(), Ok(success_status()));

    assert_eq!(app.song, before);
    assert!(request.path.exists());
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("scratch kept at")));
    assert!(remove_scratch(&request.path, request.scratch_directory.as_deref()).is_empty());
}

#[test]
fn launch_failure_removes_unedited_scratch_and_nonzero_exit_preserves_it() {
    let mut failed_launch = App {
        external_editor_requested: true,
        ..App::default()
    };
    let failed_request = failed_launch
        .take_external_editor_request()
        .expect("launch request");
    failed_launch.finish_external_editor(
        failed_request.clone(),
        Err(ExternalEditorRunError::Launch("not found".to_string())),
    );
    assert!(!failed_request.path.exists());

    let mut failed_wait = App {
        external_editor_requested: true,
        ..App::default()
    };
    let waited_request = failed_wait
        .take_external_editor_request()
        .expect("wait request");
    failed_wait.finish_external_editor(
        waited_request.clone(),
        Err(ExternalEditorRunError::Wait("wait failed".to_string())),
    );
    assert!(waited_request.path.exists());
    assert!(remove_scratch(
        &waited_request.path,
        waited_request.scratch_directory.as_deref()
    )
    .is_empty());

    let mut failed_exit = App {
        external_editor_requested: true,
        ..App::default()
    };
    let exited_request = failed_exit
        .take_external_editor_request()
        .expect("exit request");
    failed_exit.finish_external_editor(exited_request.clone(), Ok(failure_status()));
    assert!(exited_request.path.exists());
    assert!(remove_scratch(
        &exited_request.path,
        exited_request.scratch_directory.as_deref()
    )
    .is_empty());
}

#[test]
fn terminal_reentry_failure_preserves_scratch_for_recovery() {
    let mut app = App {
        external_editor_requested: true,
        ..App::default()
    };
    let request = app.take_external_editor_request().expect("request");

    app.finish_external_editor_terminal_failure(&request, "could not restore terminal");

    assert!(request.path.exists());
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("scratch kept at")));
    assert!(remove_scratch(&request.path, request.scratch_directory.as_deref()).is_empty());
}

#[test]
fn scratch_adoption_refuses_concurrent_local_changes() {
    let mut app = App {
        external_editor_requested: true,
        ..App::default()
    };
    let request = app.take_external_editor_request().expect("request");
    let mut edited = ProjectFile::with_history(app.song.clone(), app.variation_history.clone());
    edited.song.transport.bpm = 181;
    save_project_file(&request.path, &edited).expect("external edit");
    app.song.transport.bpm = 140;
    app.refresh_dirty();

    app.finish_external_editor(request.clone(), Ok(success_status()));

    assert_eq!(app.song.transport.bpm, 140);
    assert!(request.path.exists());
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("conflicts")));
    assert!(remove_scratch(&request.path, request.scratch_directory.as_deref()).is_empty());
}

#[test]
fn no_op_scratch_editor_preserves_undo_and_live_state() {
    let mut app = App::default();
    app.mutate_song(|song, _| song.transport.bpm = 140);
    let before_song = app.song.clone();
    let before_undo = app.history.undo_len();
    app.external_editor_requested = true;
    let request = app.take_external_editor_request().expect("request");

    app.finish_external_editor(request.clone(), Ok(success_status()));

    assert_eq!(app.song, before_song);
    assert_eq!(app.history.undo_len(), before_undo);
    assert!(!request.path.exists());
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("no project changes")));
}

#[test]
fn cleanup_accepts_an_already_removed_scratch_file() {
    let path = test_project_path("missing-scratch");
    assert!(remove_scratch(&path, None).is_empty());
}

#[test]
fn scratch_creation_never_overwrites_an_existing_path() {
    let path = test_project_path("exclusive-scratch");
    fs::write(&path, "sentinel").expect("existing file");

    let error = write_new_scratch(&path, b"replacement").expect_err("exclusive creation");

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read_to_string(&path).expect("sentinel remains"),
        "sentinel"
    );
    fs::remove_file(path).expect("cleanup existing file");
}

#[test]
fn scratch_cleanup_checks_owned_directory_before_deleting() {
    let path = test_project_path("ownership-check");
    let unrelated = test_project_path("unrelated-owner");
    fs::write(&path, "keep").expect("owned file");

    let warning = remove_scratch(&path, Some(&unrelated));

    assert!(warning.contains("mismatched owner"));
    assert_eq!(fs::read_to_string(&path).expect("file remains"), "keep");
    fs::remove_file(path).expect("cleanup file");
}

#[cfg(unix)]
#[test]
fn scratch_creation_denies_group_and_world_access() {
    use std::os::unix::fs::PermissionsExt;

    let project = ProjectFile::new(Song::empty());
    let scratch = create_scratch_project(&project).expect("scratch project");
    let path = scratch.path;
    let mode = fs::metadata(&path)
        .expect("scratch metadata")
        .permissions()
        .mode();
    let directory_mode = fs::metadata(path.parent().expect("scratch directory"))
        .expect("scratch directory metadata")
        .permissions()
        .mode();

    assert_eq!(mode & 0o077, 0);
    assert_eq!(directory_mode & 0o077, 0);
    assert!(remove_scratch(&path, Some(&scratch.directory)).is_empty());
}

#[test]
fn dirty_named_editor_uses_scratch_and_preserves_active_transport() {
    let path = test_project_path("named-editor");
    let original = ProjectFile::new(Song::empty());
    save_project_file(&path, &original).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.is_playing = true;
    app.playhead_row = Some(7);
    app.song.patterns[0]
        .set_note(0, 0, NoteEvent::Note { pitch: 60 }, 100)
        .expect("local edit");
    app.refresh_dirty();
    app.external_editor_requested = true;
    let request = app.take_external_editor_request().expect("request");
    assert!(request.scratch);
    assert_ne!(request.path, path);
    assert!(app.dirty);
    let mut edited = ProjectFile::with_history(app.song.clone(), app.variation_history.clone());
    edited.song.transport.bpm = 181;
    save_project_file(&request.path, &edited).expect("external edit");

    app.finish_external_editor(request, Ok(success_status()));

    assert_eq!(app.song.transport.bpm, 181);
    assert!(app.is_playing);
    assert_eq!(app.playhead_row, Some(7));
    assert!(app.dirty);
    assert_eq!(app.project_path.as_deref(), Some(path.as_path()));
    assert_eq!(
        load_project_file(&path)
            .expect("unchanged named project")
            .song
            .transport
            .bpm,
        original.song.transport.bpm
    );
    fs::remove_file(path).expect("cleanup project");
}

#[test]
fn clean_named_editor_uses_active_path_and_adopts_a_clean_baseline() {
    let path = test_project_path("clean-named-editor");
    save_project_file(&path, &ProjectFile::new(Song::empty())).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.external_editor_requested = true;
    let request = app.take_external_editor_request().expect("request");
    assert!(!request.scratch);
    assert_eq!(request.path, path);

    let mut edited = ProjectFile::with_history(app.song.clone(), app.variation_history.clone());
    edited.song.transport.bpm = 181;
    save_project_file(&path, &edited).expect("external edit");
    app.finish_external_editor(request, Ok(success_status()));

    assert_eq!(app.song.transport.bpm, 181);
    assert!(!app.dirty);
    fs::remove_file(path).expect("cleanup project");
}

#[test]
fn clean_named_editor_refuses_to_overwrite_newer_live_state() {
    let path = test_project_path("clean-named-conflict");
    save_project_file(&path, &ProjectFile::new(Song::empty())).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.external_editor_requested = true;
    let request = app.take_external_editor_request().expect("request");
    let mut edited = ProjectFile::with_history(app.song.clone(), app.variation_history.clone());
    edited.song.transport.bpm = 181;
    save_project_file(&path, &edited).expect("external edit");
    app.song.transport.bpm = 140;
    app.refresh_dirty();

    app.finish_external_editor(request, Ok(success_status()));

    assert_eq!(app.song.transport.bpm, 140);
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("not adopted")));
    fs::remove_file(path).expect("cleanup project");
}

#[test]
fn clean_named_editor_refuses_to_recreate_a_missing_active_file() {
    let path = test_project_path("missing-named-editor");
    let mut app = App {
        project_path: Some(path.clone()),
        external_editor_requested: true,
        ..App::default()
    };

    assert!(app.take_external_editor_request().is_none());
    assert!(!path.exists());
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("missing")));
}

#[cfg(unix)]
#[test]
fn named_editor_and_watcher_reject_symlink_project_paths() {
    use std::os::unix::fs::symlink;

    let target = test_project_path("symlink-target");
    let link = test_project_path("symlink-link");
    save_project_file(&target, &ProjectFile::new(Song::empty())).expect("target project");
    symlink(&target, &link).expect("project symlink");
    let mut app = App {
        project_path: Some(link.clone()),
        external_editor_requested: true,
        ..App::default()
    };

    assert!(app.take_external_editor_request().is_none());
    assert!(matches!(
        observe_project_file(&link),
        ProjectFileObservation::Unreadable(std::io::ErrorKind::InvalidInput)
    ));

    fs::remove_file(link).expect("cleanup symlink");
    fs::remove_file(target).expect("cleanup target");
}

#[cfg(unix)]
#[test]
fn named_editor_rechecks_symlink_replacement_before_adoption() {
    use std::os::unix::fs::symlink;

    let path = test_project_path("editor-symlink-swap");
    let target = test_project_path("editor-symlink-swap-target");
    save_project_file(&path, &ProjectFile::new(Song::empty())).expect("initial project");
    save_project_file(&target, &ProjectFile::new(Song::empty())).expect("target project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.external_editor_requested = true;
    let request = app.take_external_editor_request().expect("request");
    fs::remove_file(&path).expect("remove active file");
    symlink(&target, &path).expect("replace with symlink");

    app.finish_external_editor(request, Ok(success_status()));

    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("invalid")));
    fs::remove_file(path).expect("cleanup symlink");
    fs::remove_file(target).expect("cleanup target");
}

#[test]
fn stale_playback_positions_stay_bounded_after_external_reload() {
    let mut app = App::default();
    app.adopt_external_project(ProjectFile::new(Song::empty()), false);
    app.sequence_cursor = 99;

    app.dispatch_event(AppEvent::Runtime(RuntimeEvent::PlaybackUpdate(
        PlaybackUpdate::Position(PlaybackCursor {
            pattern_index: 99,
            sequence_index: None,
            position: PlaybackPosition {
                row: 99,
                offset_micros: 0,
            },
        }),
    )));

    assert!(app.pattern_index < app.song.patterns.len());
    assert!(app
        .sequence_position
        .is_none_or(|position| position < app.song.sequence.len()));
    assert!(app
        .playhead_row
        .is_some_and(|row| row < app.current_row_count()));
    assert_eq!(app.sequence_cursor, 0);
}

#[test]
fn watcher_requires_a_new_external_write_after_a_dirty_conflict() {
    let path = test_project_path("watch-conflict");
    let original = ProjectFile::new(Song::empty());
    save_project_file(&path, &original).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.song.transport.bpm = 140;
    app.refresh_dirty();
    let mut external = original;
    external.song.transport.bpm = 190;
    save_project_file(&path, &external).expect("external project");
    make_watch_due(&mut app);

    app.poll_project_hot_reload();

    assert_eq!(app.song.transport.bpm, 140);
    assert!(app.dirty);
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("blocked")));

    app.clean_song = app.song.clone();
    app.refresh_dirty();
    make_watch_due(&mut app);
    app.poll_project_hot_reload();

    assert_eq!(app.song.transport.bpm, 140);
    assert!(!app.dirty);

    let mut newer_external = ProjectFile::new(Song::empty());
    newer_external.song.transport.bpm = 191;
    save_project_file(&path, &newer_external).expect("newer external project");
    make_watch_due(&mut app);
    app.poll_project_hot_reload();

    assert_eq!(app.song.transport.bpm, 191);
    fs::remove_file(path).expect("cleanup project");
}

#[test]
fn local_save_cancels_a_blocked_external_reload() {
    let path = test_project_path("watch-save-conflict");
    let original = ProjectFile::new(Song::empty());
    save_project_file(&path, &original).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.song.transport.bpm = 140;
    app.refresh_dirty();

    let mut external = original;
    external.song.transport.bpm = 190;
    save_project_file(&path, &external).expect("external project");
    make_watch_due(&mut app);
    app.poll_project_hot_reload();
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("blocked")));

    let saved = ProjectFile::with_history(app.song.clone(), app.variation_history.clone());
    save_project_file(&path, &saved).expect("local save");
    app.apply_project_save(path.clone(), saved, false, Ok(()));
    make_watch_due(&mut app);
    app.poll_project_hot_reload();

    assert_eq!(app.song.transport.bpm, 140);
    assert!(!app.dirty);
    fs::remove_file(path).expect("cleanup project");
}

#[test]
fn invalid_watched_change_reports_once_and_keeps_undo_state() {
    let path = test_project_path("watch-invalid");
    save_project_file(&path, &ProjectFile::new(Song::empty())).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.mutate_song(|song, _| song.transport.bpm = 121);
    app.clean_song = app.song.clone();
    app.refresh_dirty();
    let undo_len = app.history.undo_len();
    fs::write(&path, "invalid project").expect("invalid external write");
    make_watch_due(&mut app);

    app.poll_project_hot_reload();
    assert_eq!(app.song.transport.bpm, 121);
    assert_eq!(app.history.undo_len(), undo_len);
    assert!(app.notification.is_some());

    app.notification = None;
    make_watch_due(&mut app);
    app.poll_project_hot_reload();
    assert!(app.notification.is_none());

    let mut corrected = ProjectFile::new(Song::empty());
    corrected.song.transport.bpm = 177;
    save_project_file(&path, &corrected).expect("corrected external project");
    make_watch_due(&mut app);
    app.poll_project_hot_reload();
    assert_eq!(app.song.transport.bpm, 177);
    fs::remove_file(path).expect("cleanup project");
}

#[test]
fn internal_save_refreshes_watch_and_missing_file_reports_once() {
    let path = test_project_path("watch-save-missing");
    let project = ProjectFile::new(Song::empty());
    save_project_file(&path, &project).expect("initial project");
    let mut app = App::from_file(&path, AppConfig::default()).expect("load app");
    app.apply_project_save(path.clone(), project, false, Ok(()));
    app.notification = None;
    make_watch_due(&mut app);
    app.poll_project_hot_reload();
    assert!(app.notification.is_none());

    fs::remove_file(&path).expect("remove active project");
    make_watch_due(&mut app);
    app.poll_project_hot_reload();
    assert!(app
        .notification
        .as_ref()
        .is_some_and(|notice| notice.message.contains("missing")));
    app.notification = None;
    make_watch_due(&mut app);
    app.poll_project_hot_reload();
    assert!(app.notification.is_none());
}

#[test]
fn watch_signature_detects_same_length_content_changes() {
    let path = test_project_path("watch-content-hash");
    fs::write(&path, "aaaa").expect("first content");
    let first = observe_project_file(&path);
    fs::write(&path, "bbbb").expect("same-length content");
    let second = observe_project_file(&path);

    assert_ne!(first, second);
    fs::remove_file(path).expect("cleanup project");
}

fn make_watch_due(app: &mut App) {
    let watch = app.project_watch.as_mut().expect("project watch");
    watch.last_poll = Instant::now() - PROJECT_WATCH_INTERVAL;
    watch.last_content_check = Instant::now() - PROJECT_CONTENT_VERIFY_INTERVAL;
}

fn test_project_path(label: &str) -> PathBuf {
    temporary_project_path().with_file_name(format!(
        "trk-{label}-{}-{}.trk",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ))
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(windows)]
fn success_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn failure_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(1)
}
