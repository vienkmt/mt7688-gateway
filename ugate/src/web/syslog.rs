//! Syslog viewer: stream `logread -f -e ugate` qua WebSocket
//! Chạy song song với toolbox (không block ping/traceroute)

use crate::web::ws::WsManager;
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

static RUNNING: AtomicBool = AtomicBool::new(false);
static STOP: AtomicBool = AtomicBool::new(false);
/// PID of logread process for cleanup
static PID: AtomicU32 = AtomicU32::new(0);

/// Messages to skip (trùng lặp với serial realtime tab)
const SKIP_CONTAINS: &[&str] = &[
    "[Dispatch] UART TX:",
    "[Dispatch] UART RX:",
    "[TCP] Nhận",
    "[MQTT] Publish",
    "[HTTP] POST",
];

/// POST /api/syslog/start — start streaming logread
pub fn handle_start(ws_manager: &Arc<WsManager>) -> super::Resp {
    // Nếu đang chạy, stop trước rồi start lại (tránh race condition)
    if RUNNING.load(Ordering::SeqCst) {
        STOP.store(true, Ordering::SeqCst);
        let pid = PID.load(Ordering::SeqCst);
        if pid > 0 {
            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
        }
        // Đợi thread cũ kết thúc (tối đa 500ms)
        for _ in 0..50 {
            if !RUNNING.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    RUNNING.store(true, Ordering::SeqCst);
    STOP.store(false, Ordering::SeqCst);

    let broadcast_tx = ws_manager.broadcast_tx.clone();

    std::thread::spawn(move || {
        stream_syslog(&broadcast_tx);
    });

    crate::web::json_resp(r#"{"ok":true}"#)
}

/// POST /api/syslog/stop — kill logread process
pub fn handle_stop() -> super::Resp {
    STOP.store(true, Ordering::SeqCst);
    let pid = PID.load(Ordering::SeqCst);
    if pid > 0 {
        unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    }
    crate::web::json_resp(r#"{"ok":true}"#)
}

/// Parse syslog line: extract time + message, detect level
/// Input: "Sun Mar  8 14:53:18 2026 daemon.info ugate[2136]: [Config] Loaded: ..."
/// Output: Some(("14:53:18", "info", "[Config] Loaded: ..."))
fn parse_syslog_line(line: &str) -> Option<(&str, &str, &str)> {
    let ugate_pos = line.find("ugate[")?;
    let msg_start = line[ugate_pos..].find(": ")?;
    let message = &line[ugate_pos + msg_start + 2..];

    let time = extract_time(line)?;

    let level = if line[..ugate_pos].contains("daemon.err") {
        "err"
    } else if line[..ugate_pos].contains("daemon.warn") {
        "warn"
    } else {
        "info"
    };

    Some((time, level, message))
}

/// Extract HH:MM:SS from syslog line (scan first 35 chars)
fn extract_time(line: &str) -> Option<&str> {
    let search = if line.len() > 35 { &line[..35] } else { line };
    for (i, _) in search.char_indices() {
        if i + 8 <= search.len() {
            let b = search[i..i + 8].as_bytes();
            if b[2] == b':' && b[5] == b':'
                && b[0].is_ascii_digit() && b[1].is_ascii_digit()
                && b[3].is_ascii_digit() && b[4].is_ascii_digit()
                && b[6].is_ascii_digit() && b[7].is_ascii_digit()
            {
                return Some(&search[i..i + 8]);
            }
        }
    }
    None
}

/// Check if message should be skipped
fn should_skip(message: &str) -> bool {
    SKIP_CONTAINS.iter().any(|s| message.contains(s))
}

/// Stream logread output via WS broadcast
fn stream_syslog(broadcast_tx: &tokio::sync::broadcast::Sender<String>) {
    // logread -f -e ugate: follow mode, filter by "ugate"
    // Không dùng -l vì OpenWrt logread không có flag đó
    // logread -f sẽ dump log cũ trước rồi follow — frontend giới hạn 200 dòng
    let child = Command::new("logread")
        .args(["-f", "-e", "ugate"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let msg = format!(
                r#"{{"type":"syslog","line":"Error: {}","level":"err"}}"#,
                crate::web::json_escape(&e.to_string())
            );
            let _ = broadcast_tx.send(msg);
            RUNNING.store(false, Ordering::SeqCst);
            return;
        }
    };

    PID.store(child.id(), Ordering::SeqCst);

    if let Some(out) = child.stdout.take() {
        let reader = std::io::BufReader::new(out);
        for line in reader.lines() {
            if STOP.load(Ordering::SeqCst) {
                let _ = child.kill();
                break;
            }

            if let Ok(raw) = line {
                if let Some((time, level, message)) = parse_syslog_line(&raw) {
                    if should_skip(message) {
                        continue;
                    }
                    let formatted = format!("{} {}", time, message);
                    let msg = format!(
                        r#"{{"type":"syslog","line":"{}","level":"{}"}}"#,
                        crate::web::json_escape(&formatted),
                        level
                    );
                    let _ = broadcast_tx.send(msg);
                }
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    PID.store(0, Ordering::SeqCst);
    RUNNING.store(false, Ordering::SeqCst);

    let _ = broadcast_tx.send(r#"{"type":"syslog","stopped":true}"#.to_string());
}
