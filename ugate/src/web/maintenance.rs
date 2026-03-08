//! System Maintenance API handlers
//! Backup/restore config, factory reset, restart, reload, version, upgrade

use crate::config::{AppState, Config};
use crate::web::{json_err, json_escape, json_resp, Resp};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Guard chống concurrent upgrade (chỉ cho phép 1 upgrade tại 1 thời điểm)
static UPGRADING: AtomicBool = AtomicBool::new(false);

/// GET /api/upgrade/url — lấy upgrade URL từ UCI
pub fn handle_get_upgrade_url() -> Resp {
    let url = crate::uci::Uci::get("ugate.@upgrade[0].url").unwrap_or_default();
    json_resp(&format!(r#"{{"url":"{}"}}"#, json_escape(&url)))
}

/// POST /api/upgrade/url — lưu upgrade URL vào UCI
pub fn handle_set_upgrade_url(body: &str) -> Resp {
    let url = crate::web::jval(body, "url").unwrap_or_default();
    let _ = crate::uci::Uci::set("ugate.@upgrade[0].url", &url);
    let _ = crate::uci::Uci::commit("ugate");
    log::info!("[Maint] Upgrade URL set: {}", url);
    json_resp(r#"{"ok":true}"#)
}

/// GET /api/version — trả về version, build date, git commit
pub fn handle_version() -> Resp {
    json_resp(&format!(
        r#"{{"version":"{}","build_date":"{}","git_commit":"{}"}}"#,
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_DATE"),
        env!("GIT_COMMIT"),
    ))
}

/// GET /api/backup — download file config UCI dạng plain text
pub fn handle_backup() -> Resp {
    let config_path = "/etc/config/ugate";
    match std::fs::read(config_path) {
        Ok(data) => {
            let resp = tiny_http::Response::from_data(data)
                .with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"application/octet-stream"[..],
                    )
                    .unwrap(),
                )
                .with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Disposition"[..],
                        &b"attachment; filename=ugate.config"[..],
                    )
                    .unwrap(),
                );
            // Chuyển Response<Cursor<Vec<u8>>> — tiny_http::Response::from_data trả đúng type
            resp
        }
        Err(e) => {
            log::error!("[Maint] Backup failed: {}", e);
            json_err(500, "cannot read config file")
        }
    }
}

/// POST /api/restore — upload config file (raw body), validate rồi ghi đè
pub fn handle_restore(request: &mut tiny_http::Request, state: &Arc<AppState>) -> Resp {
    let body = read_body_raw(request, 64 * 1024); // UCI config max 64KB
    if body.is_empty() {
        return json_err(400, "empty body");
    }

    // Validate: phải là UTF-8 text và chứa ít nhất 1 UCI section
    let text = match std::str::from_utf8(&body) {
        Ok(t) => t,
        Err(_) => return json_err(400, "invalid config: not UTF-8 text"),
    };
    if !text.contains("config ") {
        return json_err(400, "invalid config: no UCI sections found");
    }

    // Backup config hiện tại trước khi ghi đè
    let _ = std::fs::copy("/etc/config/ugate", "/tmp/ugate.backup");

    // Ghi config mới
    if let Err(e) = std::fs::write("/etc/config/ugate", &body) {
        log::error!("[Maint] Restore write failed: {}", e);
        return json_err(500, "failed to write config");
    }

    // Re-read config từ UCI và apply
    let new_cfg = Config::load();
    state.update(new_cfg);
    log::info!("[Maint] Config restored and applied");

    json_resp(r#"{"ok":true,"message":"config restored and applied"}"#)
}

/// POST /api/factory-reset — reset ugate config về mặc định
pub fn handle_factory_reset(state: &Arc<AppState>) -> Resp {
    // Ghi default config
    let default = Config::default();
    default.save_to_uci();

    // Apply vào state
    state.update(default);
    log::info!("[Maint] Factory reset: config restored to defaults");

    json_resp(r#"{"ok":true,"message":"config reset to defaults"}"#)
}

/// POST /api/restart — reboot device (trả response trước, reboot sau 1s)
pub fn handle_restart() -> Resp {
    log::info!("[Maint] Device restart requested");
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        Command::new("reboot").status().ok();
    });
    json_resp(r#"{"ok":true,"message":"restarting..."}"#)
}

/// POST /api/upgrade — upload file IPK (raw body), install qua opkg
pub fn handle_upgrade_upload(request: &mut tiny_http::Request) -> Resp {
    if UPGRADING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return json_err(409, "upgrade already in progress");
    }

    let body = read_body_raw(request, 10 * 1024 * 1024); // IPK max 2MB
    if body.len() < 100 {
        UPGRADING.store(false, Ordering::SeqCst);
        return json_err(400, "file too small or empty");
    }

    // Validate IPK: ar archive bắt đầu bằng "!<arch>\n"
    if !body.starts_with(b"!<arch>") {
        UPGRADING.store(false, Ordering::SeqCst);
        return json_err(400, "invalid IPK file format");
    }

    // Ghi vào /tmp
    if let Err(e) = std::fs::write("/tmp/ugate.ipk", &body) {
        log::error!("[Maint] IPK write failed: {}", e);
        UPGRADING.store(false, Ordering::SeqCst);
        return json_err(500, "failed to save IPK");
    }

    log::info!("[Maint] IPK uploaded: {} bytes, installing...", body.len());

    // Install async: trả response trước, install + restart service sau
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let status = Command::new("opkg")
            .args(["install", "/tmp/ugate.ipk", "--force-reinstall"])
            .status();
        match status {
            Ok(s) if s.success() => {
                log::info!("[Maint] IPK installed, restarting service...");
                Command::new("/etc/init.d/ugate")
                    .arg("restart")
                    .status()
                    .ok();
            }
            Ok(s) => log::error!("[Maint] opkg install failed: exit {}", s),
            Err(e) => log::error!("[Maint] opkg exec failed: {}", e),
        }
        let _ = std::fs::remove_file("/tmp/ugate.ipk");
        UPGRADING.store(false, Ordering::SeqCst);
    });

    json_resp(r#"{"ok":true,"message":"installing... service will restart"}"#)
}

