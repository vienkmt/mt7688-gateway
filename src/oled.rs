//! SSD1306 OLED 0.91" (128x32) via I2C for MT7688AN
//! I2C bus: /dev/i2c-0, Address: 0x3C

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;

const I2C_SLAVE: u32 = 0x0703;
const OLED_ADDR: u8 = 0x3C;
const WIDTH: usize = 128;
const HEIGHT: usize = 32;

// 5x8 font for ASCII 32-127 (space to ~)
const FONT_5X8: [u8; 480] = [
    0x00,0x00,0x00,0x00,0x00, // 32 space
    0x00,0x00,0x5F,0x00,0x00, // 33 !
    0x00,0x07,0x00,0x07,0x00, // 34 "
    0x14,0x7F,0x14,0x7F,0x14, // 35 #
    0x24,0x2A,0x7F,0x2A,0x12, // 36 $
    0x23,0x13,0x08,0x64,0x62, // 37 %
    0x36,0x49,0x55,0x22,0x50, // 38 &
    0x00,0x05,0x03,0x00,0x00, // 39 '
    0x00,0x1C,0x22,0x41,0x00, // 40 (
    0x00,0x41,0x22,0x1C,0x00, // 41 )
    0x08,0x2A,0x1C,0x2A,0x08, // 42 *
    0x08,0x08,0x3E,0x08,0x08, // 43 +
    0x00,0x50,0x30,0x00,0x00, // 44 ,
    0x08,0x08,0x08,0x08,0x08, // 45 -
    0x00,0x60,0x60,0x00,0x00, // 46 .
    0x20,0x10,0x08,0x04,0x02, // 47 /
    0x3E,0x51,0x49,0x45,0x3E, // 48 0
    0x00,0x42,0x7F,0x40,0x00, // 49 1
    0x42,0x61,0x51,0x49,0x46, // 50 2
    0x21,0x41,0x45,0x4B,0x31, // 51 3
    0x18,0x14,0x12,0x7F,0x10, // 52 4
    0x27,0x45,0x45,0x45,0x39, // 53 5
    0x3C,0x4A,0x49,0x49,0x30, // 54 6
    0x01,0x71,0x09,0x05,0x03, // 55 7
    0x36,0x49,0x49,0x49,0x36, // 56 8
    0x06,0x49,0x49,0x29,0x1E, // 57 9
    0x00,0x36,0x36,0x00,0x00, // 58 :
    0x00,0x56,0x36,0x00,0x00, // 59 ;
    0x00,0x08,0x14,0x22,0x41, // 60 <
    0x14,0x14,0x14,0x14,0x14, // 61 =
    0x41,0x22,0x14,0x08,0x00, // 62 >
    0x02,0x01,0x51,0x09,0x06, // 63 ?
    0x32,0x49,0x79,0x41,0x3E, // 64 @
    0x7E,0x11,0x11,0x11,0x7E, // 65 A
    0x7F,0x49,0x49,0x49,0x36, // 66 B
    0x3E,0x41,0x41,0x41,0x22, // 67 C
    0x7F,0x41,0x41,0x22,0x1C, // 68 D
    0x7F,0x49,0x49,0x49,0x41, // 69 E
    0x7F,0x09,0x09,0x01,0x01, // 70 F
    0x3E,0x41,0x41,0x51,0x32, // 71 G
    0x7F,0x08,0x08,0x08,0x7F, // 72 H
    0x00,0x41,0x7F,0x41,0x00, // 73 I
    0x20,0x40,0x41,0x3F,0x01, // 74 J
    0x7F,0x08,0x14,0x22,0x41, // 75 K
    0x7F,0x40,0x40,0x40,0x40, // 76 L
    0x7F,0x02,0x04,0x02,0x7F, // 77 M
    0x7F,0x04,0x08,0x10,0x7F, // 78 N
    0x3E,0x41,0x41,0x41,0x3E, // 79 O
    0x7F,0x09,0x09,0x09,0x06, // 80 P
    0x3E,0x41,0x51,0x21,0x5E, // 81 Q
    0x7F,0x09,0x19,0x29,0x46, // 82 R
    0x46,0x49,0x49,0x49,0x31, // 83 S
    0x01,0x01,0x7F,0x01,0x01, // 84 T
    0x3F,0x40,0x40,0x40,0x3F, // 85 U
    0x1F,0x20,0x40,0x20,0x1F, // 86 V
    0x7F,0x20,0x18,0x20,0x7F, // 87 W
    0x63,0x14,0x08,0x14,0x63, // 88 X
    0x03,0x04,0x78,0x04,0x03, // 89 Y
    0x61,0x51,0x49,0x45,0x43, // 90 Z
    0x00,0x00,0x7F,0x41,0x41, // 91 [
    0x02,0x04,0x08,0x10,0x20, // 92 backslash
    0x41,0x41,0x7F,0x00,0x00, // 93 ]
    0x04,0x02,0x01,0x02,0x04, // 94 ^
    0x40,0x40,0x40,0x40,0x40, // 95 _
    0x00,0x01,0x02,0x04,0x00, // 96 `
    0x20,0x54,0x54,0x54,0x78, // 97 a
    0x7F,0x48,0x44,0x44,0x38, // 98 b
    0x38,0x44,0x44,0x44,0x20, // 99 c
    0x38,0x44,0x44,0x48,0x7F, // 100 d
    0x38,0x54,0x54,0x54,0x18, // 101 e
    0x08,0x7E,0x09,0x01,0x02, // 102 f
    0x08,0x14,0x54,0x54,0x3C, // 103 g
    0x7F,0x08,0x04,0x04,0x78, // 104 h
    0x00,0x44,0x7D,0x40,0x00, // 105 i
    0x20,0x40,0x44,0x3D,0x00, // 106 j
    0x00,0x7F,0x10,0x28,0x44, // 107 k
    0x00,0x41,0x7F,0x40,0x00, // 108 l
    0x7C,0x04,0x18,0x04,0x78, // 109 m
    0x7C,0x08,0x04,0x04,0x78, // 110 n
    0x38,0x44,0x44,0x44,0x38, // 111 o
    0x7C,0x14,0x14,0x14,0x08, // 112 p
    0x08,0x14,0x14,0x18,0x7C, // 113 q
    0x7C,0x08,0x04,0x04,0x08, // 114 r
    0x48,0x54,0x54,0x54,0x20, // 115 s
    0x04,0x3F,0x44,0x40,0x20, // 116 t
    0x3C,0x40,0x40,0x20,0x7C, // 117 u
    0x1C,0x20,0x40,0x20,0x1C, // 118 v
    0x3C,0x40,0x30,0x40,0x3C, // 119 w
    0x44,0x28,0x10,0x28,0x44, // 120 x
    0x0C,0x50,0x50,0x50,0x3C, // 121 y
    0x44,0x64,0x54,0x4C,0x44, // 122 z
    0x00,0x08,0x36,0x41,0x00, // 123 {
    0x00,0x00,0x7F,0x00,0x00, // 124 |
    0x00,0x41,0x36,0x08,0x00, // 125 }
    0x08,0x08,0x2A,0x1C,0x08, // 126 ~
    0x08,0x1C,0x2A,0x08,0x08, // 127 DEL
];

