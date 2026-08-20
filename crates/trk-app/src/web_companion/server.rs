use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError},
        Arc, RwLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(test)]
use std::net::SocketAddr;

use super::{page::COMPANION_HTML, WebAction, WebActionRequest, WebBridgeState};

const FIRST_PORT: u16 = 3333;
const PORT_ATTEMPTS: u16 = 51;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 4 * 1024;
const SOCKET_TIMEOUT: Duration = Duration::from_millis(200);
// The full app suite heavily oversubscribes test threads; keep production's
// slow-client bound while allowing an in-process client to be scheduled.
#[cfg(test)]
const TEST_SOCKET_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_SLEEP: Duration = Duration::from_millis(5);

pub(super) struct WebServer {
    url: String,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl WebServer {
    pub(super) fn start(
        state: Arc<RwLock<WebBridgeState>>,
        action_tx: SyncSender<WebAction>,
    ) -> io::Result<Self> {
        Self::start_range(state, action_tx, FIRST_PORT, PORT_ATTEMPTS)
    }

    fn start_range(
        state: Arc<RwLock<WebBridgeState>>,
        action_tx: SyncSender<WebAction>,
        first_port: u16,
        attempts: u16,
    ) -> io::Result<Self> {
        Self::start_range_with_timeout(state, action_tx, first_port, attempts, SOCKET_TIMEOUT)
    }

    fn start_range_with_timeout(
        state: Arc<RwLock<WebBridgeState>>,
        action_tx: SyncSender<WebAction>,
        first_port: u16,
        attempts: u16,
        socket_timeout: Duration,
    ) -> io::Result<Self> {
        let listener = bind_loopback(first_port, attempts)?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let authority = format!("127.0.0.1:{}", address.port());
        let url = format!("http://{authority}/");
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let handle = thread::Builder::new()
            .name("trk-web-companion".to_string())
            .spawn(move || {
                serve_loop(
                    listener,
                    &authority,
                    &state,
                    &action_tx,
                    &thread_shutdown,
                    socket_timeout,
                );
            })?;
        Ok(Self {
            url,
            shutdown,
            handle: Some(handle),
        })
    }

    pub(super) fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for WebServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn bind_loopback(first_port: u16, attempts: u16) -> io::Result<TcpListener> {
    if first_port == 0 {
        return TcpListener::bind(("127.0.0.1", 0));
    }
    let mut last_error = None;
    for offset in 0..attempts.max(1) {
        let Some(port) = first_port.checked_add(offset) else {
            break;
        };
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => return Ok(listener),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "no loopback port available",
        )
    }))
}

fn serve_loop(
    listener: TcpListener,
    authority: &str,
    state: &Arc<RwLock<WebBridgeState>>,
    action_tx: &SyncSender<WebAction>,
    shutdown: &AtomicBool,
    socket_timeout: Duration,
) {
    while !shutdown.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, peer)) => {
                if peer.ip().is_loopback() {
                    handle_stream(stream, authority, state, action_tx, socket_timeout);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(IDLE_SLEEP);
            }
            Err(_) => break,
        }
    }
}

fn handle_stream(
    mut stream: TcpStream,
    authority: &str,
    state: &Arc<RwLock<WebBridgeState>>,
    action_tx: &SyncSender<WebAction>,
    socket_timeout: Duration,
) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(socket_timeout));
    let _ = stream.set_write_timeout(Some(socket_timeout));
    let response = match read_request(&mut stream) {
        Ok(request) => route(request, authority, state, action_tx),
        Err(response) => response,
    };
    let _ = write_response(&mut stream, response);
}

struct HttpRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

struct HttpResponse {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
    extra_headers: &'static [(&'static str, &'static str)],
}

