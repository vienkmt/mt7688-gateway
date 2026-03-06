use crate::config::AppState;
use crate::system_info::SystemInfo;
use rumqttc::{Client, MqttOptions, QoS, Transport};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Sync MQTT publisher running in dedicated thread
pub fn run_sync(
    state: Arc<AppState>,
    uart_rx: std::sync::mpsc::Receiver<String>,
    config_rx: std::sync::mpsc::Receiver<()>,
) {
    loop {
        let config = state.get();
        if !config.mqtt.enabled {
            // Wait for config change instead of polling
            let _ = config_rx.recv();
            while uart_rx.try_recv().is_ok() {} // drain
            continue;
        }
        if let Err(e) = run_publish_loop(&state, &uart_rx, &config_rx) {
            eprintln!("[MQTT] Error: {}. Retrying in 10s...", e);
            std::thread::sleep(Duration::from_secs(10));
        }
    }
}

fn run_publish_loop(
    state: &AppState,
    uart_rx: &std::sync::mpsc::Receiver<String>,
    config_rx: &std::sync::mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();

    // Random suffix để broker không giữ session cũ (reconnect nhanh hơn)
    let random_id: u32 = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u32)
        ^ (std::process::id() << 16);
    let client_id = format!("{}-{:08x}", config.mqtt.client_id, random_id);

    let mut opts = MqttOptions::new(&client_id, &config.mqtt.broker, config.mqtt.port);
    opts.set_keep_alive(Duration::from_secs(30));

    if config.mqtt.tls {
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        opts.set_transport(Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
            Arc::new(tls_config),
        )));
    }

    let (client, mut connection) = Client::new(opts, 10);

    let proto = if config.mqtt.tls { "MQTTS" } else { "MQTT" };
    println!("[MQTT] Connecting to {}:{} ({})...", config.mqtt.broker, config.mqtt.port, proto);

    // Track connection status: 0=connecting, 1=connected, 2=dead
    let conn_state = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let conn_state_clone = conn_state.clone();

    // Spawn connection thread to handle network I/O
    std::thread::spawn(move || {
        for notification in connection.iter() {
            match notification {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("[MQTT] Connected!");
                    conn_state_clone.store(1, Ordering::Relaxed);
                }
                Ok(_) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => {
                    eprintln!("[MQTT] Connection error: {}", e);
                    conn_state_clone.store(2, Ordering::Relaxed);
                    return;
                }
            }
        }
        conn_state_clone.store(2, Ordering::Relaxed);
    });

    // Wait for ConnAck (max 10s)
    for _ in 0..100 {
        match conn_state.load(Ordering::Relaxed) {
            1 => break,                                    // Connected
            2 => return Err("Connection failed".into()),   // Dead
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    if conn_state.load(Ordering::Relaxed) != 1 {
        return Err("Connection timeout".into());
    }

    println!("[MQTT] Publishing to '{}' every {}s", config.mqtt.topic, config.general.interval_secs);

    let topic = config.mqtt.topic.clone();
    let interval_secs = config.general.interval_secs.max(1) as u64;
    let mut last_sys_info = Instant::now();

    loop {
        // Short timeout for faster config change response
        match uart_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(json) => {
                println!("[MQTT] UART: {}", &json[..json.len().min(50)]);
                if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, false, json.as_bytes()) {
                    eprintln!("[MQTT] Publish err: {}", e);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("UART channel closed".into());
            }
        }

        // Periodic system info
        if last_sys_info.elapsed().as_secs() >= interval_secs {
            last_sys_info = Instant::now();
            let payload = SystemInfo::collect().to_json();
            if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes()) {
                eprintln!("[MQTT] Sys publish err: {}", e);
            }
        }

        // Config changed notification (no polling)
        if config_rx.try_recv().is_ok() {
            println!("[MQTT] Config changed");
            return Ok(());
        }

        // Connection dead, need reconnect
        if conn_state.load(Ordering::Relaxed) == 2 {
            return Err("Connection lost".into());
        }
    }
}