pub struct Oled {
    file: std::fs::File,
    buffer: [u8; WIDTH * HEIGHT / 8], // 512 bytes framebuffer
}

impl Oled {
    pub fn new() -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open("/dev/i2c-0")?;
        unsafe {
            #[allow(clippy::useless_conversion)]
            if libc::ioctl(file.as_raw_fd(), I2C_SLAVE as _, OLED_ADDR as i32) < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Self { file, buffer: [0; WIDTH * HEIGHT / 8] })
    }

    fn write_cmd(&mut self, cmd: u8) -> io::Result<()> {
        self.file.write_all(&[0x00, cmd])
    }

    fn write_data(&mut self, data: u8) -> io::Result<()> {
        self.file.write_all(&[0x40, data])
    }

    /// Initialize OLED display
    pub fn init(&mut self) -> io::Result<()> {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // SSD1306 init sequence for 128x32
        let init_cmds: [u8; 23] = [
            0xAE,       // Display off
            0x40,       // Set start line = 0
            0xB0,       // Set page address
            0xC8,       // COM scan direction
            0x81, 0xFF, // Contrast max
            0xA1,       // Segment remap
            0xA6,       // Normal display
            0xA8, 0x1F, // Multiplex ratio (32-1)
            0xD3, 0x00, // Display offset
            0xD5, 0xF0, // Clock divide
            0xD9, 0x22, // Pre-charge period
            0xDA, 0x02, // COM pins config
            0xDB, 0x49, // VCOMH deselect
            0x8D, 0x14, // Charge pump enable
            0xAF,       // Display on
        ];
        for &cmd in &init_cmds {
            self.write_cmd(cmd)?;
        }
        Ok(())
    }

    /// Clear display buffer
    pub fn clear(&mut self) {
        self.buffer = [0; WIDTH * HEIGHT / 8];
    }

    /// Flush buffer to display
    pub fn display(&mut self) -> io::Result<()> {
        for page in 0..4 {
            self.write_cmd(0xB0 + page)?; // Set page
            self.write_cmd(0x00)?;        // Column low
            self.write_cmd(0x10)?;        // Column high
            for col in 0..WIDTH {
                self.write_data(self.buffer[page as usize * WIDTH + col])?;
            }
        }
        Ok(())
    }

    /// Draw character at (x, page)
    fn draw_char(&mut self, x: usize, page: usize, c: char) {
        let idx = c as usize;
        if idx < 32 || idx > 127 || x + 5 > WIDTH || page >= 4 {
            return;
        }
        let font_offset = (idx - 32) * 5;
        for i in 0..5 {
            self.buffer[page * WIDTH + x + i] = FONT_5X8[font_offset + i];
        }
    }

    /// Draw string at (x, page) - page 0-3 for 32px height
    pub fn draw_string(&mut self, x: usize, page: usize, s: &str) {
        let mut col = x;
        for c in s.chars() {
            if col + 6 > WIDTH { break; }
            self.draw_char(col, page, c);
            col += 6; // 5px char + 1px spacing
        }
    }

    /// Draw character 2x height (spans 2 pages)
    fn draw_char_2x(&mut self, x: usize, page: usize, c: char) {
        let idx = c as usize;
        if idx < 32 || idx > 127 || x + 10 > WIDTH || page + 1 >= 4 {
            return;
        }
        let font_offset = (idx - 32) * 5;
        for i in 0..5 {
            let col = FONT_5X8[font_offset + i];
            // Stretch each bit to 2 bits vertically
            let mut upper: u8 = 0;
            let mut lower: u8 = 0;
            for bit in 0..4 {
                if col & (1 << bit) != 0 {
                    upper |= 3 << (bit * 2);
                }
            }
            for bit in 4..8 {
                if col & (1 << bit) != 0 {
                    lower |= 3 << ((bit - 4) * 2);
                }
            }
            // Double width: write each column twice
            self.buffer[page * WIDTH + x + i * 2] = upper;
            self.buffer[page * WIDTH + x + i * 2 + 1] = upper;
            self.buffer[(page + 1) * WIDTH + x + i * 2] = lower;
            self.buffer[(page + 1) * WIDTH + x + i * 2 + 1] = lower;
        }
    }

    /// Draw string 2x size (10px wide per char, 16px tall)
    pub fn draw_string_2x(&mut self, x: usize, page: usize, s: &str) {
        let mut col = x;
        for c in s.chars() {
            if col + 12 > WIDTH { break; }
            self.draw_char_2x(col, page, c);
            col += 12; // 10px char + 2px spacing
        }
    }
}

