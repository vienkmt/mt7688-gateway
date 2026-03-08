//! Bộ đệm offline: lưu dữ liệu khi mất kết nối
//! RAM queue → tràn thì ghi ra disk (/tmp/ugate_buffer/)
//! Khi kết nối lại, đọc từ disk trước rồi mới tới RAM
#![allow(dead_code, unused_imports)]

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::PathBuf;

pub struct OfflineBuffer {
    ram_queue: VecDeque<Vec<u8>>,
    ram_limit: usize,
    disk_path: PathBuf,
}

impl OfflineBuffer {
    pub fn new(ram_limit: usize, disk_path: PathBuf) -> Self {
        // Tạo thư mục buffer nếu chưa có
        let _ = std::fs::create_dir_all(&disk_path);
        Self {
            ram_queue: VecDeque::new(),
            ram_limit,
            disk_path,
        }
    }

    /// Thêm message vào buffer. Nếu RAM đầy → ghi ra disk
    pub fn push(&mut self, msg: Vec<u8>) {
        if self.ram_queue.len() < self.ram_limit {
            self.ram_queue.push_back(msg);
        } else {
            self.write_to_disk(&msg);
        }
    }

    /// Lấy message ra: ưu tiên disk trước (FIFO), sau đó RAM
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        // Đọc từ disk trước (dữ liệu cũ hơn)
        if let Some(data) = self.read_one_from_disk() {
            return Some(data);
        }
        self.ram_queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.ram_queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ram_queue.is_empty() && !self.disk_file().exists()
    }

    /// Ghi tất cả RAM queue ra disk (khi shutdown)
    pub fn flush_to_disk(&mut self) {
        while let Some(msg) = self.ram_queue.pop_front() {
            self.write_to_disk(&msg);
        }
    }

    /// Nạp dữ liệu từ disk vào RAM khi khởi động
    pub fn load_from_disk(&mut self) -> usize {
        let path = self.disk_file();
        if !path.exists() {
            return 0;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        let mut count = 0;
        let mut remaining = Vec::new();
        for line in content.lines() {
            if line.is_empty() { continue; }
            if let Ok(data) = hex_decode(line) {
                if self.ram_queue.len() < self.ram_limit {
                    self.ram_queue.push_back(data);
                    count += 1;
                } else {
                    remaining.push(line.to_string());
                }
            }
        }
        // Xoá file hoặc ghi lại phần chưa đọc
        if remaining.is_empty() {
            let _ = std::fs::remove_file(&path);
        } else {
            let _ = std::fs::write(&path, remaining.join("\n") + "\n");
        }
        count
    }

    fn disk_file(&self) -> PathBuf {
        self.disk_path.join("buffer.hex")
    }

    /// Ghi 1 message ra disk dạng hex (1 dòng = 1 message)
    fn write_to_disk(&self, msg: &[u8]) {
        let path = self.disk_file();
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let hex = hex_encode(msg);
            let _ = writeln!(file, "{}", hex);
        }
    }

    /// Đọc 1 dòng đầu tiên từ disk, ghi lại phần còn lại
    fn read_one_from_disk(&mut self) -> Option<Vec<u8>> {
        let path = self.disk_file();
        if !path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        let mut lines = content.lines();
        let first = lines.next()?;
        let data = hex_decode(first).ok()?;

        // Ghi lại phần còn lại
        let rest: Vec<&str> = lines.collect();
        if rest.is_empty() {
            let _ = std::fs::remove_file(&path);
        } else {
            let _ = std::fs::write(&path, rest.join("\n") + "\n");
        }

        Some(data)
    }
}

/// Encode bytes thành hex string (không phụ thuộc crate hex)
fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode hex string thành bytes
fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Tạo thư mục tạm riêng cho mỗi test (tránh xung đột khi chạy song song)
    fn unique_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ugate_test_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_ram_buffer() {
        let dir = unique_dir("ram");
        let mut buf = OfflineBuffer::new(3, dir.clone());
        buf.push(vec![1, 2, 3]);
        buf.push(vec![4, 5, 6]);
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.pop(), Some(vec![1, 2, 3]));
        assert_eq!(buf.pop(), Some(vec![4, 5, 6]));
        assert_eq!(buf.pop(), None);
        cleanup(&dir);
    }

    #[test]
    fn test_disk_overflow() {
        let dir = unique_dir("overflow");
        let mut buf = OfflineBuffer::new(2, dir.clone());
        // 2 vào RAM, 1 tràn ra disk
        buf.push(vec![1]);
        buf.push(vec![2]);
        buf.push(vec![3]); // → disk

        assert_eq!(buf.len(), 2); // Chỉ đếm RAM
        // pop đọc disk trước (dữ liệu cũ overflow), rồi RAM
        assert_eq!(buf.pop(), Some(vec![3])); // từ disk
        assert_eq!(buf.pop(), Some(vec![1])); // từ RAM
        assert_eq!(buf.pop(), Some(vec![2])); // từ RAM
        assert_eq!(buf.pop(), None);
        cleanup(&dir);
    }

    #[test]
    fn test_flush_and_load() {
        let dir = unique_dir("flush");
        let mut buf = OfflineBuffer::new(10, dir.clone());
        buf.push(vec![0xAA, 0xBB]);
        buf.push(vec![0xCC, 0xDD]);
        buf.flush_to_disk();
        assert_eq!(buf.len(), 0);

        // Tạo buffer mới, load từ disk
        let mut buf2 = OfflineBuffer::new(10, dir.clone());
        let loaded = buf2.load_from_disk();
        assert_eq!(loaded, 2);
        assert_eq!(buf2.pop(), Some(vec![0xAA, 0xBB]));
        assert_eq!(buf2.pop(), Some(vec![0xCC, 0xDD]));
        cleanup(&dir);
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = vec![0x01, 0xFF, 0x00, 0xAB];
        let hex = hex_encode(&data);
        assert_eq!(hex, "01ff00ab");
        assert_eq!(hex_decode(&hex), Ok(data));
    }
}