impl HttpResponse {
    fn text(status: &'static str, message: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: message.as_bytes().to_vec(),
            extra_headers: &[],
        }
    }

    fn json(status: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body,
            extra_headers: &[],
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, HttpResponse> {
    let mut bytes = Vec::with_capacity(1024);
    let header_end = loop {
        if let Some(index) = find_bytes(&bytes, b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() >= MAX_HEADER_BYTES {
            return Err(HttpResponse::text(
                "431 Request Header Fields Too Large",
                "request headers are too large",
            ));
        }
        let mut chunk = [0_u8; 1024];
        match stream.read(&mut chunk) {
            Ok(0) => return Err(HttpResponse::text("400 Bad Request", "incomplete request")),
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(HttpResponse::text(
                    "408 Request Timeout",
                    "request timed out",
                ));
            }
            Err(_) => return Err(HttpResponse::text("400 Bad Request", "request read failed")),
        }
    };

    if header_end > MAX_HEADER_BYTES {
        return Err(HttpResponse::text(
            "431 Request Header Fields Too Large",
            "request headers are too large",
        ));
    }
    let header_text = std::str::from_utf8(&bytes[..header_end - 4])
        .map_err(|_| HttpResponse::text("400 Bad Request", "headers must be UTF-8"))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| HttpResponse::text("400 Bad Request", "missing request line"))?;
    let parts = request_line.split(' ').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| part.is_empty())
        || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1")
    {
        return Err(HttpResponse::text(
            "400 Bad Request",
            "invalid request line",
        ));
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.starts_with([' ', '\t']) {
            return Err(HttpResponse::text(
                "400 Bad Request",
                "folded headers are unsupported",
            ));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| HttpResponse::text("400 Bad Request", "invalid header"))?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || value == b'-')
        {
            return Err(HttpResponse::text("400 Bad Request", "invalid header name"));
        }
        let name = name.to_ascii_lowercase();
        if headers.insert(name, value.trim().to_string()).is_some() {
            return Err(HttpResponse::text("400 Bad Request", "duplicate header"));
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err(HttpResponse::text(
            "400 Bad Request",
            "transfer encoding is unsupported",
        ));
    }
    let content_length = match headers.get("content-length") {
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| HttpResponse::text("400 Bad Request", "invalid content length"))?,
        None => 0,
    };
    if content_length > MAX_BODY_BYTES {
        return Err(HttpResponse::text(
            "413 Payload Too Large",
            "request body is too large",
        ));
    }
    let expected_len = header_end.saturating_add(content_length);
    while bytes.len() < expected_len {
        let mut chunk = [0_u8; 1024];
        let remaining = expected_len - bytes.len();
        let read_len = remaining.min(chunk.len());
        match stream.read(&mut chunk[..read_len]) {
            Ok(0) => return Err(HttpResponse::text("400 Bad Request", "incomplete body")),
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(HttpResponse::text(
                    "408 Request Timeout",
                    "request timed out",
                ));
            }
            Err(_) => return Err(HttpResponse::text("400 Bad Request", "body read failed")),
        }
    }
    if bytes.len() != expected_len {
        return Err(HttpResponse::text(
            "400 Bad Request",
            "pipelined requests are unsupported",
        ));
    }
    Ok(HttpRequest {
        method,
        path,
        headers,
        body: bytes[header_end..expected_len].to_vec(),
    })
}

fn route(
    request: HttpRequest,
    authority: &str,
    state: &Arc<RwLock<WebBridgeState>>,
    action_tx: &SyncSender<WebAction>,
) -> HttpResponse {
    if request.headers.get("host").map(String::as_str) != Some(authority) {
        return HttpResponse::text("400 Bad Request", "invalid Host header");
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") if request.body.is_empty() => HttpResponse {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: COMPANION_HTML.as_bytes().to_vec(),
            extra_headers: &[(
                "Content-Security-Policy",
                "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
            )],
        },
        ("GET", "/api/state") if request.body.is_empty() => state_response(state),
        ("POST", "/api/action") => action_response(request, authority, state, action_tx),
        ("GET", "/" | "/api/state") => {
            HttpResponse::text("400 Bad Request", "GET requests must not have a body")
        }
        (_, "/" | "/api/state") => HttpResponse {
            status: "405 Method Not Allowed",
            content_type: "text/plain; charset=utf-8",
            body: b"method not allowed".to_vec(),
            extra_headers: &[("Allow", "GET")],
        },
        (_, "/api/action") => HttpResponse {
            status: "405 Method Not Allowed",
            content_type: "text/plain; charset=utf-8",
            body: b"method not allowed".to_vec(),
            extra_headers: &[("Allow", "POST")],
        },
        _ => HttpResponse::text("404 Not Found", "not found"),
    }
}

fn state_response(state: &Arc<RwLock<WebBridgeState>>) -> HttpResponse {
    let snapshot = match state.read() {
        Ok(state) => state.clone(),
        Err(error) => error.into_inner().clone(),
    };
    match serde_json::to_vec(&snapshot) {
        Ok(body) => HttpResponse::json("200 OK", body),
        Err(_) => HttpResponse::text("500 Internal Server Error", "state encoding failed"),
    }
}

