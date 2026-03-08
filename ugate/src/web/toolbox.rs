//! Toolbox: network diagnostic tools (ping, traceroute, nslookup)
//! Stream output via WS broadcast channel

use crate::web::ws::WsManager;
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

static RUNNING: AtomicBool = AtomicBool::new(false);
static STOP: AtomicBool = AtomicBool::new(false);

const MAX_LINES: usize = 200;
const TIMEOUT_SECS: u64 = 60;

/// Validate target: only alphanumeric, dots, hyphens, colons (IPv6)
fn is_safe_target(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 253
        && !s.starts_with('-') // prevent flag injection (--help, --version)
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
}

/// POST /api/toolbox/run — start a diagnostic tool
pub fn handle_run(body: &str, ws_manager: &Arc<WsManager>) -> super::Resp {
    let tool = crate::web::jval(body, "tool").unwrap_or_default();
    let target = crate::web::jval(body, "target").unwrap_or_default();

    // Validate target first (before building args)
    if !is_safe_target(&target) {
        return crate::web::json_err(400, "invalid target");
    }

    // Validate tool and build args
    let (cmd, args): (&str, Vec<&str>) = match tool.as_str() {
        "ping" => ("ping", vec!["-c", "10", &target]),
        "traceroute" => ("traceroute", vec!["-m", "20", &target]),
        "nslookup" => ("nslookup", vec![&target, "8.8.8.8"]),
        _ => return crate::web::json_err(400, "invalid tool"),
    };

    // Only 1 tool at a time
    if RUNNING.swap(true, Ordering::SeqCst) {
        return crate::web::json_err(409, "a tool is already running");
    }
    STOP.store(false, Ordering::SeqCst);

    let broadcast_tx = ws_manager.broadcast_tx.clone();
    let cmd_owned = cmd.to_string();
    let args_owned: Vec<String> = args.iter().map(|a| a.to_string()).collect();

    std::thread::spawn(move || {
        run_tool(&cmd_owned, &args_owned, &broadcast_tx);
    });

    crate::web::json_resp(&format!(r#"{{"ok":true,"tool":"{}"}}"#, tool))
}

/// POST /api/toolbox/stop — kill running tool
pub fn handle_stop() -> super::Resp {
    STOP.store(true, Ordering::SeqCst);
    crate::web::json_resp(r#"{"ok":true}"#)
}

/// Run tool process, stream stdout line-by-line via broadcast
fn run_tool(cmd: &str, args: &[String], broadcast_tx: &tokio::sync::broadcast::Sender<String>) {
    let child = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let msg = format!(
                r#"{{"type":"toolbox","line":"Error: {}"}}"#,
                crate::web::json_escape(&e.to_string())
            );
            let _ = broadcast_tx.send(msg);
            let _ = broadcast_tx.send(r#"{"type":"toolbox","done":true,"code":-1}"#.to_string());
            RUNNING.store(false, Ordering::SeqCst);
            return;
        }
    };

    let stdout = child.stdout.take();
    let start = Instant::now();
    let mut lines = 0usize;

    if let Some(out) = stdout {
        let reader = std::io::BufReader::new(out);
        for line in reader.lines() {
            // Check limits
            if lines >= MAX_LINES
                || start.elapsed().as_secs() > TIMEOUT_SECS
                || STOP.load(Ordering::SeqCst)
            {
                let _ = child.kill();
                break;
            }
            if let Ok(line) = line {
                let msg = format!(
                    r#"{{"type":"toolbox","line":"{}"}}"#,
                    crate::web::json_escape(&line)
                );
                let _ = broadcast_tx.send(msg);
                lines += 1;
            }
        }
    }

    // Also read stderr (brief)
    if let Some(err) = child.stderr.take() {
        let reader = std::io::BufReader::new(err);
        for line in reader.lines().take(5) {
            if let Ok(line) = line {
                let msg = format!(
                    r#"{{"type":"toolbox","line":"{}"}}"#,
                    crate::web::json_escape(&line)
                );
                let _ = broadcast_tx.send(msg);
            }
        }
    }

    let code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
    let _ = broadcast_tx.send(format!(
        r#"{{"type":"toolbox","done":true,"code":{}}}"#,
        code
    ));
    RUNNING.store(false, Ordering::SeqCst);
}
