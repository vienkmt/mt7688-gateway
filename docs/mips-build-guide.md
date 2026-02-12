# Hướng dẫn Build Rust cho MIPS (MT7688AN)

## 1. Tổng quan

Dự án này target **MediaTek MT7688AN**, một MIPS 32-bit (MIPS 24KEc @ 580MHz) chạy trên OpenWrt 21.02.

| Thông số | Giá trị |
|----------|--------|
| **Kiến trúc** | MIPS 32-bit (mipsel) |
| **Target triple** | mipsel-unknown-linux-musl |
| **Libc** | musl (static linking) |
| **Compile chạy trên** | macOS / Linux |

## 2. Yêu cầu hệ thống

- **Docker/OrbStack**: Để build cross-compile, cần container Linux
- **Rust nightly**: MIPS target chỉ có sẵn trong nightly channel
- **cross tool**: Tool từ cross-rs để đơn giản hóa cross-compilation
- **~1GB free space**: Cho Docker image + build artifacts

## 3. Cài đặt công cụ

### 3.1 Cài đặt Rust nightly

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

### 3.2 Cài đặt cross

```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### 3.3 Cài đặt Docker/OrbStack

**macOS:**
```bash
# OrbStack (khuyến nghị): https://orbstack.dev
# hoặc Docker Desktop
open /Applications/OrbStack.app
```

**Linux:**
```bash
sudo apt-get install docker.io
sudo usermod -aG docker $USER
```

## 4. Cấu hình dự án

### 4.1 `.cargo/config.toml` (đã có)

```toml
[target.mipsel-unknown-linux-musl]
linker = "mipsel-linux-musl-gcc"

[build]
# target = "mipsel-unknown-linux-musl"
```

### 4.2 `Cross.toml` (đã có)

```toml
[target.mipsel-unknown-linux-musl]
image = "ghcr.io/cross-rs/mipsel-unknown-linux-musl:main"
build-std = ["std", "panic_abort"]
```

**Lưu ý quan trọng:**
- `build-std`: Rebuild std library vì MIPS 32-bit không có precompiled std
- `panic_abort`: Giảm kích thước binary

### 4.3 Chú ý AtomicU64

**MIPS 32-bit KHÔNG hỗ trợ AtomicU64.** Nếu dependency dùng AtomicU64:

```toml
[dependencies]
parking_lot = { version = "0.12", features = ["atomic_from_u64"] }
```

Hoặc dùng **AtomicU32** thay thế trong code:

```rust
use std::sync::atomic::{AtomicU32, Ordering};

let counter = AtomicU32::new(0);
counter.fetch_add(1, Ordering::SeqCst);
```

## 5. Build commands

### 5.1 Build release

```bash
cross +nightly build --target mipsel-unknown-linux-musl --release
```

**Output:**
```
target/mipsel-unknown-linux-musl/release/{binary_name}
```

### 5.2 Kiểm tra binary

```bash
# Kiểm tra architecture
file target/mipsel-unknown-linux-musl/release/{binary_name}
# Kết quả: "ELF 32-bit LSB executable, MIPS, MIPS32 rel2 version 1"

# Kiểm tra kích thước
ls -lh target/mipsel-unknown-linux-musl/release/{binary_name}
# Target: < 500KB
```

### 5.3 Build check nhanh (không compile)

```bash
cargo check --target mipsel-unknown-linux-musl
```

## 6. Deploy lên thiết bị

### 6.1 Transfer binary

```bash
scp target/mipsel-unknown-linux-musl/release/{binary_name} root@10.10.10.1:/tmp/
```

### 6.2 Chạy trên device

```bash
ssh root@10.10.10.1

# Trên device:
chmod +x /tmp/{binary_name}
/tmp/{binary_name}

# Hoặc chạy daemon:
nohup /tmp/{binary_name} > /var/log/gateway.log 2>&1 &
```

### 6.3 Xem logs

```bash
ssh root@10.10.10.1 tail -f /var/log/gateway.log
```

## 7. Troubleshooting

### ❌ "MIPS target not installed"

**Nguyên nhân:** Rust stable không có MIPS target
**Giải pháp:** Dùng nightly với `build-std`

```bash
cross +nightly build --target mipsel-unknown-linux-musl --release
```

### ❌ "Docker daemon not running"

**macOS:**
```bash
# OrbStack
open /Applications/OrbStack.app

# Hoặc Docker Desktop
open /Applications/Docker.app
```

**Linux:**
```bash
sudo systemctl start docker
```

### ❌ "error: target-cpu mips32r2 is not supported"

**Nguyên nhân:** Linker MIPS cấu hình sai
**Giải pháp:** Kiểm tra `Cross.toml` và `.cargo/config.toml`

```toml
# Cross.toml
[target.mipsel-unknown-linux-musl]
image = "ghcr.io/cross-rs/mipsel-unknown-linux-musl:main"
build-std = ["std", "panic_abort"]
```

### ❌ "AtomicU64 not available"

**Nguyên nhân:** MIPS 32-bit không hỗ trợ 64-bit atomics
**Giải pháp:** Thay thế bằng AtomicU32 hoặc `Mutex<u64>`

```rust
// ❌ Không được
use std::sync::atomic::AtomicU64;

// ✅ Đúng
use std::sync::atomic::AtomicU32;
let counter = AtomicU32::new(0);
```

### ❌ Binary size > 500KB

**Nguyên nhân:** Các dependency nặng (tokio, serde, etc.)
**Giải pháp:**
- Dùng `--release` (bắt buộc)
- Enable `strip` trong Cargo.toml:

```toml
[profile.release]
strip = true
```

- Xóa debug symbols:

```bash
mipsel-linux-musl-strip target/mipsel-unknown-linux-musl/release/{binary_name}
```

## 8. Tham khảo

- [cross-rs Documentation](https://github.com/cross-rs/cross)
- [OpenWrt MT7688 Docs](https://openwrt.org/toh/mediatek/linkitsmartmt7688)
- [Rust Embedded Book](https://rust-embedded.github.io/)
- CLAUDE.md (Project constraints)

---

**Last updated:** 2026-02-12
**Version:** 1.0
