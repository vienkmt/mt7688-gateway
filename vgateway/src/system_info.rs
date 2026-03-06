use std::fs;
use std::sync::OnceLock;

/// Cached external IP (fetched once via HTTPS, no OpenSSL needed - pure rustls)
static EXTERNAL_IP: OnceLock<String> = OnceLock::new();

/// System statistics collected from /proc filesystem
pub struct SystemInfo {
    pub uptime_secs: f64,
    pub ram_total_mb: f64,
    pub ram_available_mb: f64,
    pub ram_used_mb: f64,
    pub ram_buffered_mb: f64,
    pub ram_cached_mb: f64,
    pub disk_used: String,
    pub disk_total: String,
    pub disk_percent: u8,
    pub ip_address: String,
    pub external_ip: String,
    pub net_rx: String,
    pub net_tx: String,
    pub processes: usize,
    pub kernel: String,
    pub local_time: String,
}

impl SystemInfo {
    /// Format as JSON payload for MQTT/HTTP publishing
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"type":"monitor","uptime":{:.2},"ram_used":{:.1},"ram_total":{:.1},"disk_used":"{}","disk_total":"{}","disk_pct":{},"ip":"{}","ext_ip":"{}","net_rx":"{}","net_tx":"{}","procs":{}}}"#,
            self.uptime_secs, self.ram_used_mb, self.ram_total_mb,
            self.disk_used, self.disk_total, self.disk_percent,
            self.ip_address, self.external_ip, self.net_rx, self.net_tx,
            self.processes,
        )
    }

    /// Collect all system stats from /proc
    pub fn collect() -> Self {
        let mem = read_meminfo_detailed();
        let (disk_used, disk_total, disk_percent) = read_disk_usage();
        let (net_rx, net_tx) = read_network();
        Self {
            uptime_secs: read_uptime(),
            ram_total_mb: mem.total,
            ram_available_mb: mem.available,
            ram_used_mb: mem.used,
            ram_buffered_mb: mem.buffered,
            ram_cached_mb: mem.cached,
            disk_used,
            disk_total,
            disk_percent,
            ip_address: read_ip_address(),
            external_ip: EXTERNAL_IP.get_or_init(fetch_external_ip).clone(),
            net_rx,
            net_tx,
            processes: count_processes(),
            kernel: read_kernel_version(),
            local_time: read_local_time(),
        }
    }
}

/// Read uptime seconds from /proc/uptime
fn read_uptime() -> f64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse().ok())
        .unwrap_or(0.0)
}

struct MemInfo { total: f64, available: f64, used: f64, buffered: f64, cached: f64 }

/// Read detailed RAM info from /proc/meminfo (in MB)
fn read_meminfo_detailed() -> MemInfo {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0u64;
    let mut available = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    for line in content.lines() {
        if line.starts_with("MemTotal:") { total = parse_kb(line); }
        else if line.starts_with("MemAvailable:") { available = parse_kb(line); }
        else if line.starts_with("Buffers:") { buffers = parse_kb(line); }
        else if line.starts_with("Cached:") && !line.starts_with("SwapCached:") { cached = parse_kb(line); }
    }
    let used = total.saturating_sub(available);
    MemInfo {
        total: total as f64 / 1024.0,
        available: available as f64 / 1024.0,
        used: used as f64 / 1024.0,
        buffered: buffers as f64 / 1024.0,
        cached: cached as f64 / 1024.0,
    }
}

