//! Điều khiển GPIO qua chardev ioctl (API kernel hiện đại)
//! Rust thuần, không cần libgpiod, dễ cross-compile cho MIPS
//! Hỗ trợ: set ON/OFF, toggle, heartbeat LED

use crate::commands::{Command, GpioState};
use crate::web::status::SharedStats;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

// --- ioctl constants (từ linux/gpio.h) ---

const GPIOHANDLES_MAX: usize = 64;
// libc::Ioctl chỉ tồn tại trên Linux, macOS dùng c_ulong để cargo check trên host
#[cfg(target_os = "linux")]
type IoctlNum = libc::Ioctl;
#[cfg(not(target_os = "linux"))]
type IoctlNum = libc::c_ulong;
const GPIO_GET_LINEHANDLE_IOCTL: IoctlNum = 0xC16CB403u32 as IoctlNum;
const GPIOHANDLE_SET_LINE_VALUES_IOCTL: IoctlNum = 0xC040B409u32 as IoctlNum;
const GPIOHANDLE_GET_LINE_VALUES_IOCTL: IoctlNum = 0xC040B408u32 as IoctlNum;
const GPIOHANDLE_REQUEST_OUTPUT: u32 = 0x02;

#[repr(C)]
struct GpioHandleRequest {
    lineoffsets: [u32; GPIOHANDLES_MAX],
    flags: u32,
    default_values: [u8; GPIOHANDLES_MAX],
    consumer_label: [std::os::raw::c_char; 32],
    lines: u32,
    fd: i32,
}

#[repr(C)]
struct GpioHandleData {
    values: [u8; GPIOHANDLES_MAX],
}

/// Wrapper cho 1 GPIO line (output)
struct GpioLine {
    handle: std::fs::File,
}

impl GpioLine {
    /// Yêu cầu 1 line output từ GPIO chip
    fn request_output(chip: &str, line: u32, initial: bool) -> std::io::Result<Self> {
        let chip_path = format!("/dev/{}", chip);
        let chip_file = std::fs::File::open(&chip_path)?;

        let mut req = GpioHandleRequest {
            lineoffsets: [0; GPIOHANDLES_MAX],
            flags: GPIOHANDLE_REQUEST_OUTPUT,
            default_values: [0; GPIOHANDLES_MAX],
            consumer_label: [0; 32],
            lines: 1,
            fd: 0,
        };
        req.lineoffsets[0] = line;
        req.default_values[0] = if initial { 1 } else { 0 };

        // Ghi label "ugate"
        for (i, &b) in b"ugate\0".iter().enumerate() {
            req.consumer_label[i] = b as std::os::raw::c_char;
        }

        let ret = unsafe {
            libc::ioctl(chip_file.as_raw_fd(), GPIO_GET_LINEHANDLE_IOCTL, &mut req)
        };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            handle: unsafe { std::fs::File::from_raw_fd(req.fd) },
        })
    }

    fn set_value(&self, value: bool) -> std::io::Result<()> {
        let mut data = GpioHandleData { values: [0; GPIOHANDLES_MAX] };
        data.values[0] = if value { 1 } else { 0 };
        let ret = unsafe {
            libc::ioctl(self.handle.as_raw_fd(), GPIOHANDLE_SET_LINE_VALUES_IOCTL, &data)
        };
        if ret < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
    }

    fn get_value(&self) -> std::io::Result<bool> {
        let mut data = GpioHandleData { values: [0; GPIOHANDLES_MAX] };
        let ret = unsafe {
            libc::ioctl(self.handle.as_raw_fd(), GPIOHANDLE_GET_LINE_VALUES_IOCTL, &mut data)
        };
        if ret < 0 { Err(std::io::Error::last_os_error()) } else { Ok(data.values[0] != 0) }
    }

    fn toggle(&self) -> std::io::Result<bool> {
        let current = self.get_value()?;
        let new_val = !current;
        self.set_value(new_val)?;
        Ok(new_val)
    }
}

/// Task GPIO: nhận lệnh từ channel, điều khiển output + heartbeat LED
pub async fn run(
    config: crate::config::GpioConfig,
    mut cmd_rx: tokio::sync::mpsc::Receiver<Command>,
    stats: Arc<SharedStats>,
) {
    // Thử mở GPIO chip, nếu không có thì chỉ log warning
    let chip = "gpiochip0";
    let mut outputs: Vec<Option<GpioLine>> = Vec::new();

    for &pin in &config.pins {
        match GpioLine::request_output(chip, pin as u32, false) {
            Ok(line) => {
                log::info!("[GPIO] Pin {} sẵn sàng (output)", pin);
                outputs.push(Some(line));
            }
            Err(e) => {
                log::warn!("[GPIO] Không thể mở pin {}: {} (bỏ qua)", pin, e);
                outputs.push(None);
            }
        }
    }

    // Heartbeat LED
    let heartbeat = match GpioLine::request_output(chip, config.led_pin as u32, false) {
        Ok(line) => {
            log::info!("[GPIO] Heartbeat LED pin {} sẵn sàng", config.led_pin);
            Some(line)
        }
        Err(e) => {
            log::warn!("[GPIO] Heartbeat LED pin {} lỗi: {} (bỏ qua)", config.led_pin, e);
            None
        }
    };

    let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                if let Command::Gpio { pin, state } = cmd {
                    let idx = (pin.saturating_sub(1)) as usize;
                    if idx < outputs.len() {
                        if let Some(ref line) = outputs[idx] {
                            let result = match state {
                                GpioState::On => line.set_value(true).map(|_| true),
                                GpioState::Off => line.set_value(false).map(|_| false),
                                GpioState::Toggle => line.toggle(),
                            };
                            match result {
                                Ok(val) => {
                                    stats.gpio_states[idx].store(if val { 1 } else { 0 }, Ordering::Relaxed);
                                    log::debug!("[GPIO] Pin {} = {}", pin, val);
                                }
                                Err(e) => log::error!("[GPIO] Pin {} lỗi: {}", pin, e),
                            }
                        }
                    }
                }
            }
            _ = heartbeat_interval.tick() => {
                if let Some(ref hb) = heartbeat {
                    let _ = hb.toggle();
                }
            }
        }
    }
}
