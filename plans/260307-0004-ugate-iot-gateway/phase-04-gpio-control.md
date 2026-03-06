# Phase 4: GPIO Control

**Priority:** Medium
**Status:** pending
**Effort:** 1 day
**Depends on:** Phase 1

## Context

Control 4 GPIO outputs + 1 LED heartbeat via sysfs interface.
Commands từ MCU (UART) hoặc Server (WebSocket/TCP/MQTT).

## GPIO Pins (MT7688)

| GPIO | Function | Direction |
|------|----------|-----------|
| TBD | Output 1 | OUT |
| TBD | Output 2 | OUT |
| TBD | Output 3 | OUT |
| TBD | Output 4 | OUT |
| TBD | Heartbeat LED | OUT |

> Note: Cần xác định GPIO pin numbers cụ thể trên hardware.

## Implementation Steps

### 1. Create gpio.rs

```rust
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub struct GpioPin {
    pin: u8,
    value_file: File,
}

impl GpioPin {
    pub fn new(pin: u8) -> std::io::Result<Self> {
        // Export GPIO if not already
        let export_path = "/sys/class/gpio/export";
        if !Path::new(&format!("/sys/class/gpio/gpio{}", pin)).exists() {
            let mut f = File::create(export_path)?;
            write!(f, "{}", pin)?;
        }

        // Set direction to output
        let dir_path = format!("/sys/class/gpio/gpio{}/direction", pin);
        let mut f = OpenOptions::new().write(true).open(&dir_path)?;
        write!(f, "out")?;

        // Open value file for writing
        let value_path = format!("/sys/class/gpio/gpio{}/value", pin);
        let value_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&value_path)?;

        Ok(Self { pin, value_file })
    }

    pub fn set(&mut self, high: bool) -> std::io::Result<()> {
        write!(self.value_file, "{}", if high { "1" } else { "0" })
    }

    pub fn get(&mut self) -> std::io::Result<bool> {
        let mut buf = [0u8; 1];
        self.value_file.read(&mut buf)?;
        Ok(buf[0] == b'1')
    }

    pub fn toggle(&mut self) -> std::io::Result<()> {
        let current = self.get()?;
        self.set(!current)
    }
}

impl Drop for GpioPin {
    fn drop(&mut self) {
        // Unexport GPIO
        if let Ok(mut f) = File::create("/sys/class/gpio/unexport") {
            let _ = write!(f, "{}", self.pin);
        }
    }
}
```

### 2. Create GpioController

```rust
pub struct GpioController {
    outputs: [GpioPin; 4],
    led: GpioPin,
}

impl GpioController {
    pub fn new(config: &GpioConfig) -> std::io::Result<Self> {
        Ok(Self {
            outputs: [
                GpioPin::new(config.pins[0])?,
                GpioPin::new(config.pins[1])?,
                GpioPin::new(config.pins[2])?,
                GpioPin::new(config.pins[3])?,
            ],
            led: GpioPin::new(config.led_pin)?,
        })
    }

    pub fn set_output(&mut self, pin: u8, state: GpioState) -> std::io::Result<()> {
        if pin > 3 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid pin")); }
        match state {
            GpioState::On => self.outputs[pin as usize].set(true),
            GpioState::Off => self.outputs[pin as usize].set(false),
            GpioState::Toggle => self.outputs[pin as usize].toggle(),
        }
    }

    pub fn heartbeat_tick(&mut self) -> std::io::Result<()> {
        self.led.toggle()
    }
}
```

### 3. Create GPIO task in main.rs

```rust
async fn run_gpio_task(
    config: GpioConfig,
    mut cmd_rx: mpsc::Receiver<Command>,
) {
    let mut controller = match GpioController::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("GPIO init failed: {}", e);
            return;
        }
    };

    let mut heartbeat = tokio::time::interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                if let Command::Gpio { pin, state } = cmd {
                    if let Err(e) = controller.set_output(pin, state) {
                        eprintln!("GPIO error: {}", e);
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

### 4. Wire in main.rs

```rust
// GPIO task
let gpio_config = state.get().gpio.clone();
tokio::spawn(run_gpio_task(gpio_config, gpio_rx));
```

## Command Flow

```
MCU (UART)                    Server (WS/TCP/MQTT)
    │                              │
    ▼                              ▼
"GPIO:1:ON\n"              {"cmd":"gpio","pin":1,"state":"on"}
    │                              │
    ▼                              ▼
parse_uart_command()        parse_json_command()
    │                              │
    └──────────┬───────────────────┘
               ▼
         cmd_tx.send(Command::Gpio { pin: 1, state: On })
               │
               ▼
         GPIO Task
               │
               ▼
         /sys/class/gpio/gpio{X}/value = "1"
```

## Files to Create/Modify

| File | Action |
|------|--------|
| ugate/src/gpio.rs | Create |
| ugate/src/main.rs | Modify |
| ugate/src/commands.rs | Modify (add GpioState) |

## Todo

- [ ] Create gpio.rs với GpioPin struct
- [ ] Create GpioController
- [ ] Add GPIO task in main.rs
- [ ] Integrate command parsing
- [ ] Test GPIO export/unexport
- [ ] Test set high/low
- [ ] Test toggle
- [ ] Test heartbeat LED

## Success Criteria

- [ ] GPIO pins initialize
- [ ] Set ON/OFF works
- [ ] Toggle works
- [ ] MCU commands work
- [ ] Server commands work
- [ ] LED heartbeat blinks

## Hardware Testing Notes

```bash
# Manual GPIO test on device
echo 11 > /sys/class/gpio/export
echo out > /sys/class/gpio/gpio11/direction
echo 1 > /sys/class/gpio/gpio11/value
echo 0 > /sys/class/gpio/gpio11/value
echo 11 > /sys/class/gpio/unexport
```

## Next Phase

Phase 5: Vue.js Frontend
