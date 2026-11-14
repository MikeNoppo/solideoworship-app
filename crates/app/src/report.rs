use std::collections::hash_map::RandomState;
use std::fmt::Display;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rfd::{MessageDialog, MessageLevel};
use serde::{Deserialize, Serialize};

use crate::i18n::{self, fill, t};
use crate::paths;

const NO_CONSOLE: bool = cfg!(all(windows, not(debug_assertions)));
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const SECONDS_PER_DAY: u64 = 86_400;
const DEFAULT_TELEMETRY_URL: &str = "https://solideoworship.com/api/desktop/telemetry";
const DEFAULT_REPORT_URL: &str = "https://solideoworship.com/api/desktop/reports";

static MAIN_PANIC: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashRecord {
    pub app_version: Box<str>,
    pub os: Box<str>,
    pub arch: Box<str>,
    pub thread: Box<str>,
    pub location: Option<Box<str>>,
    pub panic_message: Box<str>,
    pub occurred_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportKind {
    Crash,
    Bug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportCategory {
    #[default]
    Bug,
    Display,
    Crash,
    Other,
}

impl ReportCategory {
    pub const ALL: [Self; 4] = [Self::Bug, Self::Display, Self::Crash, Self::Other];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPayload {
    pub kind: ReportKind,
    pub category: ReportCategory,
    pub message: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Box<str>>,
    pub app_version: Box<str>,
    pub os: Box<str>,
    pub arch: Box<str>,
    pub locale: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crash: Option<CrashRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPayload {
    pub install_id: Box<str>,
    pub app_version: Box<str>,
    pub os: Box<str>,
    pub arch: Box<str>,
    pub locale: Box<str>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct TelemetryState {
    install_id: String,
    last_ping_day: Option<u64>,
}

pub fn run(app: impl FnOnce() -> iced::Result) -> ExitCode {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let name = thread::current().name().unwrap_or("unnamed").to_owned();
        let record = panic_record(info, &name);
        record_crash(&paths::crash_file(), &record);

        if NO_CONSOLE {
            if name == "main"
                && let Ok(mut panic) = MAIN_PANIC.lock()
            {
                *panic = Some(info.to_string());
            }
        } else {
            previous(info);
        }
    }));

    match panic::catch_unwind(AssertUnwindSafe(app)) {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            let record = fatal_record(&error);
            record_crash(&paths::crash_file(), &record);
            fatal(&error);
            ExitCode::FAILURE
        }
        Err(_) if NO_CONSOLE => {
            let message = MAIN_PANIC
                .lock()
                .ok()
                .and_then(|mut panic| panic.take())
                .unwrap_or_default();
            fatal(&message);
            ExitCode::FAILURE
        }
        Err(payload) => panic::resume_unwind(payload),
    }
}

pub fn warn(message: &str) {
    show(MessageLevel::Warning, message);
}

pub fn record_crash(path: &Path, record: &CrashRecord) {
    let Ok(json) = serde_json::to_vec(record) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, json);
}

pub fn load_crash(path: &Path) -> Option<CrashRecord> {
    let bytes = fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(record) => Some(record),
        Err(_) => {
            let _ = fs::remove_file(path);
            None
        }
    }
}

pub fn clear_crash(path: &Path) {
    let _ = fs::remove_file(path);
}

pub fn submit_report(payload: &ReportPayload) -> Result<(), Box<str>> {
    let Some(url) = report_endpoint() else {
        return Ok(());
    };
    submit_report_to(&url, payload)
}

pub fn submit_report_to(url: &str, payload: &ReportPayload) -> Result<(), Box<str>> {
    let body =
        serde_json::to_string(payload).map_err(|error| error.to_string().into_boxed_str())?;
    post_json(url, &body)
}

pub fn ping_telemetry(enabled: bool) {
    if !enabled {
        return;
    }
    let Some(url) = telemetry_endpoint() else {
        return;
    };
    let locale = i18n::code();
    let day = now_unix() / SECONDS_PER_DAY;
    let _ = thread::Builder::new()
        .name("sdw-telemetry".into())
        .spawn(move || {
            let _ = send_telemetry_to(&paths::telemetry_file(), &url, locale, day);
        });
}