/// Get current time as HH:MM:SS
fn get_time() -> String {
    use std::process::Command;
    Command::new("date")
        .arg("+%H:%M:%S")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "--:--:--".to_string())
}

/// Get eth0.2 IP address
fn get_eth02_ip() -> String {
    use std::process::Command;
    let output = Command::new("ip")
        .args(["addr", "show", "eth0.2"])
        .output()
        .ok();
    if let Some(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("inet ") {
                if let Some(addr) = line.split_whitespace().nth(1) {
                    return addr.split('/').next().unwrap_or("-").to_string();
                }
            }
        }
    }
    "No IP".to_string()
}

/// Start background thread updating OLED with time + IP + animation
pub fn start_display_loop() {
    std::thread::spawn(|| {
        let mut oled = match Oled::new() {
            Ok(o) => o,
            Err(_) => return,
        };
        if oled.init().is_err() { return; }

        let mut frame: u8 = 0;
        loop {
            let time = get_time();
            let ip = get_eth02_ip();

            oled.clear();
            oled.draw_string_2x(16, 0, &time);  // Line 1: time

            // Moving dashes effect on page 2
            for i in 0..6 {
                let pos = ((frame as usize + i * 22) % WIDTH) as usize;
                for j in 0..8 { // Dash length = 8px
                    if pos + j < WIDTH {
                        oled.buffer[2 * WIDTH + pos + j] = 0x18; // Two lines in middle
                    }
                }
            }

            let ip_x = (WIDTH.saturating_sub(ip.len() * 6)) / 2;
            oled.draw_string(ip_x, 3, &ip);     // Line 2: IP
            let _ = oled.display();

            frame = frame.wrapping_add(3);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
    });
}