fn action_response(
    request: HttpRequest,
    authority: &str,
    state: &Arc<RwLock<WebBridgeState>>,
    action_tx: &SyncSender<WebAction>,
) -> HttpResponse {
    let content_type = request
        .headers
        .get("content-type")
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if content_type != Some("application/json") {
        return HttpResponse::text("415 Unsupported Media Type", "expected application/json");
    }
    if request.headers.get("x-trk-request").map(String::as_str) != Some("1") {
        return HttpResponse::text("403 Forbidden", "missing request marker");
    }
    let expected_origin = format!("http://{authority}");
    if request.headers.get("origin") != Some(&expected_origin) {
        return HttpResponse::text("403 Forbidden", "invalid Origin header");
    }
    let action = match serde_json::from_slice::<WebActionRequest>(&request.body) {
        Ok(request) => request.into_action(),
        Err(_) => return HttpResponse::text("400 Bad Request", "invalid action"),
    };
    let (fresh, valid) = match state.read() {
        Ok(state) => (
            action.revision() == state.revision,
            action_in_range(action, &state),
        ),
        Err(error) => {
            let state = error.into_inner();
            (
                action.revision() == state.revision,
                action_in_range(action, &state),
            )
        }
    };
    if !fresh {
        return HttpResponse::text("409 Conflict", "stale state revision");
    }
    if !valid {
        return HttpResponse::text("422 Unprocessable Content", "action target is out of range");
    }
    match action_tx.try_send(action) {
        Ok(()) => HttpResponse::json("202 Accepted", br#"{"accepted":true}"#.to_vec()),
        Err(TrySendError::Full(_)) => HttpResponse::text(
            "503 Service Unavailable",
            "action queue is full; retry later",
        ),
        Err(TrySendError::Disconnected(_)) => {
            HttpResponse::text("503 Service Unavailable", "action bridge is unavailable")
        }
    }
}

fn action_in_range(action: WebAction, state: &WebBridgeState) -> bool {
    let pattern_row = |pattern: usize, row: usize| {
        state
            .patterns
            .get(pattern)
            .is_some_and(|pattern| row < pattern.rows)
    };
    match action {
        WebAction::SelectPattern { index, .. } => index < state.patterns.len(),
        WebAction::SelectTrack { index, .. }
        | WebAction::ToggleTrackMute { index, .. }
        | WebAction::ToggleTrackSolo { index, .. } => index < state.tracks.len(),
        WebAction::CreateNote {
            pattern,
            row,
            track,
            pitch,
            ..
        } => pattern_row(pattern, row) && track < state.tracks.len() && pitch <= 127,
        WebAction::MoveNote {
            pattern,
            row,
            track,
            to_row,
            pitch,
            ..
        } => {
            pattern_row(pattern, row)
                && pattern_row(pattern, to_row)
                && track < state.tracks.len()
                && pitch <= 127
        }
        WebAction::ResizeNote {
            pattern,
            row,
            track,
            gate,
            ..
        } => pattern_row(pattern, row) && track < state.tracks.len() && (1..=127).contains(&gate),
        WebAction::DeleteNote {
            pattern,
            row,
            track,
            ..
        }
        | WebAction::SetNoteVelocity {
            pattern,
            row,
            track,
            ..
        } => pattern_row(pattern, row) && track < state.tracks.len(),
        WebAction::SetCcPoint {
            pattern,
            row,
            track,
            controller,
            value,
            ..
        } => {
            pattern_row(pattern, row)
                && track < state.tracks.len()
                && controller <= 127
                && value <= 127
        }
        WebAction::ClearCcPoint {
            pattern,
            row,
            track,
            controller,
            ..
        } => pattern_row(pattern, row) && track < state.tracks.len() && controller <= 127,
        WebAction::TogglePlayback { .. } | WebAction::Stop { .. } => true,
    }
}

fn write_response(stream: &mut TcpStream, response: HttpResponse) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nCross-Origin-Resource-Policy: same-origin\r\n",
        response.status,
        response.content_type,
        response.body.len()
    )?;
    for (name, value) in response.extra_headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(stream, "\r\n")?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
pub(super) fn start_test_server(
    state: Arc<RwLock<WebBridgeState>>,
    action_tx: SyncSender<WebAction>,
    first_port: u16,
    attempts: u16,
) -> io::Result<WebServer> {
    WebServer::start_range_with_timeout(state, action_tx, first_port, attempts, TEST_SOCKET_TIMEOUT)
}

#[cfg(test)]
pub(super) fn server_address(url: &str) -> SocketAddr {
    url.trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .expect("server address")
}
