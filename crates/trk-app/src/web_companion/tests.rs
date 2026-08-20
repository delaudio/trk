use std::{
    cell::Cell,
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    sync::{mpsc, Arc, RwLock},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use trk_core::{AutomationTarget, NoteEvent};

use super::{
    finite_meter,
    page::COMPANION_HTML,
    same_action_surface,
    server::{server_address, start_test_server},
    WebAction, WebActionRequest, WebCompanion,
};
use crate::{app_mode::AppMode, config::AppConfig, App};

#[test]
fn snapshot_projects_music_state_without_local_paths() {
    let mut app = App::default();
    app.song.metadata.title = "Bridge test".to_string();
    app.song.patterns[0]
        .set_note(3, 1, NoteEvent::Note { pitch: 64 }, 96)
        .expect("note");
    app.pattern_index = 0;
    app.cursor.row = 3;
    app.project_path = Some(PathBuf::from("/secret/session/project.trk"));

    let state = app.web_bridge_state();
    let json = serde_json::to_string(&state).expect("state json");

    assert!(json.contains("Bridge test"));
    assert!(json.contains("\"pitch\":64"));
    assert!(json.contains("\"velocity\":96"));
    assert!(!json.contains("/secret"));
    assert!(!json.contains("project.trk"));
    assert_eq!(state.tracks[1].active_note.map(|note| note.pitch), Some(64));

    app.song.patterns[0]
        .set_note_event(5, 1, NoteEvent::NoteOff, None)
        .expect("note off");
    app.cursor.row = 5;
    assert_eq!(app.web_bridge_state().tracks[1].active_note, None);
}

#[test]
fn snapshot_clamps_non_finite_or_out_of_range_meter_values() {
    assert_eq!(finite_meter(f32::NAN), 0.0);
    assert_eq!(finite_meter(f32::INFINITY), 0.0);
    assert_eq!(finite_meter(-0.5), 0.0);
    assert_eq!(finite_meter(1.5), 1.0);
}

#[test]
fn publishing_builds_snapshots_only_at_the_browser_poll_rate() {
    let app = App::default();
    let initial_state = app.web_bridge_state();
    let mut companion = WebCompanion::start(initial_state).expect("companion");
    let builds = Cell::new(0);

    assert!(!companion.publish_if_due(|| {
        builds.set(builds.get() + 1);
        app.web_bridge_state()
    }));
    assert_eq!(builds.get(), 0);

    std::thread::sleep(super::PUBLISH_INTERVAL + std::time::Duration::from_millis(5));
    assert!(companion.publish_if_due(|| {
        builds.set(builds.get() + 1);
        app.web_bridge_state()
    }));
    assert_eq!(builds.get(), 1);
}

#[test]
fn queued_actions_require_the_published_edit_surface_and_revision() {
    let mut app = App::default();
    let published = app.web_bridge_state();
    let current = app.web_bridge_state();
    assert!(same_action_surface(&published, &current));

    app.song
        .current_pattern_mut()
        .expect("pattern")
        .set_note(4, 0, NoteEvent::Note { pitch: 64 }, 100)
        .expect("local edit");
    assert!(!same_action_surface(&published, &app.web_bridge_state()));

    app.apply_web_action(
        WebAction::ToggleTrackMute {
            revision: 7,
            index: 0,
        },
        8,
    );
    assert!(!app.song.tracks[0].muted);
}

#[test]
fn normal_lowercase_b_requests_companion_without_taking_other_modes() {
    let mut app = App::default();
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
    assert!(app.take_web_companion_request());

    app.handle_key(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT));
    assert!(!app.take_web_companion_request());
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::SHIFT));
    assert!(!app.take_web_companion_request());

    app.mode = AppMode::Edit;
    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
    assert!(!app.take_web_companion_request());
}

