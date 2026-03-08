//! Kênh HTTP POST publisher (async)
//! Nhận dữ liệu UART qua tokio mpsc channel, POST tới URL đã cấu hình
//! Dùng ureq (sync) trong spawn_blocking để không block tokio runtime
//! Tự động reload khi config thay đổi

use crate::commands::Command;
use crate::config::AppState;
use crate::web::status::SharedStats;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::sync::mpsc;

/// Vòng lặp chính: chờ dữ liệu và POST, đọc response body gửi ngược MCU
pub async fn run(
    state: Arc<AppState>,
    mut data_rx: mpsc::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<Command>,
    stats: Arc<SharedStats>,
) {
    loop {
        let config = state.get();
        if !config.http.enabled || config.http.url.is_empty() {
            stats.http_state.store(0, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_secs(5)).await;
            while data_rx.try_recv().is_ok() {}
            continue;
        }
        stats.http_state.store(2, Ordering::Relaxed); // active = connected
        if let Err(e) = run_publish_loop(&state, &mut data_rx, &cmd_tx, &stats).await {
            log::error!("[HTTP] Lỗi: {}. Thử lại sau 10s...", e);
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}

/// Vòng lặp publish: nhận dữ liệu từ channel, POST qua ureq
async fn run_publish_loop(
    state: &AppState,
    data_rx: &mut mpsc::Receiver<Vec<u8>>,
    cmd_tx: &mpsc::Sender<Command>,
    stats: &Arc<SharedStats>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();
    let mut config_watch = state.subscribe();

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();

    let method = config.http.method.clone();
    let method_str = match method {
        crate::config::HttpMethod::Get => "GET",
        crate::config::HttpMethod::Post => "POST",
    };
    log::info!("[HTTP] {} tới '{}'", method_str, config.http.url);

    loop {
        tokio::select! {
            _ = config_watch.changed() => {
                log::info!("[HTTP] Config thay đổi, reload...");
                return Ok(());
            }

            Some(data) = data_rx.recv() => {
                let url = config.http.url.clone();
                let agent = agent.clone();
                let is_get = method == crate::config::HttpMethod::Get;

                // Detect wrapped JSON (bắt đầu bằng '{') hoặc raw bytes
                let is_wrapped = data.first() == Some(&b'{');
                // Text encoding: gửi string UTF-8, ngược lại hex
                let data_str = if is_wrapped {
                    // Wrapped JSON từ fan-out → gửi trực tiếp
                    String::from_utf8_lossy(&data).into_owned()
                } else if config.general.data_as_text {
                    // Text mode: thử UTF-8, fallback hex
                    match std::str::from_utf8(&data) {
                        Ok(s) => format!(r#"{{"data":"{}","len":{}}}"#, crate::web::json_escape(s), data.len()),
                        Err(_) => {
                            let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                            format!(r#"{{"data":"{}","len":{}}}"#, hex, data.len())
                        }
                    }
                } else {
                    let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                    format!(r#"{{"data":"{}","len":{}}}"#, hex, data.len())
                };
                // GET query value: wrapped → parse fields, raw → data field
                let get_query = if is_wrapped {
                    let dname = crate::web::jval(&data_str, "device_name").unwrap_or_default();
                    let ts = crate::web::jval(&data_str, "timestamp").unwrap_or_default();
                    let dv = crate::web::jval(&data_str, "data").unwrap_or_default();
                    format!("device_name={}&timestamp={}&data={}", dname, ts, dv)
                } else {
                    let dv = crate::web::jval(&data_str, "data").unwrap_or_default();
                    format!("data={}", dv)
                };

                let stats_c = stats.clone();
                let cmd_tx_c = cmd_tx.clone();
                tokio::task::spawn_blocking(move || {
                    let result = if is_get {
                        let sep = if url.contains('?') { "&" } else { "?" };
                        agent.get(&format!("{}{}{}", url, sep, get_query)).call()
                    } else {
                        agent.post(&url)
                            .set("Content-Type", "application/json")
                            .send_string(&data_str)
                    };
                    match result {
                        Ok(resp) => {
                            stats_c.http_sent.fetch_add(1, Ordering::Relaxed);
                            // Đọc response body (giới hạn 10KB, tránh OOM nếu server trả HTML lớn)
                            let mut body = String::new();
                            use std::io::Read;
                            if resp.into_reader().take(10240)
                                .read_to_string(&mut body).is_ok() {
                                let trimmed = body.trim();
                                if !trimmed.is_empty() {
                                    let cmd = if let Some(cmd) = crate::commands::parse_json_command(trimmed) {
                                        cmd
                                    } else {
                                        Command::UartTx { data: trimmed.to_string() }
                                    };
                                    let _ = cmd_tx_c.blocking_send(cmd);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("[HTTP] {} thất bại: {}", if is_get { "GET" } else { "POST" }, e);
                            stats_c.http_failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        }
    }
}
