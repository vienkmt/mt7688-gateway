//! Bộ kết nối lại với exponential backoff
//! Dùng cho TCP client và các kết nối cần tự động thử lại khi mất

use std::time::Duration;

pub struct Reconnector {
    delay: Duration,
    min_delay: Duration,
    max_delay: Duration,
    attempts: u32,
}

impl Reconnector {
    pub fn new(min: Duration, max: Duration) -> Self {
        Self {
            delay: min,
            min_delay: min,
            max_delay: max,
            attempts: 0,
        }
    }

    /// Trả về thời gian chờ tiếp theo, tăng gấp đôi mỗi lần (tối đa max_delay)
    pub fn next_delay(&mut self) -> Duration {
        self.attempts += 1;
        let current = self.delay;
        self.delay = (self.delay * 2).min(self.max_delay);
        current
    }

    /// Reset về trạng thái ban đầu khi kết nối thành công
    pub fn reset(&mut self) {
        self.delay = self.min_delay;
        self.attempts = 0;
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let mut r = Reconnector::new(Duration::from_secs(1), Duration::from_secs(8));
        assert_eq!(r.next_delay(), Duration::from_secs(1));
        assert_eq!(r.next_delay(), Duration::from_secs(2));
        assert_eq!(r.next_delay(), Duration::from_secs(4));
        assert_eq!(r.next_delay(), Duration::from_secs(8)); // max
        assert_eq!(r.next_delay(), Duration::from_secs(8)); // vẫn max
        assert_eq!(r.attempts(), 5);
    }

    #[test]
    fn test_reset() {
        let mut r = Reconnector::new(Duration::from_secs(1), Duration::from_secs(60));
        r.next_delay();
        r.next_delay();
        r.reset();
        assert_eq!(r.next_delay(), Duration::from_secs(1));
        assert_eq!(r.attempts(), 1);
    }
}