#[test]
fn configured_normal_binding_keeps_precedence_over_builtin_b() {
    let mut config = AppConfig::default();
    config
        .keymap
        .normal
        .insert("b".to_string(), "patterns".to_string());
    let mut app = App::new(config);

    app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));

    assert_eq!(app.mode, AppMode::Patterns);
    assert!(!app.take_web_companion_request());
}

#[test]
fn served_page_is_self_contained_and_has_canvas_controls() {
    assert!(COMPANION_HTML.contains("<canvas id=\"visual\""));
    assert!(COMPANION_HTML.contains("/api/state"));
    assert!(COMPANION_HTML.contains("/api/action"));
    assert!(!COMPANION_HTML.contains("<script src="));
    assert!(!COMPANION_HTML.contains("<link rel=\"stylesheet\""));
    assert!(!COMPANION_HTML.contains("https://"));
    assert!(COMPANION_HTML.contains("async function poll(){if(inflight)return;"));
    assert!(COMPANION_HTML.contains("signature!==patternSignature"));
    assert!(COMPANION_HTML.contains("signature!==trackSignature"));
    assert!(COMPANION_HTML.contains("pattern:d.pattern"));
    assert!(COMPANION_HTML.contains("d.revision"));
}

#[test]
fn move_note_payload_accepts_browser_camel_case_coordinates() {
    let request: WebActionRequest = serde_json::from_str(
        r#"{"type":"moveNote","revision":7,"pattern":0,"row":4,"track":1,"toRow":8,"sourcePitch":60,"pitch":64}"#,
    )
    .expect("browser move payload");

    assert_eq!(
        request.into_action(),
        WebAction::MoveNote {
            revision: 7,
            pattern: 0,
            row: 4,
            track: 1,
            to_row: 8,
            source_pitch: 60,
            pitch: 64,
        }
    );
}

#[test]
fn loopback_smoke_serves_document_state_and_queues_action() {
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, action_rx) = mpsc::sync_channel(4);
    let server = start_test_server(state, action_tx, 0, 1).expect("server");
    let authority = server_address(server.url()).to_string();

    let page = send_request(
        server.url(),
        &format!("GET / HTTP/1.1\r\nHost: {authority}\r\n\r\n"),
    );
    assert!(page.starts_with("HTTP/1.1 200 OK"));
    assert!(page.contains("Content-Security-Policy:"));
    assert!(page.contains("trk companion"));

    let state_response = send_request(
        server.url(),
        &format!("GET /api/state HTTP/1.1\r\nHost: {authority}\r\n\r\n"),
    );
    assert!(state_response.starts_with("HTTP/1.1 200 OK"));
    assert!(state_response.contains("\"version\":2"));

    let body = r#"{"type":"togglePlayback","revision":0}"#;
    let action_response = send_request(
        server.url(),
        &action_request(&authority, body, Some(&format!("http://{authority}"))),
    );
    assert!(action_response.starts_with("HTTP/1.1 202 Accepted"));
    assert_eq!(
        action_rx.try_recv(),
        Ok(WebAction::TogglePlayback { revision: 0 })
    );
}

#[test]
fn actions_require_same_origin_marker_strict_json_and_valid_targets() {
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, _action_rx) = mpsc::sync_channel(4);
    let server = start_test_server(state, action_tx, 0, 1).expect("server");
    let authority = server_address(server.url()).to_string();

    let missing_origin = send_request(
        server.url(),
        &action_request(&authority, r#"{"type":"stop","revision":0}"#, None),
    );
    assert!(
        missing_origin.starts_with("HTTP/1.1 403 Forbidden"),
        "unexpected missing-origin response: {missing_origin:?}"
    );

    let cross_origin = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"stop","revision":0}"#,
            Some("https://example.test"),
        ),
    );
    assert!(cross_origin.starts_with("HTTP/1.1 403 Forbidden"));

    let unknown_field = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"stop","revision":0,"extra":true}"#,
            Some(&format!("http://{authority}")),
        ),
    );
    assert!(unknown_field.starts_with("HTTP/1.1 400 Bad Request"));

    let known_but_extraneous_field = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"stop","revision":0,"index":0}"#,
            Some(&format!("http://{authority}")),
        ),
    );
    assert!(known_but_extraneous_field.starts_with("HTTP/1.1 400 Bad Request"));

    let out_of_range = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"toggleTrackMute","revision":0,"index":99}"#,
            Some(&format!("http://{authority}")),
        ),
    );
    assert!(out_of_range.starts_with("HTTP/1.1 422 Unprocessable Content"));

    let invalid_velocity = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"setNoteVelocity","revision":0,"pattern":0,"row":0,"track":0,"pitch":60,"velocity":255}"#,
            Some(&format!("http://{authority}")),
        ),
    );
    assert!(invalid_velocity.starts_with("HTTP/1.1 422 Unprocessable Content"));
}