/// GET /api/upgrade/check — check update từ remote URL (cấu hình trong UCI)
pub fn handle_upgrade_check() -> Resp {
    let url = crate::uci::Uci::get("ugate.@upgrade[0].url").unwrap_or_default();
    if url.is_empty() {
        return json_err(400, "upgrade URL not configured");
    }

    // Fetch manifest JSON từ remote
    let resp = match ureq::get(&url).call() {
        Ok(r) => r,
        Err(e) => {
            log::error!("[Maint] Upgrade check failed: {}", e);
            return json_err(502, "failed to fetch update info");
        }
    };

    let manifest = match resp.into_string() {
        Ok(s) => s,
        Err(_) => return json_err(502, "invalid response from update server"),
    };

    // Parse manifest JSON thủ công
    let latest = crate::web::jval(&manifest, "version").unwrap_or_default();
    let changelog = crate::web::jval(&manifest, "changelog").unwrap_or_default();
    let size = crate::web::jval(&manifest, "size").unwrap_or_default();
    let ipk_url = crate::web::jval(&manifest, "url").unwrap_or_default();

    let current = env!("CARGO_PKG_VERSION");
    let has_update = version_gt(&latest, current);

    json_resp(&format!(
        r#"{{"current_version":"{}","latest_version":"{}","has_update":{},"changelog":"{}","size":"{}","url":"{}"}}"#,
        current,
        json_escape(&latest),
        has_update,
        json_escape(&changelog),
        json_escape(&size),
        json_escape(&ipk_url),
    ))
}

