//! Xác thực đơn giản qua password + session cookie
//! Session lưu trong RAM, hết hạn sau TTL hoặc khi restart

use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};
use std::time::Instant;

/// Số session tối đa (thiết bị IoT ít user, 4 là đủ)
const MAX_SESSIONS: usize = 4;
/// Session hết hạn sau 2 giờ (tự logout)
const SESSION_TTL_SECS: u64 = 2 * 3600;

struct Session {
    token: String,
    created: Instant,
}

/// Khoảng cách tối thiểu giữa các lần login fail (chống brute-force)
const LOGIN_COOLDOWN_SECS: u64 = 2;

/// Quản lý session đơn giản (token trong RAM, giới hạn số lượng, TTL)
pub struct SessionManager {
    sessions: RwLock<VecDeque<Session>>,
    last_fail: Mutex<Option<Instant>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(VecDeque::new()),
            last_fail: Mutex::new(None),
        }
    }

    /// Kiểm tra rate limit, trả false nếu đang trong cooldown
    pub fn check_rate_limit(&self) -> bool {
        let last = self.last_fail.lock().unwrap_or_else(|e| e.into_inner());
        match *last {
            Some(t) => t.elapsed().as_secs() >= LOGIN_COOLDOWN_SECS,
            None => true,
        }
    }

    /// Ghi nhận login fail
    pub fn record_fail(&self) {
        let mut last = self.last_fail.lock().unwrap_or_else(|e| e.into_inner());
        *last = Some(Instant::now());
    }

    /// Tạo session mới, trả về token. Xoá session cũ nhất nếu vượt giới hạn.
    pub fn create_session(&self) -> String {
        let token = generate_token();
        let mut sessions = self.sessions.write().unwrap();
        // Xoá session hết hạn trước
        sessions.retain(|s| s.created.elapsed().as_secs() < SESSION_TTL_SECS);
        if sessions.len() >= MAX_SESSIONS {
            sessions.pop_front();
        }
        sessions.push_back(Session { token: token.clone(), created: Instant::now() });
        token
    }

    /// Kiểm tra session hợp lệ (chưa hết hạn)
    pub fn check_session(&self, cookie: Option<&str>) -> bool {
        let token = match cookie {
            Some(c) => extract_token(c),
            None => return false,
        };
        match token {
            Some(t) => self.sessions.read().unwrap().iter().any(|s| {
                s.token == t && s.created.elapsed().as_secs() < SESSION_TTL_SECS
            }),
            None => false,
        }
    }
}

/// Kiểm tra password từ request body (JSON: {"password":"xxx"})
pub fn validate_password(body: &str, expected: &str) -> bool {
    // Parse JSON đơn giản
    if let Some(pos) = body.find("\"password\"") {
        let rest = &body[pos + 10..];
        if let Some(start) = rest.find('"') {
            let val = &rest[start + 1..];
            if let Some(end) = val.find('"') {
                return &val[..end] == expected;
            }
        }
    }
    false
}

/// Tạo token ngẫu nhiên từ /dev/urandom (16 bytes = 32 hex chars)
fn generate_token() -> String {
    use std::io::Read;
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut buf);
    } else {
        // Fallback nếu không đọc được urandom (hiếm khi xảy ra trên Linux)
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        buf[..8].copy_from_slice(&ts.to_le_bytes()[..8]);
        buf[8..12].copy_from_slice(&std::process::id().to_le_bytes());
    }
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Trích xuất token từ Cookie header
fn extract_token(cookie: &str) -> Option<&str> {
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("session=") {
            return Some(val);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_password() {
        assert!(validate_password(r#"{"password":"admin"}"#, "admin"));
        assert!(!validate_password(r#"{"password":"wrong"}"#, "admin"));
    }

    #[test]
    fn test_session_flow() {
        let mgr = SessionManager::new();
        let token = mgr.create_session();
        let cookie = format!("session={}", token);
        assert!(mgr.check_session(Some(&cookie)));
        assert!(!mgr.check_session(Some("session=invalid")));
        assert!(!mgr.check_session(None));
    }
}
