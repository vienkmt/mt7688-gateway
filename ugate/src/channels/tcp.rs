//! Kênh TCP Server + Client (song hướng)
//! Server: lắng nghe kết nối, nhận lệnh và gửi dữ liệu UART
//! Client: kết nối tới remote server, tự động reconnect với exponential backoff
//! Dữ liệu nhận từ TCP được parse thành Command (GPIO, UART TX)

use crate::channels::reconnect::Reconnector;
use crate::commands::{self, Command};
use crate::config::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, watch};

/// Chạy TCP Server: lắng nghe kết nối, gửi dữ liệu UART và nhận lệnh
pub async fn run_server(
    state: Arc<AppState>,
    uart_rx: broadcast::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<Command>,
    stats: Arc<crate::web::status::SharedStats>,
) {
    let mut config_watch = state.subscribe();
    let uart_rx = uart_rx;

    loop {
        let config = state.get();
        if !config.tcp.enabled || config.tcp.mode == crate::config::TcpMode::Client {
            // Chỉ set disabled nếu cả server+client đều không chạy
            if !config.tcp.enabled {
                stats.tcp_state.store(0, std::sync::atomic::Ordering::Relaxed);
            }
            tokio::select! {
                _ = config_watch.changed() => {}
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
            continue;
        }

        let addr = format!("0.0.0.0:{}", config.tcp.server_port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => {
                log::info!("[TCP Server] Lắng nghe tại {}", addr);
                stats.tcp_state.store(1, std::sync::atomic::Ordering::Relaxed);
                l
            }
            Err(e) => {
                log::error!("[TCP Server] Không thể bind {}: {}", addr, e);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        loop {
            tokio::select! {
                _ = config_watch.changed() => {
                    log::info!("[TCP Server] Config thay đổi, khởi động lại...");
                    break;
                }

                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            log::info!("[TCP Server] Kết nối từ {}", addr);
                            stats.tcp_state.store(2, std::sync::atomic::Ordering::Relaxed);
                            stats.tcp_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let cmd_tx = cmd_tx.clone();
                            let conn_uart_rx = uart_rx.resubscribe();
                            let conn_shutdown = state.subscribe();
                            tokio::spawn(handle_connection(stream, conn_uart_rx, cmd_tx, conn_shutdown));
                        }
                        Err(e) => {
                            log::error!("[TCP Server] Accept lỗi: {}", e);
                        }
                    }
                }
            }
        }
    }
}

/// Chạy TCP Client: kết nối tới remote server, tự reconnect
pub async fn run_client(
    state: Arc<AppState>,
    uart_rx: broadcast::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<Command>,
    stats: Arc<crate::web::status::SharedStats>,
) {
    let mut reconnector = Reconnector::new(Duration::from_secs(1), Duration::from_secs(60));
    let mut config_watch = state.subscribe();

    loop {
        let config = state.get();
        if !config.tcp.enabled || config.tcp.mode == crate::config::TcpMode::Server {
            tokio::select! {
                _ = config_watch.changed() => {}
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
            continue;
        }

        let addr = format!("{}:{}", config.tcp.client_host, config.tcp.client_port);
        stats.tcp_state.store(1, std::sync::atomic::Ordering::Relaxed); // waiting
        log::info!("[TCP Client] Đang kết nối tới {}...", addr);

        tokio::select! {
            _ = config_watch.changed() => {
                log::info!("[TCP Client] Config thay đổi");
                continue;
            }

            result = TcpStream::connect(&addr) => {
                match result {
                    Ok(stream) => {
                        log::info!("[TCP Client] Đã kết nối {}", addr);
                        stats.tcp_state.store(2, std::sync::atomic::Ordering::Relaxed);
                        stats.tcp_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        reconnector.reset();
                        let conn_shutdown = state.subscribe();
                        handle_connection(stream, uart_rx.resubscribe(), cmd_tx.clone(), conn_shutdown).await;
                        stats.tcp_connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        stats.tcp_state.store(1, std::sync::atomic::Ordering::Relaxed);
                        log::warn!("[TCP Client] Mất kết nối {}", addr);
                    }
                    Err(e) => {
                        let delay = reconnector.next_delay();
                        log::warn!("[TCP Client] Kết nối thất bại: {}. Thử lại sau {:?} (lần {})",
                            e, delay, reconnector.attempts());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }
}

/// Xử lý 1 kết nối TCP: đọc dữ liệu + gửi dữ liệu UART
async fn handle_connection(
    stream: TcpStream,
    mut uart_rx: broadcast::Receiver<Vec<u8>>,
    cmd_tx: mpsc::Sender<Command>,
    mut shutdown_rx: watch::Receiver<()>,
) {
    let (mut reader, mut writer) = stream.into_split();
    let mut buf = vec![0u8; 1024];

    loop {
        tokio::select! {
            // Dừng khi config thay đổi
            _ = shutdown_rx.changed() => {
                log::info!("[TCP] Config thay đổi, đóng kết nối");
                break;
            }

            // Đọc dữ liệu từ TCP (raw bytes, không cần newline)
            result = tokio::io::AsyncReadExt::read(&mut reader, &mut buf) => {
                match result {
                    Ok(0) => break, // Kết nối đóng
                    Ok(n) => {
                        let received = String::from_utf8_lossy(&buf[..n]);
                        let trimmed = received.trim();
                        if !trimmed.is_empty() {
                            let cmd = if let Some(cmd) = commands::parse_json_command(trimmed) {
                                cmd
                            } else {
                                // Không phải JSON command → gửi raw xuống UART
                                Command::UartTx { data: trimmed.to_string() }
                            };
                            log::info!("[TCP] Nhận {} bytes → {:?}", n, cmd);
                            let _ = cmd_tx.send(cmd).await;
                        }
                    }
                    Err(e) => {
                        log::error!("[TCP] Lỗi đọc: {}", e);
                        break;
                    }
                }
            }

            // Gửi dữ liệu UART tới TCP client
            result = uart_rx.recv() => {
                match result {
                    Ok(data) => {
                        if writer.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("[TCP] Bỏ qua {} message", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}
