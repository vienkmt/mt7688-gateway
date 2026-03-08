//! Thu thập trạng thái hệ thống và thống kê các kênh
//! SharedStats dùng atomic counters, chia sẻ giữa tất cả tasks
//! StatusCollector đọc /proc/* để lấy thông tin hệ thống

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Mutex;

/// Snapshot /proc/stat cho tính CPU delta
struct CpuSnapshot {
    idle: u64,
    total: u64,
}

/// Bộ đếm atomic chia sẻ giữa UART, MQTT, TCP, GPIO tasks
pub struct SharedStats {
    cpu_prev: Mutex<Option<CpuSnapshot>>,
    /// MQTT client ID hiện tại (set bởi MQTT thread mỗi lần connect)
    pub mqtt_client_id: Mutex<String>,
    pub uart_rx_bytes: AtomicU32,
    pub uart_rx_frames: AtomicU32,
    pub uart_tx_bytes: AtomicU32,
    pub uart_tx_frames: AtomicU32,
    pub uart_failed: AtomicU32,
    pub mqtt_published: AtomicU32,
    pub mqtt_failed: AtomicU32,
    pub mqtt_state: AtomicU8, // 0=disabled, 1=disconnected, 2=connected
    pub tcp_connections: AtomicU8,
    pub tcp_state: AtomicU8, // 0=disabled, 1=disconnected, 2=connected
    pub http_state: AtomicU8, // 0=disabled, 1=active, 2=error
    pub http_sent: AtomicU32,
    pub http_failed: AtomicU32,
    pub gpio_states: [AtomicU8; 4],
}

impl SharedStats {
    pub fn new() -> Self {
        Self {
            cpu_prev: Mutex::new(None),
            mqtt_client_id: Mutex::new(String::new()),
            uart_rx_bytes: AtomicU32::new(0),
            uart_rx_frames: AtomicU32::new(0),
            uart_tx_bytes: AtomicU32::new(0),
            uart_tx_frames: AtomicU32::new(0),
            uart_failed: AtomicU32::new(0),
            mqtt_published: AtomicU32::new(0),
            mqtt_failed: AtomicU32::new(0),
            mqtt_state: AtomicU8::new(0),
            tcp_connections: AtomicU8::new(0),
            tcp_state: AtomicU8::new(0),
            http_state: AtomicU8::new(0),
            http_sent: AtomicU32::new(0),
            http_failed: AtomicU32::new(0),
            gpio_states: [
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
            ],
        }
    }

    /// Tính CPU% từ delta /proc/stat giữa 2 lần gọi (giống top)
    fn read_cpu_percent(&self) -> u8 {
        let cur = match read_proc_stat() {
            Some(s) => s,
            None => return 0,
        };
        let mut prev_lock = self.cpu_prev.lock().unwrap();
        let pct = if let Some(prev) = prev_lock.as_ref() {
            let d_total = cur.total.saturating_sub(prev.total);
            let d_idle = cur.idle.saturating_sub(prev.idle);
            if d_total == 0 {
                0
            } else {
                (((d_total - d_idle) * 100) / d_total) as u8
            }
        } else {
            0
        };
        *prev_lock = Some(cur);
        pct
    }

    /// Thu thập trạng thái thành JSON string (không dùng serde_json)
    pub fn to_status_json(&self, config: &crate::config::Config) -> String {
        let uptime = read_uptime();
        let datetime = read_datetime();
        let (ram_used, ram_total) = read_mem_info();
        let cpu = self.read_cpu_percent();

        format!(
            r#"{{"type":"status","version":"{}","uptime":"{}","datetime":"{}","cpu":{},"ram_used":{},"ram_total":{},"uart":{{"rx_bytes":{},"rx_frames":{},"tx_bytes":{},"tx_frames":{},"failed":{},"config":"{} 8N1"}},"mqtt":{{"enabled":{},"state":"{}","client_id":"{}","published":{},"failed":{}}},"http":{{"enabled":{},"state":"{}","sent":{},"failed":{}}},"tcp":{{"enabled":{},"state":"{}","connections":{}}},"gpio":[{},{},{},{}]}}"#,
            env!("CARGO_PKG_VERSION"),
            uptime,
            datetime,
            cpu,
            ram_used,
            ram_total,
            self.uart_rx_bytes.load(Ordering::Relaxed),
            self.uart_rx_frames.load(Ordering::Relaxed),
            self.uart_tx_bytes.load(Ordering::Relaxed),
            self.uart_tx_frames.load(Ordering::Relaxed),
            self.uart_failed.load(Ordering::Relaxed),
            config.uart.baudrate,
            config.mqtt.enabled,
            state_str(self.mqtt_state.load(Ordering::Relaxed)),
            self.mqtt_client_id.lock().unwrap(),
            self.mqtt_published.load(Ordering::Relaxed),
            self.mqtt_failed.load(Ordering::Relaxed),
            config.http.enabled,
            state_str(self.http_state.load(Ordering::Relaxed)),
            self.http_sent.load(Ordering::Relaxed),
            self.http_failed.load(Ordering::Relaxed),
            config.tcp.enabled,
            state_str(self.tcp_state.load(Ordering::Relaxed)),
            self.tcp_connections.load(Ordering::Relaxed),
            self.gpio_states[0].load(Ordering::Relaxed) != 0,
            self.gpio_states[1].load(Ordering::Relaxed) != 0,
            self.gpio_states[2].load(Ordering::Relaxed) != 0,
            self.gpio_states[3].load(Ordering::Relaxed) != 0,
        )
    }
}

fn state_str(s: u8) -> &'static str {
    match s {
        0 => "disabled",
        1 => "waiting",
        2 => "connected",
        _ => "unknown",
    }
}

/// Đọc datetime hệ thống via `date` command
fn read_datetime() -> String {
    std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Đọc uptime từ /proc/uptime
fn read_uptime() -> String {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(String::from))
        .and_then(|s| s.parse::<f64>().ok())
        .map(format_uptime)
        .unwrap_or_else(|| "0m".into())
}

fn format_uptime(secs: f64) -> String {
    let s = secs as u64;
    let d = s / 86400;
    let h = (s % 86400) / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if d > 0 {
        format!("{}d {}h {}m {}s", d, h, m, sec)
    } else if h > 0 {
        format!("{}h {}m {}s", h, m, sec)
    } else {
        format!("{}m {}s", m, sec)
    }
}

/// Đọc thông tin RAM từ /proc/meminfo (đơn vị MB)
fn read_mem_info() -> (u16, u16) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut free = 0u64;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total = parse_meminfo_kb(line);
        } else if line.starts_with("MemAvailable:") {
            free = parse_meminfo_kb(line);
        }
    }
    let total_mb = (total / 1024) as u16;
    let used_mb = ((total.saturating_sub(free)) / 1024) as u16;
    (used_mb, total_mb)
}

fn parse_meminfo_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Đọc CPU% từ /proc/stat delta (giống top/htop)
/// Lần gọi đầu trả 0, từ lần 2 trở đi tính delta chính xác
fn read_proc_stat() -> Option<CpuSnapshot> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let cpu_line = content.lines().next()?; // "cpu  user nice system idle ..."
    let vals: Vec<u64> = cpu_line
        .split_whitespace()
        .skip(1) // bỏ "cpu"
        .filter_map(|s| s.parse().ok())
        .collect();
    if vals.len() < 4 {
        return None;
    }
    let idle = vals[3]; // idle + iowait nếu có
    let total: u64 = vals.iter().sum();
    Some(CpuSnapshot { idle, total })
}
