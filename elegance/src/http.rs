//! HTTP requests and WebSockets. Requests run on background threads; results arrive on the UI
//! thread, where state may be written.

use std::ptr::NonNull;
use std::time::Duration;

use elegance_sys as sys;

use crate::error::{self, ffi, Error};
use crate::ffi as cffi;

pub type Headers = Vec<(String, String)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Headers,
    /// Sent as is; set a Content-Type header to go with it.
    pub body: Vec<u8>,
    pub timeout: Duration,
}

impl Default for Request {
    fn default() -> Self {
        Self { url: String::new(), method: "GET".into(), headers: Vec::new(), body: Vec::new(), timeout: Duration::from_secs(30) }
    }
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self { url: url.into(), ..Default::default() }
    }
    pub fn post(url: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self { url: url.into(), method: "POST".into(), body: body.into(), ..Default::default() }
    }
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into();
        self
    }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl From<&str> for Request {
    fn from(url: &str) -> Self {
        Request::get(url)
    }
}

impl From<String> for Request {
    fn from(url: String) -> Self {
        Request::get(url)
    }
}

fn c_headers(headers: &Headers) -> Vec<sys::ElgHeader> {
    headers.iter().map(|(n, v)| sys::ElgHeader { name: cffi::s(n), value: cffi::s(v) }).collect()
}

