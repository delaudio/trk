use std::{
    cell::Cell,
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    sync::{mpsc, Arc, RwLock},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use trk_core::NoteEvent;

use super::{
    finite_meter,
    page::COMPANION_HTML,
    server::{server_address, start_test_server},
    WebAction, WebCompanion,
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
    assert!(state_response.contains("\"version\":1"));

    let body = r#"{"type":"togglePlayback"}"#;
    let action_response = send_request(
        server.url(),
        &action_request(&authority, body, Some(&format!("http://{authority}"))),
    );
    assert!(action_response.starts_with("HTTP/1.1 202 Accepted"));
    assert_eq!(action_rx.try_recv(), Ok(WebAction::TogglePlayback));
}

#[test]
fn actions_reject_cross_origin_requests_and_require_strict_json_and_valid_targets() {
    let app = App::default();
    let state = Arc::new(RwLock::new(app.web_bridge_state()));
    let (action_tx, action_rx) = mpsc::sync_channel(4);
    let server = start_test_server(state, action_tx, 0, 1).expect("server");
    let authority = server_address(server.url()).to_string();

    let missing_origin = send_request(
        server.url(),
        &action_request(&authority, r#"{"type":"stop"}"#, None),
    );
    assert!(missing_origin.starts_with("HTTP/1.1 202 Accepted"));
    assert_eq!(action_rx.try_recv(), Ok(WebAction::Stop));

    let cross_origin = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"stop"}"#,
            Some("https://example.test"),
        ),
    );
    assert!(cross_origin.starts_with("HTTP/1.1 403 Forbidden"));

    let unknown_field = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"stop","extra":true}"#,
            Some(&format!("http://{authority}")),
        ),
    );
    assert!(unknown_field.starts_with("HTTP/1.1 400 Bad Request"));

    let out_of_range = send_request(
        server.url(),
        &action_request(
            &authority,
            r#"{"type":"toggleTrackMute","index":99}"#,
            Some(&format!("http://{authority}")),
        ),
    );
    assert!(out_of_range.starts_with("HTTP/1.1 422 Unprocessable Content"));
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
    assert!(oversized.starts_with("HTTP/1.1 413 Payload Too Large"));

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

    app.apply_web_action(WebAction::ToggleTrackMute { index: 0 });
    app.apply_web_action(WebAction::ToggleTrackSolo { index: 1 });

    assert!(app.song.tracks[0].muted);
    assert!(app.song.tracks[1].solo);
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
        r#"{"type":"stop"}"#,
        Some(&format!("http://{authority}")),
    );

    assert!(send_request(server.url(), &request).starts_with("HTTP/1.1 202 Accepted"));
    assert!(send_request(server.url(), &request).starts_with("HTTP/1.1 503 Service Unavailable"));
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
    stream.shutdown(Shutdown::Write).expect("shutdown write");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("read response");
    response
}
