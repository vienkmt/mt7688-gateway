use crate::config::AppState;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::unix::AsyncFd;

/// Async UART reader - sends to MQTT (std mpsc) and HTTP (tokio mpsc)
pub async fn run(
    state: Arc<AppState>,
    mqtt_tx: std::sync::mpsc::Sender<String>,
    http_tx: tokio::sync::mpsc::Sender<String>,
) {
    let mut retry_secs = 5u64;
    loop {
        let config = state.get();
        if !config.uart.enabled {
            tokio::time::sleep(Duration::from_secs(10)).await;
            retry_secs = 5;
            continue;
        }
        match run_read_loop(&state, &mqtt_tx, &http_tx).await {
            Ok(()) => retry_secs = 5,
            Err(e) => {
                eprintln!("[UART] Error: {}. Retrying in {}s...", e, retry_secs);
                tokio::time::sleep(Duration::from_secs(retry_secs)).await;
                retry_secs = (retry_secs * 2).min(60);
            }
        }
    }
}

async fn run_read_loop(
    state: &AppState,
    mqtt_tx: &std::sync::mpsc::Sender<String>,
    http_tx: &tokio::sync::mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();
    let mut config_rx = state.subscribe();

    // Open serial port with O_NONBLOCK for async
    let path = std::ffi::CString::new(config.uart.port.as_str())?;
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK) };
    if fd < 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("Cannot open {}: {}", config.uart.port, err).into());
    }

    // Verify it's a real TTY device
    if unsafe { libc::isatty(fd) } != 1 {
        unsafe { libc::close(fd) };
        return Err(format!("{} is not a TTY device", config.uart.port).into());
    }

    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    configure_serial(&file, config.uart.baudrate)?;

    // Wrap in AsyncFd for epoll-backed async I/O
    let async_fd = AsyncFd::new(file)?;
    let mut buffer = Vec::with_capacity(1024);

    println!(
        "[UART] Opened {} @ {} baud (async/epoll)",
        config.uart.port, config.uart.baudrate
    );

    loop {
        tokio::select! {
            // Config changed → reconnect with new settings
            _ = config_rx.changed() => {
                println!("[UART] Config changed, reconnecting...");
                return Ok(());
            }

            // UART readable (epoll wakeup)
            result = async_fd.readable() => {
                let mut guard = result?;

                match guard.try_io(|inner| read_line_nonblocking(inner.get_ref(), &mut buffer)) {
                    Ok(Ok(Some(line))) => {
                        println!("[UART] Received: {}", line);
                        let json = format_uart_json(&line);
                        let _ = mqtt_tx.send(json.clone());
                        let _ = http_tx.try_send(json);
                    }
                    Ok(Ok(None)) => {
                        // Partial data, yield to avoid busy loop
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Ok(Err(e)) => return Err(e.into()),
                    Err(_would_block) => {}
                }
            }
        }
    }
}

fn read_line_nonblocking(
    file: &std::fs::File,
    buffer: &mut Vec<u8>,
) -> std::io::Result<Option<String>> {
    use std::io::Read;
    let mut tmp = [0u8; 256];

    loop {
        match (&*file).read(&mut tmp) {
            Ok(0) => return Ok(None),
            Ok(n) => {
                buffer.extend_from_slice(&tmp[..n]);
                // Check for newline
                if let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                    let line = String::from_utf8_lossy(&buffer[..pos]).to_string();
                    buffer.drain(..=pos);
                    if !line.is_empty() {
                        return Ok(Some(line));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        }
    }
}

fn format_uart_json(line: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Escape JSON special chars + control characters (RFC 8259)
    let mut escaped = String::with_capacity(line.len());
    for c in line.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => escaped.push(c),
        }
    }

    format!(r#"{{"type":"uart","data":"{}","ts":{}}}"#, escaped, ts)
}

/// Configure serial port baudrate using libc termios
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
            let err = std::io::Error::last_os_error();
            return Err(format!("tcgetattr failed: {}", err).into());
        }

        // Raw mode: no echo, no canonical, no signal chars
        libc::cfmakeraw(&mut tios);

        // Set baudrate
        libc::cfsetispeed(&mut tios, speed);
        libc::cfsetospeed(&mut tios, speed);

        // 8N1
        tios.c_cflag &= !libc::CSIZE;
        tios.c_cflag |= libc::CS8;
        tios.c_cflag &= !libc::PARENB;
        tios.c_cflag &= !libc::CSTOPB;
        tios.c_cflag |= libc::CREAD | libc::CLOCAL;

        // VMIN=1, VTIME=0 → return as soon as 1 byte available
        tios.c_cc[libc::VMIN] = 1;
        tios.c_cc[libc::VTIME] = 0;

        if libc::tcsetattr(fd, libc::TCSANOW, &tios) != 0 {
            let err = std::io::Error::last_os_error();
            return Err(format!("tcsetattr failed: {}", err).into());
        }
    }

    Ok(())
}