#[test]
fn web_actions_reject_stale_revisions() {
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, _action_rx) = mpsc::sync_channel(4);
    let server = start_test_server(state, action_tx, 0, 1).expect("server");
    let authority = server_address(server.url()).to_string();
    let response = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"stop","revision":99}"#,
            Some(&format!("http://{authority}")),
        ),
    );

    assert!(
        response.starts_with("HTTP/1.1 409 Conflict"),
        "unexpected stale-action response: {response:?}"
    );
}

#[test]
fn http_boundary_rejects_oversized_bodies_bad_hosts_and_unsupported_methods() {
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, _action_rx) = mpsc::sync_channel(4);
    let server = start_test_server(state, action_tx, 0, 1).expect("server");
    let authority = server_address(server.url()).to_string();

    let oversized = send_request(
        server.url(),
        &format!("POST /api/action HTTP/1.1\r\nHost: {authority}\r\nContent-Length: 4097\r\n\r\n"),
    );
    assert!(
        oversized.starts_with("HTTP/1.1 413 Payload Too Large"),
        "unexpected oversized-request response: {oversized:?}"
    );

    let bad_host = send_request(
        server.url(),
        "GET /api/state HTTP/1.1\r\nHost: example.test\r\n\r\n",
    );
    assert!(bad_host.starts_with("HTTP/1.1 400 Bad Request"));

    let unsupported = send_request(
        server.url(),
        &format!("DELETE /api/action HTTP/1.1\r\nHost: {authority}\r\n\r\n"),
    );
    assert!(unsupported.starts_with("HTTP/1.1 405 Method Not Allowed"));
    assert!(unsupported.contains("\r\nAllow: POST\r\n"));

    let unsupported = send_request(
        server.url(),
        &format!("POST /api/state HTTP/1.1\r\nHost: {authority}\r\n\r\n"),
    );
    assert!(unsupported.starts_with("HTTP/1.1 405 Method Not Allowed"));
    assert!(unsupported.contains("\r\nAllow: GET\r\n"));
}

#[test]
fn accepted_actions_use_existing_app_mutation_paths() {
    let mut app = App::default();

    app.apply_web_action(
        WebAction::ToggleTrackMute {
            revision: 0,
            index: 0,
        },
        0,
    );
    app.apply_web_action(
        WebAction::ToggleTrackSolo {
            revision: 0,
            index: 1,
        },
        0,
    );

    assert!(app.song.tracks[0].muted);
    assert!(app.song.tracks[1].solo);
    assert!(app.dirty);

    app.apply_web_action(
        WebAction::SetCcPoint {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            controller: 74,
            value: 96,
        },
        0,
    );
    assert_eq!(app.song.patterns[0].automation.len(), 1);
    app.apply_web_action(
        WebAction::ClearCcPoint {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            controller: 74,
        },
        0,
    );
    assert!(app.song.patterns[0].automation.is_empty());
    app.undo();
    assert_eq!(app.song.patterns[0].automation.len(), 1);
}

