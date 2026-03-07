/// Đồng bộ đồng hồ hệ thống từ HTTP Date header (không cần NTP)
/// Dùng HTTP thuần (không HTTPS) để tránh vòng lặp: TLS cần time đúng mới validate cert được
/// PHẢI chạy trước mọi kết nối TLS (MQTT, HTTP POST)
pub fn sync_time() {
    log::info!("[Time] Syncing system clock via HTTP...");

    let resp = match ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .head("http://www.google.com")
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[Time] HTTP request failed: {}", e);
            return;
        }
    };

    let date_str = match resp.header("date") {
        Some(d) => d.to_string(),
        None => {
            log::warn!("[Time] No Date header in response");
            return;
        }
    };

    if let Some(ts) = parse_http_date(&date_str) {
        unsafe {
            let tv = libc::timeval {
                tv_sec: ts as _,
                tv_usec: 0,
            };
            if libc::settimeofday(&tv, std::ptr::null()) == 0 {
                log::info!("[Time] Clock synced: {}", date_str);
            } else {
                log::warn!("[Time] settimeofday failed (not root?)");
            }
        }
    } else {
        log::warn!("[Time] Failed to parse date: {}", date_str);
    }
}

/// Parse HTTP date "Thu, 06 Feb 2026 11:30:00 GMT" -> unix timestamp (UTC)
fn parse_http_date(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let day: u64 = parts[1].parse().ok()?;
    let month = match parts[2] {
        "Jan" => 1, "Feb" => 2, "Mar" => 3, "Apr" => 4,
        "May" => 5, "Jun" => 6, "Jul" => 7, "Aug" => 8,
        "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12,
        _ => return None,
    };
    let year: u64 = parts[3].parse().ok()?;
    let time: Vec<u64> = parts[4].split(':').filter_map(|t| t.parse().ok()).collect();
    if time.len() != 3 {
        return None;
    }

    let mut days = 0u64;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let mdays = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += mdays[m as usize];
    }
    if month > 2 && is_leap(year) {
        days += 1;
    }
    days += day - 1;

    Some(days * 86400 + time[0] * 3600 + time[1] * 60 + time[2])
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
