//! ugate - IoT Gateway cho OpenWrt
//! Phần mềm gateway chạy trên MT7688 (MIPS 580MHz, 64MB RAM)
//! Đọc dữ liệu UART từ MCU, fan-out qua MQTT/HTTP/TCP, điều khiển GPIO
//! Web UI quản lý cấu hình qua trình duyệt

mod channels;
mod commands;
mod config;
mod gpio;
mod time_sync;
mod uart;
mod uci;
mod web;

use config::{AppState, Config};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Khởi tạo syslog logger — log tới /dev/log (OpenWrt logread)
    syslog::init(
        syslog::Facility::LOG_DAEMON,
        log::LevelFilter::Info,
        Some("ugate"),
    ).ok();

    log::info!("ugate v{} khởi động...", env!("CARGO_PKG_VERSION"));

    // Đồng bộ đồng hồ hệ thống trước khi kết nối TLS (cần time chính xác cho cert)
    time_sync::sync_time();

    // Nạp cấu hình từ UCI (/etc/config/ugate)
    let config = Config::load();
    let state = Arc::new(AppState::new(config.clone()));

    // Bộ đếm thống kê chia sẻ giữa tất cả tasks
    let stats = Arc::new(web::status::SharedStats::new());

    // --- Hạ tầng kênh truyền ---

    // Broadcast UART: phân phối frame thô tới tất cả subscriber
    let (uart_broadcast_tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(64);

    // Kênh MQTT: std mpsc (MQTT chạy trên OS thread riêng)
    let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<Vec<u8>>();

    // Kênh HTTP POST: async
    let (http_tx, http_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    // Kênh lệnh: gộp từ WS/TCP/MQTT → dispatcher → GPIO + UART TX
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<commands::Command>(32);

    // Kênh nội bộ: dispatcher → GPIO
    let (gpio_tx, gpio_rx) = tokio::sync::mpsc::channel::<commands::Command>(32);

    // Kênh lệnh WS (std mpsc cho WebSocket thread)
    let (ws_cmd_tx, _ws_cmd_rx) = std::sync::mpsc::channel::<commands::Command>();

    // Thông báo thay đổi config cho MQTT thread
    let (config_notify_tx, config_notify_rx) = std::sync::mpsc::channel::<()>();
    state.set_mqtt_notifier(config_notify_tx);

    // Kênh MQTT subscribe → cmd (std mpsc vì MQTT chạy trên OS thread)
    let (mqtt_cmd_tx, mqtt_cmd_rx) = std::sync::mpsc::channel::<commands::Command>();

    // --- Khởi chạy MQTT publisher + subscriber (OS thread riêng) ---
    let mqtt_state = state.clone();
    let mqtt_stats = stats.clone();
    std::thread::spawn(move || {
        channels::mqtt::run_sync(mqtt_state, mqtt_rx, config_notify_rx, mqtt_cmd_tx, mqtt_stats);
    });

    // --- Khởi chạy HTTP publisher (async) ---
    tokio::spawn(channels::http_pub::run(state.clone(), http_rx, cmd_tx.clone(), stats.clone()));

    // --- Khởi chạy TCP Server + Client ---
    tokio::spawn(channels::tcp::run_server(
        state.clone(),
        uart_broadcast_tx.subscribe(),
        cmd_tx.clone(),
        stats.clone(),
    ));
    tokio::spawn(channels::tcp::run_client(
        state.clone(),
        uart_broadcast_tx.subscribe(),
        cmd_tx.clone(),
        stats.clone(),
    ));

    // --- Khởi chạy GPIO controller ---
    tokio::spawn(gpio::run(config.gpio.clone(), gpio_rx, stats.clone()));

    // --- WebSocket manager ---
    let ws_manager = Arc::new(web::ws::WsManager::new(
        ws_cmd_tx,
        config.web.max_ws_connections,
    ));

    // --- Command dispatcher: phân phối lệnh từ tất cả nguồn → GPIO + UART TX ---
    let dispatch_state = state.clone();
    let dispatch_stats = stats.clone();
    let dispatch_ws_broadcast = ws_manager.broadcast_tx.clone();
    tokio::spawn(async move {
        // Mở UART writer cho chiều gửi xuống MCU
        let cfg = dispatch_state.get();
        let uart_port = cfg.uart.port.clone();
        let uart_baud = cfg.uart.baudrate;
        let mut uart_writer = match uart::writer::UartWriter::new(&uart_port, uart_baud) {
            Ok(w) => {
                log::info!("[Dispatch] UART TX sẵn sàng: {}", uart_port);
                Some(w)
            }
            Err(e) => {
                log::warn!("[Dispatch] Không mở UART TX {}: {} (chỉ GPIO)", uart_port, e);
                None
            }
        };

        let mut cmd_rx = cmd_rx;
        loop {
            // Nhận command từ async channel (TCP/WS/HTTP) hoặc MQTT std channel
            let cmd = tokio::select! {
                Some(cmd) = cmd_rx.recv() => cmd,
                // Poll MQTT subscribe commands (std mpsc → async bridge)
                _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                    while let Ok(cmd) = mqtt_cmd_rx.try_recv() {
                        dispatch_command(&cmd, &gpio_tx, &mut uart_writer, &dispatch_stats, &dispatch_ws_broadcast).await;
                    }
                    continue;
                }
            };
            dispatch_command(&cmd, &gpio_tx, &mut uart_writer, &dispatch_stats, &dispatch_ws_broadcast).await;
        }
    });

    // --- HTTP server ---
    let session_mgr = Arc::new(web::auth::SessionManager::new());

    // Status broadcast thread: push trạng thái mỗi 1 giây qua WebSocket
    let ws_broadcast = ws_manager.broadcast_tx.clone();
    let status_stats = stats.clone();
    let status_state = state.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let cfg = status_state.get();
            let json = status_stats.to_status_json(&cfg);
            let _ = ws_broadcast.send(json);
        }
    });

    // Fan-out UART data tới WS clients
    let ws_broadcast_uart = ws_manager.broadcast_tx.clone();
    let mut uart_ws_rx = uart_broadcast_tx.subscribe();
    tokio::spawn(async move {
        loop {
            match uart_ws_rx.recv().await {
                Ok(data) => {
                    // Gửi UART data dạng hex tới WS
                    let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                    let json = format!(r#"{{"type":"uart","dir":"rx","hex":"{}","len":{}}}"#, hex, data.len());
                    let _ = ws_broadcast_uart.send(json);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // --- Fan-out: broadcast UART → MQTT + HTTP ---
    let mut uart_rx = uart_broadcast_tx.subscribe();
    let fanout_state = state.clone();
    tokio::spawn(async move {
        loop {
            match uart_rx.recv().await {
                Ok(data) => {
                    let cfg = fanout_state.get();
                    let payload = if cfg.general.wrap_json {
                        // Wrap raw data thành JSON với metadata
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        // data_as_text: gửi string (UTF-8), ngược lại hex encode
                        let data_str = if cfg.general.data_as_text {
                            match std::str::from_utf8(&data) {
                                Ok(s) => crate::web::json_escape(s),
                                // Fallback hex nếu không phải UTF-8
                                Err(_) => data.iter().map(|b| format!("{:02x}", b)).collect(),
                            }
                        } else {
                            data.iter().map(|b| format!("{:02x}", b)).collect()
                        };
                        let json = format!(
                            r#"{{"device_name":"{}","timestamp":{},"data":"{}"}}"#,
                            crate::web::json_escape(&cfg.general.device_name), ts, data_str
                        );
                        json.into_bytes()
                    } else {
                        data
                    };
                    let _ = mqtt_tx.send(payload.clone());
                    let _ = http_tx.try_send(payload);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("[FanOut] Bỏ qua {} message (quá tải)", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // --- Khởi chạy UART reader ---
    tokio::spawn(uart::reader::run(state.clone(), uart_broadcast_tx, stats.clone()));

    // --- Khởi chạy HTTP server (blocking, spawn_blocking) ---
    let server_state = state.clone();
    let server_ws = ws_manager.clone();
    let server_session = session_mgr.clone();
    tokio::task::spawn_blocking(move || {
        web::server::run(server_state, server_ws, server_session);
    });

    log::info!("ugate v{} đang chạy (tất cả kênh sẵn sàng)", env!("CARGO_PKG_VERSION"));

    // Graceful shutdown
    tokio::signal::ctrl_c().await.ok();
    log::info!("ugate đang tắt...");
}

/// Phân phối command tới đích phù hợp: GPIO hoặc UART TX
async fn dispatch_command(
    cmd: &commands::Command,
    gpio_tx: &tokio::sync::mpsc::Sender<commands::Command>,
    uart_writer: &mut Option<uart::writer::UartWriter>,
    stats: &web::status::SharedStats,
    ws_broadcast: &broadcast::Sender<String>,
) {
    match cmd {
        commands::Command::Gpio { .. } => {
            let _ = gpio_tx.send(cmd.clone()).await;
        }
        commands::Command::UartTx { data } => {
            let bytes = data.as_bytes();
            let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            if let Some(ref mut writer) = uart_writer {
                match writer.write(bytes) {
                    Ok(()) => {
                        log::info!("[Dispatch] UART TX: {} bytes", bytes.len());
                        stats.uart_tx_bytes.fetch_add(bytes.len() as u32, std::sync::atomic::Ordering::Relaxed);
                        stats.uart_tx_frames.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let json = format!(r#"{{"type":"uart","dir":"tx","hex":"{}","len":{}}}"#, hex, bytes.len());
                        let _ = ws_broadcast.send(json);
                    }
                    Err(e) => {
                        log::error!("[Dispatch] UART TX lỗi: {}", e);
                        let json = format!(r#"{{"type":"uart","dir":"tx","hex":"{}","len":{},"err":"write failed"}}"#, hex, bytes.len());
                        let _ = ws_broadcast.send(json);
                    }
                }
            } else {
                log::warn!("[Dispatch] UART TX không sẵn sàng, bỏ qua");
                let json = format!(r#"{{"type":"uart","dir":"tx","hex":"{}","len":{},"err":"uart not ready"}}"#, hex, bytes.len());
                let _ = ws_broadcast.send(json);
            }
        }
    }
}