pub fn send_telemetry_to(
    state_path: &Path,
    url: &str,
    locale: &str,
    day: u64,
) -> Result<bool, Box<str>> {
    let mut state = fs::read_to_string(state_path)
        .ok()
        .and_then(|json| serde_json::from_str::<TelemetryState>(&json).ok())
        .unwrap_or_default();

    if state.last_ping_day == Some(day) && !state.install_id.is_empty() {
        return Ok(false);
    }

    if state.install_id.trim().is_empty() {
        state.install_id = generate_install_id();
    }

    let payload = TelemetryPayload {
        install_id: state.install_id.clone().into_boxed_str(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        locale: locale.into(),
    };
    let body =
        serde_json::to_string(&payload).map_err(|error| error.to_string().into_boxed_str())?;
    post_json(url, &body)?;

    state.last_ping_day = Some(day);
    if let Ok(json) = serde_json::to_vec(&state) {
        if let Some(parent) = state_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(state_path, json);
    }

    Ok(true)
}

pub(crate) fn post_json(url: &str, body: &str) -> Result<(), Box<str>> {
    if let Some(rest) = url.strip_prefix("http://") {
        return post_http(rest, body);
    }
    if url.starts_with("https://") {
        return post_curl(url, body);
    }
    Err("Unsupported report endpoint URL scheme.".into())
}

fn post_http(rest: &str, body: &str) -> Result<(), Box<str>> {
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let socket_target = if authority.contains(':') {
        authority.to_owned()
    } else {
        format!("{authority}:80")
    };
    let addr = socket_target
        .to_socket_addrs()
        .map_err(|error| error.to_string().into_boxed_str())?
        .next()
        .ok_or_else(|| Box::<str>::from("Could not resolve report host."))?;

    let mut stream = TcpStream::connect_timeout(&addr, HTTP_TIMEOUT)
        .map_err(|error| error.to_string().into_boxed_str())?;
    let _ = stream.set_read_timeout(Some(HTTP_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HTTP_TIMEOUT));

    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nUser-Agent: SolideoWorship/{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        env!("CARGO_PKG_VERSION"),
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string().into_boxed_str())?;
    stream
        .flush()
        .map_err(|error| error.to_string().into_boxed_str())?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|error| error.to_string().into_boxed_str())?;

    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .unwrap_or(0);

    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(format!("Server returned HTTP {status}.").into_boxed_str())
    }
}

fn post_curl(url: &str, body: &str) -> Result<(), Box<str>> {
    let mut command = Command::new("curl");
    command
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--max-time",
            "10",
            "--request",
            "POST",
            "--header",
            "Content-Type: application/json",
            "--user-agent",
            concat!("SolideoWorship/", env!("CARGO_PKG_VERSION")),
            "--data-binary",
            "@-",
            url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let mut child = command
        .spawn()
        .map_err(|error| error.to_string().into_boxed_str())?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(body.as_bytes())
            .map_err(|error| error.to_string().into_boxed_str())?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string().into_boxed_str())?;

    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if detail.is_empty() {
            Err("Could not send report.".into())
        } else {
            Err(detail.into_boxed_str())
        }
    }
}

fn telemetry_endpoint() -> Option<String> {
    non_empty_env("SDW_TELEMETRY_URL")
        .or_else(|| option_env!("SDW_TELEMETRY_URL").map(str::to_owned))
        .or_else(|| (!cfg!(debug_assertions)).then(|| DEFAULT_TELEMETRY_URL.to_owned()))
}

fn report_endpoint() -> Option<String> {
    non_empty_env("SDW_REPORT_URL")
        .or_else(|| option_env!("SDW_REPORT_URL").map(str::to_owned))
        .or_else(|| (!cfg!(debug_assertions)).then(|| DEFAULT_REPORT_URL.to_owned()))
}

pub(crate) fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn panic_record(info: &PanicHookInfo<'_>, thread: &str) -> CrashRecord {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .map(str::to_owned)
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| info.to_string());
    let location = info
        .location()
        .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()).into_boxed_str());

    CrashRecord {
        app_version: env!("CARGO_PKG_VERSION").into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        thread: thread.into(),
        location,
        panic_message: message.into_boxed_str(),
        occurred_at: now_unix(),
    }
}

