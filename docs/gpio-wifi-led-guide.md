# GPIO WiFi LED Guide

## Hardware Info

| Property | Value |
|----------|-------|
| Board | LinkIt Smart 7688 |
| LED | WiFi LED (Orange) |
| Hardware GPIO | GPIO44 |
| Linux GPIO | 460 |
| Logic | **Active-Low** (0=ON, 1=OFF) |

## GPIO Mapping

MT7628AN kernel uses offset-based GPIO numbering:

```
Bank0: GPIO0-31   → Linux 480-511
Bank1: GPIO32-63  → Linux 448-479
Bank2: GPIO64-95  → Linux 416-447

Formula (Bank1): linux_gpio = 448 + (hw_gpio - 32)
GPIO44: 448 + (44 - 32) = 460
```

## Shell Commands

### Export GPIO

```bash
echo 460 > /sys/class/gpio/export
# Note: "Resource busy" error is OK if already exported
```

### Configure as Output

```bash
echo out > /sys/class/gpio/gpio460/direction
```

### Control LED

```bash
# Turn ON (active-low)
echo 0 > /sys/class/gpio/gpio460/value

# Turn OFF
echo 1 > /sys/class/gpio/gpio460/value
```

### Read Current State

```bash
cat /sys/class/gpio/gpio460/value
```

### Cleanup (Optional)

```bash
echo 460 > /sys/class/gpio/unexport
```

## Rust Implementation

```rust
use std::fs;
use std::io::Result;
use std::path::Path;

const WIFI_LED_GPIO: u32 = 460;

pub struct WifiLed;

impl WifiLed {
    /// Initialize WiFi LED GPIO
    pub fn init() -> Result<()> {
        let gpio_path = format!("/sys/class/gpio/gpio{}", WIFI_LED_GPIO);

        // Export if not exists
        if !Path::new(&gpio_path).exists() {
            fs::write("/sys/class/gpio/export", WIFI_LED_GPIO.to_string())?;
        }

        // Set as output
        fs::write(format!("{}/direction", gpio_path), "out")?;
        Ok(())
    }

    /// Turn LED ON (active-low)
    pub fn on() -> Result<()> {
        fs::write(
            format!("/sys/class/gpio/gpio{}/value", WIFI_LED_GPIO),
            "0"
        )
    }

    /// Turn LED OFF
    pub fn off() -> Result<()> {
        fs::write(
            format!("/sys/class/gpio/gpio{}/value", WIFI_LED_GPIO),
            "1"
        )
    }

    /// Toggle LED state
    pub fn toggle() -> Result<()> {
        let value = fs::read_to_string(
            format!("/sys/class/gpio/gpio{}/value", WIFI_LED_GPIO)
        )?;

        match value.trim() {
            "0" => Self::off(),
            _ => Self::on(),
        }
    }

    /// Blink LED n times
    pub fn blink(times: u32, delay_ms: u64) -> Result<()> {
        use std::thread::sleep;
        use std::time::Duration;

        for _ in 0..times {
            Self::on()?;
            sleep(Duration::from_millis(delay_ms));
            Self::off()?;
            sleep(Duration::from_millis(delay_ms));
        }
        Ok(())
    }
}
```

### Usage Example

```rust
fn main() -> std::io::Result<()> {
    WifiLed::init()?;

    // Blink 3 times on startup
    WifiLed::blink(3, 200)?;

    // Keep LED on while running
    WifiLed::on()?;

    // ... application logic ...

    WifiLed::off()?;
    Ok(())
}
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "Resource busy" on export | GPIO already exported, ignore error |
| "Permission denied" | Run as root or add udev rule |
| LED not responding | Check pin mux: `cat /sys/kernel/debug/pinctrl/pinctrl/pinmux-pins` |
| LED controlled by driver | Disable trigger: `echo none > /sys/class/leds/*/trigger` |

## References

- Device Tree: `devicetree/linkit.dts`
- Hardware GPIO44 = Bank1, offset 12
- Active-low: Common for LEDs (sink current to GND)
