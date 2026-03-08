//! WebSocket handler dùng tungstenite
//! Vì tiny-http upgrade trả Box<dyn ReadWrite> (không split được),
//! dùng 1 thread duy nhất + set read timeout ngắn để luân phiên đọc/ghi

use crate::commands::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tungstenite::{Message, WebSocket};

/// Quản lý tất cả WebSocket connections
pub struct WsManager {
    pub broadcast_tx: broadcast::Sender<String>,
    pub cmd_tx: std::sync::mpsc::Sender<Command>,
    pub connections: AtomicU8,
    pub max_connections: u8,
}

impl WsManager {
    pub fn new(cmd_tx: std::sync::mpsc::Sender<Command>, max_conn: u8) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        Self {
            broadcast_tx,
            cmd_tx,
            connections: AtomicU8::new(0),
            max_connections: max_conn,
        }
    }

    /// Broadcast dữ liệu tới tất cả WS clients
    #[allow(dead_code)]
    pub fn broadcast(&self, data: String) {
        let _ = self.broadcast_tx.send(data);
    }
}

/// Wrapper để set read timeout trên raw fd (nếu stream là socket)
#[allow(dead_code)]
fn try_set_read_timeout(stream: &dyn std::any::Any, timeout: std::time::Duration) {
    // Thử downcast sang TcpStream để set timeout
    if let Some(tcp) = stream.downcast_ref::<std::net::TcpStream>() {
        let _ = tcp.set_read_timeout(Some(timeout));
    }
}

/// Timeout cho idle WS connection (không nhận broadcast nào trong 120s → đóng)
const WS_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Xử lý 1 WebSocket connection (tiny-http đã gửi 101 Upgrade)
/// Single-thread: luân phiên gửi broadcast data + đọc lệnh từ client
pub fn handle_websocket<S>(stream: S, manager: Arc<WsManager>)
where
    S: std::io::Read + std::io::Write + Send + 'static,
{
    // H6 fix: fetch_add trước, rollback nếu vượt giới hạn (tránh TOCTOU race)
    let prev = manager.connections.fetch_add(1, Ordering::Relaxed);
    if prev >= manager.max_connections {
        manager.connections.fetch_sub(1, Ordering::Relaxed);
        log::warn!("[WS] Từ chối kết nối (đã đạt giới hạn {})", manager.max_connections);
        return;
    }

    // tiny-http đã gửi 101 Upgrade → from_raw_socket (không handshake lại)
    let mut ws: WebSocket<S> = WebSocket::from_raw_socket(
        stream,
        tungstenite::protocol::Role::Server,
        None,
    );

    log::info!("[WS] Kết nối mới (tổng: {})", manager.connections.load(Ordering::Relaxed));

    let mut broadcast_rx = manager.broadcast_tx.subscribe();
    let _cmd_tx = manager.cmd_tx.clone();
    let mut last_activity = std::time::Instant::now();

    // Single-thread loop: gửi broadcast trước, rồi thử đọc (non-blocking)
    loop {
        // Kiểm tra idle timeout
        if last_activity.elapsed() > WS_IDLE_TIMEOUT {
            log::info!("[WS] Đóng kết nối do idle timeout");
            let _ = ws.close(None);
            break;
        }

        // 1. Gửi tất cả pending broadcast messages
        let mut sent = 0;
        let mut send_failed = false;
        loop {
            match broadcast_rx.try_recv() {
                Ok(data) => {
                    if ws.send(Message::Text(data)).is_err() {
                        send_failed = true;
                        break;
                    }
                    sent += 1;
                    last_activity = std::time::Instant::now();
                }
                Err(broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Closed) => {
                    let _ = ws.close(None);
                    manager.connections.fetch_sub(1, Ordering::Relaxed);
                    return;
                }
            }
        }

        // Send fail → client đã disconnect, thoát ngay
        if send_failed {
            break;
        }

        // 2. Không có broadcast data → sleep ngắn để tránh busy-loop
        if sent == 0 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    manager.connections.fetch_sub(1, Ordering::Relaxed);
    log::info!("[WS] Ngắt kết nối (còn: {})", manager.connections.load(Ordering::Relaxed));
}
