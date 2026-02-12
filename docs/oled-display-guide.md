# OLED Display Guide

## Hardware Info

| Property | Value |
|----------|-------|
| Display | SSD1306 0.91" OLED |
| Resolution | 128x32 pixels |
| Interface | I2C |
| I2C Address | `0x3C` |
| I2C Bus | `/dev/i2c-0` |

## Features

- **Line 1:** Time display `HH:MM:SS` (font 2x, 16px)
- **Line 2:** Animation - moving dashes
- **Line 3:** eth0.2 IP address (centered, 8px)
- **Update:** ~33fps animation, time/IP every frame

## Module: `src/oled.rs`

### Public Functions

```rust
/// Start background display loop (time + IP + animation)
pub fn start_display_loop();
```

### Usage in main.rs

```rust
mod oled;

fn main() {
    oled::start_display_loop();
    // ... rest of app
}
```

## I2C Protocol

### Command vs Data

| Prefix | Type | Description |
|--------|------|-------------|
| `0x00` | Command | Control bytes (init, page set) |
| `0x40` | Data | Pixel data |

### Init Sequence

```
0xAE        Display OFF
0x40        Start line = 0
0xB0        Page address
0xC8        COM scan direction
0x81, 0xFF  Contrast max
0xA1        Segment remap
0xA6        Normal display
0xA8, 0x1F  Multiplex (32-1)
0xD3, 0x00  Display offset
0xD5, 0xF0  Clock divide
0xD9, 0x22  Pre-charge
0xDA, 0x02  COM pins
0xDB, 0x49  VCOMH
0x8D, 0x14  Charge pump ON
0xAF        Display ON
```

## Display Layout

```
+---------------------------+
|     HH:MM:SS (2x font)    |  Page 0-1 (16px)
+---------------------------+
|   ---- ---- ---- ----     |  Page 2 (8px) animation
+---------------------------+
|     192.168.1.100         |  Page 3 (8px) IP
+---------------------------+
```

## Font

- **5x8 ASCII font** (chars 32-127)
- **2x scaling:** 10px wide, 16px tall (spans 2 pages)
- Stored in `FONT_5X8` array (480 bytes)

## Troubleshooting

| Issue | Solution |
|-------|----------|
| No display | Check I2C: `i2cdetect -y 0` (should show 0x3C) |
| Garbled | Wrong init sequence or timing |
| Permission denied | Run as root |

## Dependencies

```toml
[dependencies]
libc = "0.2"  # For ioctl I2C_SLAVE
```

## References

- SSD1306 Datasheet
- Arduino sample: `docs/OLED_0in91/`
- I2C guide: `docs/i2c-rtc-guide.md`
