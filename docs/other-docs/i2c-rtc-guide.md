# I2C & RTC Guide

## Part 1: I2C Interface

### Hardware Info

| Property | Value |
|----------|-------|
| Controller | i2c-mt7621 |
| Bus | `/dev/i2c-0` |
| Speed | 100 kHz |
| SDA/SCL | GPIO4/GPIO5 (I2C pins) |

### Kernel Modules

```
i2c_mt7621   - MT7628 I2C controller
i2c_dev      - /dev/i2c-* interface
i2c_core     - I2C subsystem core
```

### Shell Commands

```bash
# List I2C buses
ls /dev/i2c*

# Scan for devices
i2cdetect -y 0

# Read single byte
i2cget -y 0 <addr> <reg>

# Write single byte
i2cset -y 0 <addr> <reg> <value>

# Dump all registers
i2cdump -y 0 <addr>
```

### Detected Devices

| Address | Device | Status |
|---------|--------|--------|
| `0x1a` | Unknown (audio codec?) | Available |
| `0x51` | RTC PCF8563 | UU (kernel driver) |

**UU** = Used by kernel driver, access via sysfs instead.

### Rust I2C Access

```rust
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;

const I2C_SLAVE: u64 = 0x0703;

pub struct I2c {
    file: std::fs::File,
}

impl I2c {
    pub fn new(bus: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(bus)?;
        Ok(Self { file })
    }

    pub fn set_slave(&self, addr: u8) -> io::Result<()> {
        unsafe {
            if libc::ioctl(self.file.as_raw_fd(), I2C_SLAVE, addr as i32) < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub fn read_reg(&mut self, reg: u8) -> io::Result<u8> {
        self.file.write_all(&[reg])?;
        let mut buf = [0u8; 1];
        self.file.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn write_reg(&mut self, reg: u8, value: u8) -> io::Result<()> {
        self.file.write_all(&[reg, value])
    }
}
```

---

## Part 2: RTC PCF8563

### Hardware Info

| Property | Value |
|----------|-------|
| Chip | NXP PCF8563 |
| I2C Address | `0x51` |
| Interface | I2C |
| Backup | Battery (CR1220) |

### Register Map

| Reg | Name | Bits | Description |
|-----|------|------|-------------|
| 0x00 | Control_1 | 8 | Control/status |
| 0x01 | Control_2 | 8 | Control/status |
| 0x02 | Seconds | BCD | 0-59, bit7=VL (voltage low) |
| 0x03 | Minutes | BCD | 0-59 |
| 0x04 | Hours | BCD | 0-23 |
| 0x05 | Days | BCD | 1-31 |
| 0x06 | Weekdays | 0-6 | 0=Sunday |
| 0x07 | Months | BCD | 1-12, bit7=century |
| 0x08 | Years | BCD | 0-99 |

### Method 1: Via Sysfs (Recommended)

RTC driver đã load, dùng sysfs interface:

```bash
# Read time
cat /sys/class/rtc/rtc0/time    # 09:07:31
cat /sys/class/rtc/rtc0/date    # 2026-02-12

# Read via hwclock
hwclock -r

# Set time from system
hwclock -w

# Set system time from RTC
hwclock -s
```

### Method 2: Direct I2C (for learning)

```bash
# Đọc giây (reg 0x02)
i2cget -y 0 0x51 0x02

# Đọc toàn bộ time registers
i2cdump -y -r 0x02-0x08 0 0x51
```

### Rust Implementation