/// POST /api/upgrade/remote — download IPK từ remote, verify checksum, install
pub fn handle_upgrade_remote() -> Resp {
    if UPGRADING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return json_err(409, "upgrade already in progress");
    }

    let url = crate::uci::Uci::get("ugate.@upgrade[0].url").unwrap_or_default();
    if url.is_empty() {
        UPGRADING.store(false, Ordering::SeqCst);
        return json_err(400, "upgrade URL not configured");
    }

    // Fetch manifest
    let manifest = match ureq::get(&url).call().and_then(|r| r.into_string().map_err(Into::into)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[Maint] Remote upgrade manifest fetch failed: {}", e);
            UPGRADING.store(false, Ordering::SeqCst);
            return json_err(502, "failed to fetch update info");
        }
    };

    let ipk_url = crate::web::jval(&manifest, "url").unwrap_or_default();
    let expected_checksum = crate::web::jval(&manifest, "checksum").unwrap_or_default();
    let version = crate::web::jval(&manifest, "version").unwrap_or_default();

    if ipk_url.is_empty() {
        UPGRADING.store(false, Ordering::SeqCst);
        return json_err(400, "no download URL in manifest");
    }

    log::info!("[Maint] Remote upgrade to v{} from {}", version, ipk_url);

    let version_clone = version.clone();
    // Download + install trong thread riêng (trả response trước)
    std::thread::spawn(move || {
        // Download IPK
        let resp = match ureq::get(&ipk_url).call() {
            Ok(r) => r,
            Err(e) => {
                log::error!("[Maint] IPK download failed: {}", e);
                UPGRADING.store(false, Ordering::SeqCst);
                return;
            }
        };

        let reader = resp.into_reader();
        let mut limited = std::io::Read::take(reader, 10 * 1024 * 1024);
        let mut data = Vec::new();
        // Giới hạn 2MB tránh OOM trên thiết bị 64MB RAM
        if let Err(e) = std::io::Read::read_to_end(&mut limited, &mut data) {
            log::error!("[Maint] IPK read failed: {}", e);
            UPGRADING.store(false, Ordering::SeqCst);
            return;
        }

        // Ghi vào /tmp
        if let Err(e) = std::fs::write("/tmp/ugate.ipk", &data) {
            log::error!("[Maint] IPK write failed: {}", e);
            UPGRADING.store(false, Ordering::SeqCst);
            return;
        }

        // Verify checksum nếu có (dùng sha256sum CLI)
        if !expected_checksum.is_empty() {
            if !verify_checksum("/tmp/ugate.ipk", &expected_checksum) {
                log::error!("[Maint] Checksum mismatch, aborting upgrade");
                let _ = std::fs::remove_file("/tmp/ugate.ipk");
                UPGRADING.store(false, Ordering::SeqCst);
                return;
            }
        }

        // Install
        let status = Command::new("opkg")
            .args(["install", "/tmp/ugate.ipk", "--force-reinstall"])
            .status();
        match status {
            Ok(s) if s.success() => {
                log::info!("[Maint] v{} installed, restarting service...", version_clone);
                Command::new("/etc/init.d/ugate")
                    .arg("restart")
                    .status()
                    .ok();
            }
            Ok(s) => log::error!("[Maint] opkg install failed: exit {}", s),
            Err(e) => log::error!("[Maint] opkg exec failed: {}", e),
        }
        let _ = std::fs::remove_file("/tmp/ugate.ipk");
        UPGRADING.store(false, Ordering::SeqCst);
    });

    json_resp(&format!(
        r#"{{"ok":true,"message":"downloading and installing v{}..."}}"#,
        json_escape(&version),
    ))
}

// --- Helpers ---

/// Đọc raw binary body từ request (không giới hạn 4KB)
fn read_body_raw(request: &mut tiny_http::Request, max_size: usize) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    let _ = request.as_reader().take(max_size as u64).read_to_end(&mut buf);
    buf
}

/// So sánh version: trả true nếu a > b (semver đơn giản: x.y.z)
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    let va = parse(a);
    let vb = parse(b);
    va > vb
}

/// Verify SHA256 checksum dùng sha256sum CLI (có sẵn trên OpenWrt)
/// Format checksum: "sha256:abcdef..." hoặc plain hash
fn verify_checksum(file_path: &str, expected: &str) -> bool {
    let output = match Command::new("sha256sum").arg(file_path).output() {
        Ok(o) if o.status.success() => o,
        _ => {
            log::error!("[Maint] sha256sum not available, cannot verify checksum");
            return false; // reject nếu không verify được
        }
    };

    let actual = String::from_utf8_lossy(&output.stdout);
    let actual_hash = actual.split_whitespace().next().unwrap_or("");

    // Hỗ trợ format "sha256:xxx" hoặc plain hash
    let expected_hash = expected.strip_prefix("sha256:").unwrap_or(expected);

    if actual_hash == expected_hash {
        log::info!("[Maint] Checksum verified OK");
        true
    } else {
        log::error!(
            "[Maint] Checksum mismatch: expected={} actual={}",
            expected_hash,
            actual_hash
        );
        false
    }
}