fn parse_kb(line: &str) -> u64 {
    line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn read_kernel_version() -> String {
    fs::read_to_string("/proc/version")
        .ok()
        .and_then(|s| s.split_whitespace().nth(2).map(|v| v.to_string()))
        .unwrap_or_else(|| "Unknown".into())
}

fn read_local_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // UTC+7 offset
    let local = secs + 7 * 3600;
    let days = local / 86400;
    let time = local % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    // Simple date calc from days since 1970
    let (y, mo, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, m, s)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    let mut rem = days;
    loop {
        let leap = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if rem < leap { break; }
        rem -= leap;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [31, if leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 1;
    for &md in &mdays {
        if rem < md { break; }
        rem -= md;
        mo += 1;
    }
    (y, mo, rem + 1)
}

/// Read flash usage from /proc/mtd (uboot + kernel + rootfs + overlay)
fn read_disk_usage() -> (String, String, u8) {
    let mut flash_total = 0u64;
    let mut fixed_used = 0u64; // uboot + kernel + rootfs (100% used)

    // Parse /proc/mtd for all partitions
    if let Ok(content) = fs::read_to_string("/proc/mtd") {
        for line in content.lines().skip(1) { // skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Ok(size) = u64::from_str_radix(parts[1], 16) {
                    flash_total += size;
                    let name = parts[3].trim_matches('"');
                    // These partitions are fixed/read-only
                    if name != "rootfs_data" && name != "firmware" {
                        fixed_used += size;
                    }
                }
            }
        }
    }

    // Add overlay used space (user data in rootfs_data/jffs2)
    let mut overlay_used = 0u64;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(b"/overlay\0".as_ptr() as *const libc::c_char, &mut stat) == 0 && stat.f_blocks > 0 {
            let bsize = stat.f_bsize as u64;
            let total = stat.f_blocks as u64 * bsize;
            let free = stat.f_bfree as u64 * bsize;
            overlay_used = total.saturating_sub(free);
        }
    }

    let total_used = fixed_used + overlay_used;
    if flash_total > 0 {
        let pct = ((total_used * 100) / flash_total).min(100) as u8;
        (format_bytes(total_used), format_bytes(flash_total), pct)
    } else {
        ("N/A".into(), "N/A".into(), 0)
    }
}

/// Read network RX/TX bytes from /proc/net/dev (first non-lo interface)
fn read_network() -> (String, String) {
    let content = fs::read_to_string("/proc/net/dev").unwrap_or_default();
    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let iface = parts[0].trim();
        if iface == "lo" {
            continue;
        }
        let vals: Vec<&str> = parts[1].split_whitespace().collect();
        if vals.len() >= 10 {
            let rx = vals[0].parse::<u64>().unwrap_or(0);
            let tx = vals[8].parse::<u64>().unwrap_or(0);
            return (format_bytes(rx), format_bytes(tx));
        }
    }
    ("0KB".into(), "0KB".into())
}

/// Read IP address + prefix from /proc/net/fib_trie, fallback to UDP socket trick
fn read_ip_address() -> String {
    if let Ok(content) = fs::read_to_string("/proc/net/fib_trie") {
        let lines: Vec<&str> = content.lines().collect();
        let mut prefix = 24u8;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            // Track subnet prefix from "+-- x.x.x.x/YY" entries
            if let Some(rest) = trimmed.strip_prefix("+--") {
                if let Some(slash) = rest.rfind('/') {
                    if let Ok(p) = rest[slash + 1..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .parse::<u8>()
                    {
                        if p < 32 {
                            prefix = p;
                        }
                    }
                }
            }
            // Find "|-- IP" followed by "/32 host LOCAL"
            if let Some(rest) = trimmed.strip_prefix("|--") {
                let ip = rest.trim();
                if i + 1 < lines.len()
                    && lines[i + 1].trim().contains("host LOCAL")
                    && !ip.starts_with("127.")
                {
                    return format!("{}/{}", ip, prefix);
                }
            }
        }
    }
    // Fallback: UDP socket trick
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "N/A".into())
}

/// Count running processes by counting numeric dirs in /proc
fn count_processes() -> usize {
    fs::read_dir("/proc")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|n| n.chars().all(|c| c.is_ascii_digit()))
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Fetch external IP via HTTPS (rustls, no OpenSSL!) - called once and cached
fn fetch_external_ip() -> String {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .get("https://ifconfig.me")
        .set("User-Agent", "curl/8.0")
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "N/A".into())
}

/// Format byte count as human-readable (KB/M/G)
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1}G", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}M", bytes as f64 / 1_048_576.0)
    } else {
        format!("{}KB", bytes / 1024)
    }
}
