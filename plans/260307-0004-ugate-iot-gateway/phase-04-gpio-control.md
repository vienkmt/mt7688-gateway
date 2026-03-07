# Phase 4: GPIO Control

**Priority:** Medium
**Status:** pending
**Effort:** 1 day
**Depends on:** Phase 3

## Context

Control GPIO outputs via **GPIO chardev** (modern kernel API).
Rust thuần với ioctl - không cần libgpiod, dễ cross-compile cho MIPS.

## Architecture

```
UCI Config (/etc/config/ugate)
    │
    ▼
chip + line numbers
    │
    ▼
/dev/gpiochipX + ioctl
    │
    ▼
GpioController
```

**Không cần DTS** — chardev chỉ cần chip name + line number.

## UCI Config

```
config gpio
    option chip 'gpiochip0'
    list output '11'
    list output '12'
    list output '14'
    list output '15'
    option heartbeat '44'
    option heartbeat_interval '500'
```

## Implementation

### 1. GPIO chardev ioctl constants

```rust
// gpio_ioctl.rs
use std::os::raw::c_char;

pub const GPIOHANDLES_MAX: usize = 64;

#[repr(C)]
pub struct GpioHandleRequest {
    pub lineoffsets: [u32; GPIOHANDLES_MAX],
    pub flags: u32,
    pub default_values: [u8; GPIOHANDLES_MAX],
    pub consumer_label: [c_char; 32],
    pub lines: u32,
    pub fd: i32,
}

#[repr(C)]
pub struct GpioHandleData {
    pub values: [u8; GPIOHANDLES_MAX],
}

// ioctl numbers (from linux/gpio.h)
// _IOWR(0xB4, 0x03, 364) = 0xC16CB403
pub const GPIO_GET_LINEHANDLE_IOCTL: libc::c_ulong = 0xC16CB403;
pub const GPIOHANDLE_SET_LINE_VALUES_IOCTL: libc::c_ulong = 0xC040B409;
pub const GPIOHANDLE_GET_LINE_VALUES_IOCTL: libc::c_ulong = 0xC040B408;

pub const GPIOHANDLE_REQUEST_OUTPUT: u32 = 0x02;

// Compile-time assert: verify struct sizes match ioctl expectations
// If this fails on MIPS, ioctl numbers need recalculation
const _: () = assert!(
    std::mem::size_of::<GpioHandleRequest>() == 364,
    "GpioHandleRequest size mismatch — recalculate GPIO_GET_LINEHANDLE_IOCTL"
);
const _: () = assert!(
    std::mem::size_of::<GpioHandleData>() == 64,
    "GpioHandleData size mismatch — recalculate GPIOHANDLE_*_LINE_VALUES_IOCTL"
);
```

### 2. GpioLine wrapper

```rust
// gpio.rs
use std::fs::File;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd};

pub struct GpioLine {
    handle: File,
}

impl GpioLine {
    pub fn request_output(chip: &str, line: u32, initial: bool) -> io::Result<Self> {
        let chip_file = File::open(format!("/dev/{}", chip))?;

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

        // Copy "ugate" to consumer_label
        let label = b"ugate\0";
        for (i, &b) in label.iter().enumerate() {
            req.consumer_label[i] = b as c_char;
        }

        let ret = unsafe {
            libc::ioctl(chip_file.as_raw_fd(), GPIO_GET_LINEHANDLE_IOCTL, &mut req)
        };

        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            handle: unsafe { File::from_raw_fd(req.fd) },
        })
    }

    pub fn set_value(&self, value: bool) -> io::Result<()> {
        let mut data = GpioHandleData { values: [0; GPIOHANDLES_MAX] };
        data.values[0] = if value { 1 } else { 0 };

        let ret = unsafe {
            libc::ioctl(self.handle.as_raw_fd(), GPIOHANDLE_SET_LINE_VALUES_IOCTL, &data)
        };

        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn get_value(&self) -> io::Result<bool> {
        let mut data = GpioHandleData { values: [0; GPIOHANDLES_MAX] };

        let ret = unsafe {
            libc::ioctl(self.handle.as_raw_fd(), GPIOHANDLE_GET_LINE_VALUES_IOCTL, &mut data)
        };

        if ret < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(data.values[0] != 0)
        }
    }

    pub fn toggle(&self) -> io::Result<()> {
        let current = self.get_value()?;
        self.set_value(!current)
    }
}
```

