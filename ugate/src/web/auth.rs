//! Xác thực đơn giản qua password + session cookie
//! Session lưu trong RAM, hết hạn khi restart

use std::collections::VecDeque;
use std::sync::RwLock;

/// Số session tối đa (thiết bị IoT ít user, 4 là đủ)
const MAX_SESSIONS: usize = 4;

/// Quản lý session đơn giản (token trong RAM, giới hạn số lượng)
pub struct SessionManager {
    tokens: RwLock<VecDeque<String>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(VecDeque::new()),
        }
    }

    /// Tạo session mới, trả về token. Xoá session cũ nhất nếu vượt giới hạn.
    pub fn create_session(&self) -> String {
        let token = generate_token();
        let mut tokens = self.tokens.write().unwrap();
        if tokens.len() >= MAX_SESSIONS {
            tokens.pop_front(); // xoá session cũ nhất
        }
        tokens.push_back(token.clone());
        token
    }

    /// Kiểm tra session hợp lệ
    pub fn check_session(&self, cookie: Option<&str>) -> bool {
        let token = match cookie {
            Some(c) => extract_token(c),
            None => return false,
        };
        match token {
            Some(t) => self.tokens.read().unwrap().iter().any(|s| s == t),
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

/// Tạo token ngẫu nhiên từ timestamp + pid
fn generate_token() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    format!("{:016x}{:08x}", ts, pid)
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
