//! Ghi dữ liệu/lệnh ra UART TX xuống MCU
//! Dùng để gửi lệnh điều khiển hoặc dữ liệu từ gateway xuống thiết bị

use std::io::Write;
use std::os::unix::io::{AsRawFd, FromRawFd};

pub struct UartWriter {
    file: std::fs::File,
}

impl UartWriter {
    /// Mở UART port để ghi — blocking write, không cần O_NONBLOCK
    pub fn new(port: &str, baudrate: u32) -> std::io::Result<Self> {
        let path = std::ffi::CString::new(port)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let fd = unsafe { libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NOCTTY) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let file = unsafe { std::fs::File::from_raw_fd(fd) };

        // Configure termios cho write (baudrate, raw mode, 8N1)
        configure_write_serial(&file, baudrate)?;

        Ok(Self { file })
    }

    /// Write bytes to UART
    pub fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.file.write_all(data)?;
        // Đợi dữ liệu gửi xong tới hardware
        unsafe { libc::tcdrain(self.file.as_raw_fd()) };
        Ok(())
    }
}

/// Configure serial port cho write: raw mode, baudrate, 8N1
fn configure_write_serial(file: &std::fs::File, baudrate: u32) -> std::io::Result<()> {
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
            return Err(std::io::Error::last_os_error());
        }

        libc::cfmakeraw(&mut tios);
        libc::cfsetospeed(&mut tios, speed);

        // 8N1
        tios.c_cflag &= !libc::CSIZE;
        tios.c_cflag |= libc::CS8;
        tios.c_cflag &= !libc::PARENB;
        tios.c_cflag &= !libc::CSTOPB;
        tios.c_cflag |= libc::CLOCAL;

        if libc::tcsetattr(fd, libc::TCSANOW, &tios) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    Ok(())
}