#[test]
fn note_gate_velocity_and_cc_actions_update_the_shared_song_model() {
    let mut app = App::default();
    app.apply_web_action(
        WebAction::CreateNote {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            pitch: 64,
        },
        0,
    );
    app.apply_web_action(
        WebAction::ResizeNote {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            pitch: 64,
            gate: 6,
        },
        0,
    );
    app.apply_web_action(
        WebAction::SetNoteVelocity {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            pitch: 64,
            velocity: 96,
        },
        0,
    );
    app.apply_web_action(
        WebAction::SetCcPoint {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            controller: 74,
            value: 64,
        },
        0,
    );

    app.apply_web_action(
        WebAction::CreateNote {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            pitch: 65,
        },
        0,
    );
    app.apply_web_action(
        WebAction::ResizeNote {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            pitch: 65,
            gate: 2,
        },
        0,
    );
    app.apply_web_action(
        WebAction::SetNoteVelocity {
            revision: 0,
            pattern: 0,
            row: 4,
            track: 0,
            pitch: 65,
            velocity: 32,
        },
        0,
    );

    let pattern = &app.song.patterns[0];
    let cell = pattern.cell(4, 0).expect("note");
    assert_eq!(cell.note, Some(NoteEvent::Note { pitch: 64 }));
    assert_eq!(cell.gate, Some(6));
    assert_eq!(cell.velocity, Some(96));
    assert_eq!(
        pattern.automation[0].target,
        AutomationTarget::MidiCc {
            track: app.song.tracks[0].id,
            controller: 74,
        }
    );
    assert!(app.dirty);
}

#[test]
fn full_action_queue_returns_retryable_response() {
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, _action_rx) = mpsc::sync_channel(1);
    let server = start_test_server(state, action_tx, 0, 1).expect("server");
    let authority = server_address(server.url()).to_string();
    let request = action_request(
        &authority,
        r#"{"type":"stop","revision":0}"#,
        Some(&format!("http://{authority}")),
    );

    let accepted = send_request(server.url(), &request);
    assert!(
        accepted.starts_with("HTTP/1.1 202 Accepted"),
        "unexpected first queued-action response: {accepted:?}"
    );
    let full = send_request(server.url(), &request);
    assert!(
        full.starts_with("HTTP/1.1 503 Service Unavailable"),
        "unexpected full-queue response: {full:?}"
    );
}

#[test]
fn binding_skips_an_occupied_starting_port_and_drop_stops_server() {
    let guard = (40_000..60_000)
        .find_map(|port| {
            let first = TcpListener::bind(("127.0.0.1", port)).ok()?;
            let second = TcpListener::bind(("127.0.0.1", port + 1)).ok()?;
            drop(second);
            Some(first)
        })
        .expect("two consecutive test ports");
    let first_port = guard.local_addr().expect("guard address").port();
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, _action_rx) = mpsc::sync_channel(1);
    let server = start_test_server(state, action_tx, first_port, 2).expect("fallback server");
    let address = server_address(server.url());
    assert_eq!(address.port(), first_port + 1);
    drop(server);
    assert!(TcpStream::connect(address).is_err());
}

fn action_request(authority: &str, body: &str, origin: Option<&str>) -> String {
    let origin = origin.map_or_else(String::new, |origin| format!("Origin: {origin}\r\n"));
    format!(
        "POST /api/action HTTP/1.1\r\nHost: {authority}\r\n{origin}Content-Type: application/json\r\nX-Trk-Request: 1\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn send_request(url: &str, request: &str) -> String {
    let address = server_address(url);
    let mut stream = TcpStream::connect(address).expect("connect");
    stream.write_all(request.as_bytes()).expect("write request");
    if let Err(error) = stream.shutdown(Shutdown::Write) {
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::NotConnected,
            "shutdown write: {error}"
        );
    }
    let mut response = Vec::new();
    if let Err(error) = stream.read_to_end(&mut response) {
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset,
            "read response: {error}"
        );
    }
    String::from_utf8(response).expect("UTF-8 response")
}
