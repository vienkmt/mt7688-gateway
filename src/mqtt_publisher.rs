use crate::config::AppState;
use crate::system_info::SystemInfo;
use rumqttc::{Client, MqttOptions, QoS, Transport};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Sync MQTT publisher running in dedicated thread
pub fn run_sync(state: Arc<AppState>, uart_rx: std::sync::mpsc::Receiver<String>) {
    loop {
        let config = state.get();
        if !config.mqtt.enabled {
            std::thread::sleep(Duration::from_secs(5));
            while uart_rx.try_recv().is_ok() {} // drain
            continue;
        }
        if let Err(e) = run_publish_loop(&state, &uart_rx) {
            eprintln!("[MQTT] Error: {}. Retrying in 10s...", e);
            std::thread::sleep(Duration::from_secs(10));
        }
    }
}

fn run_publish_loop(
    state: &AppState,
    uart_rx: &std::sync::mpsc::Receiver<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 100000)
        .unwrap_or(0);
    let client_id = format!("{}-{}", config.mqtt.client_id, ts);

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

    // Spawn connection thread to handle network I/O
    std::thread::spawn(move || {
        for notification in connection.iter() {
            match notification {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("[MQTT] Connected!");
                }
                Ok(_) => {
                    // Small yield to reduce CPU
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => {
                    eprintln!("[MQTT] Connection error: {}", e);
                    return;
                }
            }
        }
    });

    // Wait a bit for connection to establish
    std::thread::sleep(Duration::from_millis(500));

    println!("[MQTT] Publishing to '{}' every {}s", config.mqtt.topic, config.general.interval_secs);

    let topic = config.mqtt.topic.clone();
    let interval_secs = config.general.interval_secs.max(1) as u64;
    let mut last_sys_info = Instant::now();
    let mut config_check = Instant::now();

    loop {
        // Blocking recv with timeout (saves CPU)
        match uart_rx.recv_timeout(Duration::from_secs(1)) {
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

        // Check config
        if config_check.elapsed().as_secs() >= 2 {
            config_check = Instant::now();
            let new_config = state.get();
            if new_config.mqtt.broker != config.mqtt.broker
                || new_config.mqtt.port != config.mqtt.port
                || new_config.mqtt.tls != config.mqtt.tls
                || !new_config.mqtt.enabled
            {
                println!("[MQTT] Config changed");
                return Ok(());
            }
        }
    }
}