### 3. GpioController

```rust
pub struct GpioController {
    outputs: Vec<GpioLine>,  // output1-4
    heartbeat: GpioLine,
}

impl GpioController {
    pub fn new(config: &GpioConfig) -> io::Result<Self> {
        let mut outputs = Vec::with_capacity(config.outputs.len());
        for &line in &config.outputs {
            outputs.push(GpioLine::request_output(&config.chip, line, false)?);
        }

        let heartbeat = GpioLine::request_output(&config.chip, config.heartbeat, false)?;

        Ok(Self { outputs, heartbeat })
    }

    /// Set output state. Pin numbers are 1-based (1-4).
    pub fn set_output(&self, pin: u8, state: GpioState) -> io::Result<()> {
        if pin < 1 || pin > 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "pin must be 1-4"));
        }
        let idx = (pin - 1) as usize;

        match state {
            GpioState::On => self.outputs[idx].set_value(true),
            GpioState::Off => self.outputs[idx].set_value(false),
            GpioState::Toggle => self.outputs[idx].toggle(),
        }
    }

    pub fn heartbeat_tick(&self) -> io::Result<()> {
        self.heartbeat.toggle()
    }
}
```

### 4. GPIO task

```rust
async fn run_gpio_task(
    config: GpioConfig,
    mut cmd_rx: mpsc::Receiver<Command>,
) {
    let controller = match GpioController::new(&config) {
        Ok(c) => c,
        Err(e) => {
            log::error!("GPIO init failed: {}", e);
            return;
        }
    };

    let mut heartbeat = tokio::time::interval(
        Duration::from_millis(config.heartbeat_interval)
    );

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                if let Command::Gpio { pin, state } = cmd {
                    if let Err(e) = controller.set_output(pin, state) {
                        log::error!("GPIO set error: {}", e);
                    }
                }
            }
            _ = heartbeat.tick() => {
                let _ = controller.heartbeat_tick();
            }
        }
    }
}
```

## Command Flow

```
"GPIO:1:ON\n"              {"cmd":"gpio","pin":1,"state":"on"}
    │                              │
    └──────────┬───────────────────┘
               ▼
         Command::Gpio { pin: 1, state: On }
               │
               ▼
         ioctl(GPIOHANDLE_SET_LINE_VALUES_IOCTL)
```

## Files to Create

| File | Action |
|------|--------|
| ugate/src/gpio_ioctl.rs | Create - ioctl constants |
| ugate/src/gpio.rs | Create - GpioLine, GpioController |
| ugate/src/main.rs | Modify |
| ugate/src/config.rs | Add GpioConfig |

## Dependencies

```toml
# Chỉ cần libc cho ioctl
libc = "0.2"
```

## Todo

- [ ] Create gpio_ioctl.rs với ioctl constants
- [ ] Create gpio.rs với GpioLine wrapper
- [ ] Create GpioController
- [ ] Add GPIO task in main.rs
- [ ] Add GpioConfig to config.rs (read from UCI)
- [ ] Test on hardware với gpiodetect
- [ ] Test set/get/toggle
- [ ] Test heartbeat LED

## Hardware Testing

```bash
# Verify chip exists
ls /dev/gpiochip*

# Test với gpioset (if libgpiod-tools installed)
gpioset gpiochip0 11=1
gpioset gpiochip0 11=0
```

## Success Criteria

- [ ] GPIO chardev ioctl works on MIPS
- [ ] Lines initialize correctly
- [ ] Set ON/OFF works
- [ ] Toggle works
- [ ] Heartbeat LED blinks

## Next Phase

Phase 5: Vue.js Frontend