fn with_request<R>(request: &Request, f: impl FnOnce(&sys::ElgRequest) -> R) -> R {
    let headers = c_headers(&request.headers);
    let c = sys::ElgRequest {
        url: cffi::s(&request.url),
        method: cffi::s(&request.method),
        headers: headers.as_ptr(),
        header_count: headers.len(),
        body: if request.body.is_empty() { sys::ElgBytes::EMPTY } else { sys::ElgBytes::new(&request.body) },
        timeout_ms: request.timeout.as_millis() as i64,
    };
    f(&c)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub status: i32,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Response {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    /// The first header with this name (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

/// A response, or why the request failed (no connection, timeout, ...).
pub type HttpResult = Result<Response, String>;

unsafe fn read_response(r: &sys::ElgResponse) -> HttpResult { unsafe {
    if !r.ok {
        return Err(r.error.as_str().to_owned());
    }
    let headers = std::slice::from_raw_parts(r.headers, r.header_count)
        .iter()
        .map(|h| (h.name.as_str().to_owned(), h.value.as_str().to_owned()))
        .collect();
    Ok(Response { status: r.status, headers, body: r.body.as_slice().to_vec() })
}}

/// A request started with [`fetch`]: cancel it with `cancel` (or let it finish).
pub struct Task {
    ptr: NonNull<sys::HttpTask>,
}

impl Clone for Task {
    fn clone(&self) -> Self {
        Self { ptr: NonNull::new(unsafe { sys::elg_http_task_clone(self.ptr.as_ptr()) }).unwrap() }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        unsafe { sys::elg_http_task_free(self.ptr.as_ptr()) }
    }
}

impl Task {
    /// `done` won't run.
    pub fn cancel(&self) {
        ffi!(sys::elg_http_task_cancel(self.ptr.as_ptr()));
    }
}

/// Starts a request; `done` runs on the UI thread with the result (or not at all if cancelled,
/// or if no App is running by then). Call from any thread.
pub fn fetch(request: impl Into<Request>, done: impl FnOnce(HttpResult) + Send + 'static) -> Task {
    let request = request.into();
    let done = std::sync::Mutex::new(Some(done));
    let done = cffi::send_closure(move |args, _| {
        if let Some(done) = done.lock().unwrap_or_else(|e| e.into_inner()).take() {
            done(unsafe { read_response(&*(args as *const sys::ElgResponse)) });
        }
    });
    let ptr = with_request(&request, |r| ffi!(sys::elg_http_fetch(r, done)));
    Task { ptr: NonNull::new(ptr).unwrap() }
}

/// A request as state: views that read it update when it finishes. It starts when made, and is
/// cancelled when the last clone goes. UI thread.
///
/// ```ignore
/// let weather = Fetch::new("https://example.com/weather.json");
/// Text(move || if weather.is_loading() { "Loading".into() } else { weather.response().map(|r| r.text()).unwrap_or_default() })
/// ```
pub struct Fetch {
    ptr: NonNull<sys::HttpFetch>,
}

impl Clone for Fetch {
    fn clone(&self) -> Self {
        Self { ptr: NonNull::new(unsafe { sys::elg_http_fetch_state_clone(self.ptr.as_ptr()) }).unwrap() }
    }
}

impl Drop for Fetch {
    fn drop(&mut self) {
        unsafe { sys::elg_http_fetch_state_free(self.ptr.as_ptr()) }
    }
}

impl Fetch {
    pub fn new(request: impl Into<Request>) -> Self {
        let request = request.into();
        let ptr = with_request(&request, |r| ffi!(sys::elg_http_fetch_state(r)));
        Self { ptr: NonNull::new(ptr).unwrap() }
    }
    pub fn is_loading(&self) -> bool {
        ffi!(sys::elg_http_fetch_state_flag(self.ptr.as_ptr(), 0))
    }
    pub fn has_response(&self) -> bool {
        ffi!(sys::elg_http_fetch_state_flag(self.ptr.as_ptr(), 1))
    }
    /// A response arrived with a 2xx status.
    pub fn ok(&self) -> bool {
        ffi!(sys::elg_http_fetch_state_flag(self.ptr.as_ptr(), 2))
    }
    /// The response, or why there's none (the error, or "" while loading).
    pub fn result(&self) -> HttpResult {
        let mut result = Err(String::new());
        let mut each = |args: *mut std::ffi::c_void, _: *mut std::ffi::c_void| unsafe {
            result = read_response(&*(args as *const sys::ElgResponse));
        };
        ffi!(sys::elg_http_fetch_state_result(self.ptr.as_ptr(), cffi::borrowed(&mut each)));
        result
    }
    pub fn response(&self) -> Option<Response> {
        self.result().ok()
    }
    pub fn error(&self) -> String {
        self.result().err().unwrap_or_default()
    }
    /// Sends the request again.
    pub fn reload(&self) {
        ffi!(sys::elg_http_fetch_state_reload(self.ptr.as_ptr()));
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebSocketRequest {
    /// ws:// or wss://
    pub url: String,
    /// Sent with the opening handshake.
    pub headers: Headers,
    /// For the handshake only.
    pub timeout: Duration,
}

impl WebSocketRequest {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), headers: Vec::new(), timeout: Duration::from_secs(30) }
    }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

impl From<&str> for WebSocketRequest {
    fn from(url: &str) -> Self {
        WebSocketRequest::new(url)
    }
}

/// What a WebSocket tells you. Each runs on the UI thread; after `on_close` nothing else runs.
#[derive(Default)]
pub struct WebSocketEvents {
    pub on_open: Option<Box<dyn FnMut() + Send>>,
    pub on_message: Option<Box<dyn FnMut(&str) + Send>>,
    pub on_binary: Option<Box<dyn FnMut(&[u8]) + Send>>,
    /// The close code and reason.
    pub on_close: Option<Box<dyn FnMut(i32, &str) + Send>>,
}

impl WebSocketEvents {
    pub fn on_open(mut self, f: impl FnMut() + Send + 'static) -> Self {
        self.on_open = Some(Box::new(f));
        self
    }
    pub fn on_message(mut self, f: impl FnMut(&str) + Send + 'static) -> Self {
        self.on_message = Some(Box::new(f));
        self
    }
    pub fn on_binary(mut self, f: impl FnMut(&[u8]) + Send + 'static) -> Self {
        self.on_binary = Some(Box::new(f));
        self
    }
    pub fn on_close(mut self, f: impl FnMut(i32, &str) + Send + 'static) -> Self {
        self.on_close = Some(Box::new(f));
        self
    }
}

/// A WebSocket connection. Clones refer to the same connection.
pub struct WebSocket {
    ptr: NonNull<sys::WebSocket>,
}

impl Clone for WebSocket {
    fn clone(&self) -> Self {
        Self { ptr: NonNull::new(unsafe { sys::elg_websocket_clone(self.ptr.as_ptr()) }).unwrap() }
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        unsafe { sys::elg_websocket_free(self.ptr.as_ptr()) }
    }
}

unsafe impl Send for WebSocket {}
unsafe impl Sync for WebSocket {}

impl WebSocket {
    /// Starts connecting; call from any thread.
    pub fn connect(request: impl Into<WebSocketRequest>, events: WebSocketEvents) -> Result<WebSocket, Error> {
        let request = request.into();
        let headers = c_headers(&request.headers);
        let on_open = match events.on_open {
            Some(f) => {
                let f = std::sync::Mutex::new(f);
                cffi::send_closure(move |_, _| (f.lock().unwrap_or_else(|e| e.into_inner()))())
            }
            None => sys::ElgClosure::NONE,
        };
        let on_message = match events.on_message {
            Some(f) => {
                let f = std::sync::Mutex::new(f);
                cffi::send_closure(move |args, _| unsafe {
                    (f.lock().unwrap_or_else(|e| e.into_inner()))((*(args as *const sys::ElgStr)).as_str())
                })
            }
            None => sys::ElgClosure::NONE,
        };
        let on_binary = match events.on_binary {
            Some(f) => {
                let f = std::sync::Mutex::new(f);
                cffi::send_closure(move |args, _| unsafe {
                    (f.lock().unwrap_or_else(|e| e.into_inner()))((*(args as *const sys::ElgBytes)).as_slice())
                })
            }
            None => sys::ElgClosure::NONE,
        };
        let on_close = match events.on_close {
            Some(f) => {
                let f = std::sync::Mutex::new(f);
                cffi::send_closure(move |args, _| unsafe {
                    let close = &*(args as *const sys::ElgWebSocketClose);
                    (f.lock().unwrap_or_else(|e| e.into_inner()))(close.code, close.reason.as_str())
                })
            }
            None => sys::ElgClosure::NONE,
        };
        let events = sys::ElgWebSocketEvents { on_open, on_message, on_binary, on_close };
        let ptr = unsafe {
            sys::elg_websocket_connect(cffi::s(&request.url), headers.as_ptr(), headers.len(), request.timeout.as_millis() as i64, events)
        };
        error::result()?;
        Ok(Self { ptr: NonNull::new(ptr).unwrap() })
    }
    /// Sends a text message (UTF-8). Queued while connecting; dropped once closed. Any thread.
    pub fn send(&self, text: &str) {
        ffi!(sys::elg_websocket_send(self.ptr.as_ptr(), cffi::s(text), false));
    }
    pub fn send_binary(&self, bytes: &[u8]) {
        let text = sys::ElgStr { ptr: bytes.as_ptr().cast(), len: bytes.len() };
        ffi!(sys::elg_websocket_send(self.ptr.as_ptr(), text, true));
    }
    /// Closes with this code and reason; its events don't run anymore. Any thread.
    pub fn close(&self, code: i32, reason: &str) {
        ffi!(sys::elg_websocket_close(self.ptr.as_ptr(), code, cffi::s(reason)));
    }
    /// Connected and not closed (yet). UI thread, as events see it.
    pub fn is_open(&self) -> bool {
        ffi!(sys::elg_websocket_is_open(self.ptr.as_ptr()))
    }
}
