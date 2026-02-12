use crate::config::AppState;
use std::io::{BufRead, BufReader};
use std::os::unix::io::FromRawFd;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Start UART reader in background thread
/// Reads lines from serial port, wraps in JSON, sends to MQTT + HTTP channels
pub fn start_background(
    state: Arc<AppState>,
    mqtt_tx: SyncSender<String>,
    http_tx: SyncSender<String>,
) {
    thread::spawn(move || {
        let mut retry_secs = 5u64;
        loop {
            let config = state.get();
            if !config.uart.enabled {
                thread::sleep(Duration::from_secs(10));
                retry_secs = 5;
                continue;
            }
            match run_read_loop(&state, &mqtt_tx, &http_tx) {
                Ok(()) => retry_secs = 5, // config changed, reset backoff
                Err(e) => {
                    eprintln!("[UART] Error: {}. Retrying in {}s...", e, retry_secs);
                    thread::sleep(Duration::from_secs(retry_secs));
                    retry_secs = (retry_secs * 2).min(60); // backoff up to 60s
                }
            }
        }
    });
}

/// Open serial port, configure baudrate via termios, read lines
fn run_read_loop(
    state: &AppState,
    mqtt_tx: &SyncSender<String>,
    http_tx: &SyncSender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = state.get();
    let version = state.version();

    let path = std::ffi::CString::new(config.uart.port.as_str())?;
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK) };
    if fd < 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("Cannot open {}: {}", config.uart.port, err).into());
    }

    // Verify it's a real TTY device
    if unsafe { libc::isatty(fd) } != 1 {
        unsafe { libc::close(fd); }
        return Err(format!("{} is not a TTY device", config.uart.port).into());
    }

    // Clear O_NONBLOCK after open (we want blocking reads)
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
    }

    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    configure_serial(&file, config.uart.baudrate)?;

    println!(
        "[UART] Opened {} @ {} baud",
        config.uart.port, config.uart.baudrate
    );

    let reader = BufReader::new(file);
    for line_result in reader.lines() {
        // Check config change → reconnect with new port/baudrate
        if state.version() != version {
            println!("[UART] Config changed, reconnecting...");
            return Ok(());
        }

        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[UART] Read error: {}", e);
                continue;
            }
        };

        if line.is_empty() {
            continue;
        }

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Escape JSON special chars + all control characters (RFC 8259)
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

        let json = format!(
            r#"{{"type":"uart","data":"{}","ts":{}}}"#,
            escaped, ts
        );

        let _ = mqtt_tx.send(json.clone());
        let _ = http_tx.send(json);
    }

    Err("Serial port closed".into())
}

/// Configure serial port baudrate using libc termios
fn configure_serial(
    file: &std::fs::File,
    baudrate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::io::AsRawFd;
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

        // VMIN=1, VTIME=0 → blocking read, return as soon as 1 byte available
        tios.c_cc[libc::VMIN] = 1;
        tios.c_cc[libc::VTIME] = 0;

        if libc::tcsetattr(fd, libc::TCSANOW, &tios) != 0 {
            let err = std::io::Error::last_os_error();
            return Err(format!("tcsetattr failed: {}", err).into());
        }
    }

    Ok(())
}