```rust
use std::fs;
use std::io::Result;

/// Read RTC via sysfs (recommended)
pub fn read_rtc_sysfs() -> Result<(String, String)> {
    let date = fs::read_to_string("/sys/class/rtc/rtc0/date")?;
    let time = fs::read_to_string("/sys/class/rtc/rtc0/time")?;
    Ok((date.trim().to_string(), time.trim().to_string()))
}

/// Read RTC via direct I2C (educational)
pub fn read_rtc_i2c(i2c: &mut I2c) -> Result<RtcTime> {
    i2c.set_slave(0x51)?;

    let sec = bcd_to_dec(i2c.read_reg(0x02)? & 0x7F);
    let min = bcd_to_dec(i2c.read_reg(0x03)? & 0x7F);
    let hour = bcd_to_dec(i2c.read_reg(0x04)? & 0x3F);
    let day = bcd_to_dec(i2c.read_reg(0x05)? & 0x3F);
    let month = bcd_to_dec(i2c.read_reg(0x07)? & 0x1F);
    let year = bcd_to_dec(i2c.read_reg(0x08)?) as u16 + 2000;

    Ok(RtcTime { year, month, day, hour, min, sec })
}

pub struct RtcTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
}

fn bcd_to_dec(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

fn dec_to_bcd(dec: u8) -> u8 {
    ((dec / 10) << 4) | (dec % 10)
}
```

### Usage Example

```rust
fn main() -> std::io::Result<()> {
    // Method 1: Sysfs (simple)
    let (date, time) = read_rtc_sysfs()?;
    println!("RTC: {} {}", date, time);

    // Method 2: Direct I2C (advanced)
    let mut i2c = I2c::new("/dev/i2c-0")?;
    let rtc = read_rtc_i2c(&mut i2c)?;
    println!("RTC: {}-{:02}-{:02} {:02}:{:02}:{:02}",
        rtc.year, rtc.month, rtc.day, rtc.hour, rtc.min, rtc.sec);

    Ok(())
}
```

---

## Part 3: Troubleshooting & Notes

### OpenWrt Package Repository Fix

SNAPSHOT builds có repo không ổn định. Sửa như sau:

```bash
# Backup config
cp /etc/opkg/distfeeds.conf /etc/opkg/distfeeds.conf.bak

# Use archive mirror (HTTP, stable)
cat > /etc/opkg/distfeeds.conf << 'EOF'
src/gz openwrt_core http://archive.openwrt.org/releases/21.02.7/targets/ramips/mt76x8/packages
src/gz openwrt_base http://archive.openwrt.org/releases/21.02.7/packages/mipsel_24kc/base
src/gz openwrt_packages http://archive.openwrt.org/releases/21.02.7/packages/mipsel_24kc/packages
EOF

# Update and install
opkg update
opkg install i2c-tools
```

**Note:** SNAPSHOT → archive, HTTPS → HTTP (no SSL required).

### I2C Device Access

| Scenario | Solution |
|----------|----------|
| Device shows `UU` | Kernel driver active, use sysfs |
| Permission denied | Run as root |
| Device not found | Check `i2cdetect`, verify wiring |
| Bus busy | Another process using I2C |

### RTC Notes

| Issue | Cause | Solution |
|-------|-------|----------|
| Wrong time after reboot | No battery | Install CR1220 |
| VL bit set (bit7 of 0x02) | Low voltage detected | Replace battery, re-set time |
| Time in UTC | Linux default | Set timezone or convert in app |

### Timezone

RTC lưu UTC. Board time = UTC, Vietnam = UTC+7:

```bash
# Set timezone
export TZ='ICT-7'

# Or in /etc/profile
echo "export TZ='ICT-7'" >> /etc/profile
```

### Kernel Messages

```bash
# Check I2C init
dmesg | grep -i i2c

# Check RTC init
dmesg | grep -i rtc
```

Expected output:
```
i2c /dev entries driver
i2c-mt7621 10000900.i2c: clock 100 kHz
rtc-pcf8563 0-0051: registered as rtc0
```

### Dependencies for Rust

```toml
# Cargo.toml (if using libc for ioctl)
[dependencies]
libc = "0.2"
```

## References

- PCF8563 Datasheet: NXP
- Device Tree: `devicetree/linkit.dts`
- I2C Bus: `/dev/i2c-0`
- RTC Sysfs: `/sys/class/rtc/rtc0/`
