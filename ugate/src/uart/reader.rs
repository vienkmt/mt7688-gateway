//! Đọc UART không đồng bộ qua AsyncFd + epoll
//! Đọc byte từ cổng serial, phát hiện frame theo chế độ cấu hình (none/frame/modbus)
//! Phân phối frame hoàn chỉnh tới tất cả kênh qua broadcast channel

use crate::config::{AppState, FrameMode};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::unix::AsyncFd;
use tokio::sync::broadcast;

/// Fan-out UART data to all channel subscribers
pub async fn run(
    state: Arc<AppState>,
    broadcast_tx: broadcast::Sender<Vec<u8>>,
    stats: Arc<crate::web_api::status::SharedStats>,
) {
    let mut retry_secs = 5u64;
    loop {
        let config = state.get();
        if !config.uart.enabled {
            tokio::time::sleep(Duration::from_secs(10)).await;
            retry_secs = 5;
            continue;
        }
        match run_read_loop(&state, &broadcast_tx, &stats).await {
            Ok(()) => retry_secs = 5,
            Err(e) => {
                log::error!("[UART] Error: {}. Retrying in {}s...", e, retry_secs);
                tokio::time::sleep(Duration::from_secs(retry_secs)).await;
                retry_secs = (retry_secs * 2).min(60);
            }
        }
    }
}

async fn run_read_loop(
    state: &AppState,
    broadcast_tx: &broadcast::Sender<Vec<u8>>,
    stats: &crate::web_api::status::SharedStats,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();
    let mut config_rx = state.subscribe();

    // Open serial port with O_NONBLOCK
    let path = std::ffi::CString::new(config.uart.port.as_str())?;
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK) };
    if fd < 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("Cannot open {}: {}", config.uart.port, err).into());
    }

    if unsafe { libc::isatty(fd) } != 1 {
        unsafe { libc::close(fd) };
        return Err(format!("{} is not a TTY device", config.uart.port).into());
    }

    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    configure_serial(&file, config.uart.baudrate)?;

    let async_fd = AsyncFd::new(file)?;
    let mut buffer = Vec::with_capacity(1024);
    let gap_duration = Duration::from_millis(config.uart.gap_ms as u64);

    log::info!("[UART] Opened {} @ {} baud, mode={:?}",
        config.uart.port, config.uart.baudrate, config.uart.frame_mode);

    loop {
        tokio::select! {
            _ = config_rx.changed() => {
                log::info!("[UART] Config changed, reconnecting...");
                return Ok(());
            }

            result = async_fd.readable() => {
                let mut guard = result?;

                match guard.try_io(|inner| read_bytes(inner.get_ref(), &mut buffer)) {
                    Ok(Ok(true)) => {
                        // Got data, check frame completion based on mode
                        let frame = match config.uart.frame_mode {
                            FrameMode::None => {
                                // Gap-based: wait for silence then flush
                                tokio::time::sleep(gap_duration).await;
                                if !buffer.is_empty() {
                                    Some(buffer.drain(..).collect::<Vec<u8>>())
                                } else {
                                    None
                                }
                            }
                            FrameMode::Frame => {
                                if buffer.len() >= config.uart.frame_length as usize {
                                    Some(buffer.drain(..config.uart.frame_length as usize).collect())
                                } else {
                                    None
                                }
                            }
                            FrameMode::Modbus => {
                                // Modbus: gap-based with 3.5T silence detection
                                let gap_3t5 = modbus_gap_ms(config.uart.baudrate);
                                tokio::time::sleep(Duration::from_millis(gap_3t5)).await;
                                if buffer.len() >= 4 {
                                    // Minimum Modbus frame: addr(1) + func(1) + data(?) + crc(2)
                                    let frame_data: Vec<u8> = buffer.drain(..).collect();
                                    if verify_modbus_crc(&frame_data) {
                                        Some(frame_data)
                                    } else {
                                        log::warn!("[UART] Modbus CRC error, dropping {} bytes", frame_data.len());
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                        };

                        if let Some(ref data) = frame {
                            // Lọc noise: bỏ qua frame <= 2 bytes toàn 0x00
                            if data.len() <= 2 && data.iter().all(|&b| b == 0) {
                                continue;
                            }
                            log::debug!("[UART] Frame: {} bytes", data.len());
                            stats.uart_rx_bytes.fetch_add(data.len() as u32, std::sync::atomic::Ordering::Relaxed);
                            stats.uart_rx_frames.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let _ = broadcast_tx.send(data.clone());
                        }

                        // Buffer overflow protection
                        if buffer.len() > 512 {
                            log::warn!("[UART] Buffer overflow, flushing {} bytes", buffer.len());
                            buffer.clear();
                        }
                    }
                    Ok(Ok(false)) => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Ok(Err(e)) => return Err(e.into()),
                    Err(_would_block) => {}
                }
            }
        }
    }
}

/// Read available bytes into buffer, returns true if data was read
fn read_bytes(file: &std::fs::File, buffer: &mut Vec<u8>) -> std::io::Result<bool> {
    use std::io::Read;
    let mut tmp = [0u8; 256];
    match (&*file).read(&mut tmp) {
        Ok(0) => Ok(false),
        Ok(n) => {
            buffer.extend_from_slice(&tmp[..n]);
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(false),
        Err(e) => Err(e),
    }
}

/// Calculate Modbus 3.5T gap in ms based on baudrate
fn modbus_gap_ms(baudrate: u32) -> u64 {
    // 3.5 character times: 3.5 * 11 bits / baudrate * 1000
    let gap = (3.5 * 11.0 / baudrate as f64 * 1000.0) as u64;
    gap.max(2) // Minimum 2ms
}

/// Verify Modbus RTU CRC-16 (CRC-16/IBM, polynomial 0xA001)
fn verify_modbus_crc(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let payload = &data[..data.len() - 2];
    let received_crc = u16::from_le_bytes([data[data.len() - 2], data[data.len() - 1]]);
    let calculated = crc16_modbus(payload);
    calculated == received_crc
}

fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/// Configure serial port using libc termios
fn configure_serial(file: &std::fs::File, baudrate: u32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let fd = file.as_raw_fd();

    let speed = match baudrate {
        9600 => libc::B9600,
        19200 => libc::B19200,
        38400 => libc::B38400,
        57600 => libc::B57600,
        115200 => libc::B115200,
        230400 => libc::B230400,
        _ => libc::B115200,
    };

    unsafe {
        let mut tios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut tios) != 0 {
            return Err(format!("tcgetattr failed: {}", std::io::Error::last_os_error()).into());
        }

        libc::cfmakeraw(&mut tios);
        libc::cfsetispeed(&mut tios, speed);
        libc::cfsetospeed(&mut tios, speed);

        // 8N1
        tios.c_cflag &= !libc::CSIZE;
        tios.c_cflag |= libc::CS8;
        tios.c_cflag &= !libc::PARENB;
        tios.c_cflag &= !libc::CSTOPB;
        tios.c_cflag |= libc::CREAD | libc::CLOCAL;

        tios.c_cc[libc::VMIN] = 1;
        tios.c_cc[libc::VTIME] = 0;

        if libc::tcsetattr(fd, libc::TCSANOW, &tios) != 0 {
            return Err(format!("tcsetattr failed: {}", std::io::Error::last_os_error()).into());
        }
    }

    Ok(())
}