fn fatal_record(error: &dyn Display) -> CrashRecord {
    CrashRecord {
        app_version: env!("CARGO_PKG_VERSION").into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        thread: "main".into(),
        location: None,
        panic_message: error.to_string().into_boxed_str(),
        occurred_at: now_unix(),
    }
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn generate_install_id() -> String {
    let mut high = RandomState::new().build_hasher();
    high.write_u64(now_unix());
    high.write_u32(std::process::id());
    let a = high.finish();

    let mut low = RandomState::new().build_hasher();
    low.write_u64(a.rotate_left(17));
    let b = low.finish();

    let time_low = (a >> 32) as u32;
    let time_mid = ((a >> 16) & 0xffff) as u16;
    let time_hi = ((a & 0x0fff) as u16) | 0x4000;
    let clock_seq = (((b >> 48) & 0x3fff) as u16) | 0x8000;
    let node = b & 0x0000_ffff_ffff_ffff;

    format!("{time_low:08x}-{time_mid:04x}-{time_hi:04x}-{clock_seq:04x}-{node:012x}")
}

fn fatal(error: &dyn Display) {
    show(
        MessageLevel::Error,
        &fill(t().app.fatal_error, &[("error", error)]),
    );
}

fn show(level: MessageLevel, message: &str) {
    if NO_CONSOLE {
        MessageDialog::new()
            .set_level(level)
            .set_title("SolideoWorship")
            .set_description(message)
            .show();
    } else {
        eprintln!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::{env, fs, process, thread};

    use super::{
        CrashRecord, ReportCategory, ReportKind, ReportPayload, TelemetryPayload, clear_crash,
        load_crash, record_crash, send_telemetry_to, submit_report_to,
    };

    fn accept_once(status: &str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let reply =
            format!("HTTP/1.1 {status}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 4096];
            loop {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&buffer[..header_end]);
                    let len = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("Content-Length: ")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if buffer.len() >= header_end + 4 + len {
                        let body =
                            String::from_utf8_lossy(&buffer[header_end + 4..header_end + 4 + len])
                                .into_owned();
                        let _ = stream.write_all(reply.as_bytes());
                        return body;
                    }
                }
            }
            String::new()
        });
        (format!("http://{addr}/api/desktop/telemetry"), handle)
    }

    #[test]
    fn crash_record_round_trips_to_disk() {
        let dir = env::temp_dir().join(format!("sdw-crash-test-{}", process::id()));
        let path = dir.join("pending_crash.json");
        let record = CrashRecord {
            app_version: "0.1.0-beta.2".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            thread: "main".into(),
            location: Some("crates/app/src/main.rs:42:5".into()),
            panic_message: "something failed".into(),
            occurred_at: 1_700_000_000,
        };

        record_crash(&path, &record);
        let stored = load_crash(&path).unwrap();
        assert_eq!(stored, record);
        clear_crash(&path);
        assert_eq!(load_crash(&path), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn report_submission_posts_json_and_propagates_http_errors() {
        let payload = ReportPayload {
            kind: ReportKind::Bug,
            category: ReportCategory::Display,
            message: "Projector flickers".into(),
            contact: Some("operator@church.org".into()),
            app_version: "0.1.0-beta.2".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            locale: "id".into(),
            crash: None,
        };
        let (url, server) = accept_once("201 Created");
        assert_eq!(submit_report_to(&url, &payload), Ok(()));
        let body = server.join().unwrap();
        let received: ReportPayload = serde_json::from_str(&body).unwrap();
        assert_eq!(received, payload);

        let (bad_url, bad_server) = accept_once("500 Internal Server Error");
        assert!(submit_report_to(&bad_url, &payload).is_err());
        let _ = bad_server.join();
    }

    #[test]
    fn telemetry_ping_sends_once_per_day_and_persists_install_id() {
        let dir = env::temp_dir().join(format!("sdw-telemetry-test-{}", process::id()));
        let state_path = dir.join("telemetry.json");
        let (url, server) = accept_once("200 OK");

        assert_eq!(send_telemetry_to(&state_path, &url, "id", 100), Ok(true));
        let body = server.join().unwrap();
        let payload: TelemetryPayload = serde_json::from_str(&body).unwrap();
        assert_eq!(payload.install_id.len(), 36);
        assert_eq!(&*payload.locale, "id");
        assert_eq!(&*payload.app_version, env!("CARGO_PKG_VERSION"));

        assert_eq!(
            send_telemetry_to(&state_path, "http://127.0.0.1:1/unused", "id", 100),
            Ok(false)
        );

        let (url2, server2) = accept_once("200 OK");
        assert_eq!(send_telemetry_to(&state_path, &url2, "id", 101), Ok(true));
        let body2 = server2.join().unwrap();
        let payload2: TelemetryPayload = serde_json::from_str(&body2).unwrap();
        assert_eq!(payload2.install_id, payload.install_id);

        let _ = fs::remove_dir_all(dir);
    }
}
