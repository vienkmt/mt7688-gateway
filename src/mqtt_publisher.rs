use crate::config::AppState;
use crate::system_info::SystemInfo;
use rumqttc::{Client, MqttOptions, QoS, Transport};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Start MQTT publisher in background thread
pub fn start_background(state: Arc<AppState>, uart_rx: Receiver<String>) {
    thread::spawn(move || loop {
        let config = state.get();
        if !config.mqtt.enabled {
            thread::sleep(Duration::from_secs(5));
            // Drain UART channel to avoid backpressure
            while uart_rx.try_recv().is_ok() {}
            continue;
        }
        if let Err(e) = run_publish_loop(&state, &uart_rx) {
            eprintln!("[MQTT] Error: {}. Retrying in 10s...", e);
            thread::sleep(Duration::from_secs(10));
        }
    });
}

/// Connect to broker and publish system stats + UART data in a loop
fn run_publish_loop(state: &AppState, uart_rx: &Receiver<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config = state.get();
    let version = state.version();

    // Unique client_id per connection to avoid broker rejecting duplicate
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % 100000)
        .unwrap_or(0);
    let client_id = format!("{}-{}", config.mqtt.client_id, ts);
    let mut opts = MqttOptions::new(&client_id, &config.mqtt.broker, config.mqtt.port);
    opts.set_keep_alive(Duration::from_secs(30));

    // TLS mode: use rustls with Mozilla CA roots
    if config.mqtt.tls {
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        opts.set_transport(Transport::tls_with_config(
            rumqttc::TlsConfiguration::Rustls(Arc::new(tls_config)),
        ));
    }

    let (client, mut connection) = Client::new(opts, 10);

    // Connection event loop in separate thread
    let _conn_handle = thread::spawn(move || {
        for notification in connection.iter() {
            if let Err(e) = notification {
                eprintln!("[MQTT] Connection error: {}", e);
                return;
            }
        }
    });

    let proto = if config.mqtt.tls { "MQTTS" } else { "MQTT" };
    println!("[MQTT] Connected to {}:{} ({})", config.mqtt.broker, config.mqtt.port, proto);
    println!("[MQTT] Publishing to '{}' every {}s", config.mqtt.topic, config.general.interval_secs);

    let tick = Duration::from_millis(100);
    let mut elapsed_ms: u64 = 0;
    let interval_ms = config.general.interval_secs.max(1) * 1000;

    loop {
        thread::sleep(tick);
        elapsed_ms += 100;

        // Publish any pending UART messages (non-blocking drain)
        while let Ok(uart_json) = uart_rx.try_recv() {
            if let Err(e) = client.publish(&config.mqtt.topic, QoS::AtLeastOnce, false, uart_json.as_bytes()) {
                eprintln!("[MQTT] UART publish failed: {}", e);
                drop(client);
                return Err(e.into());
            }
        }

        // Reconnect if config changed
        if state.version() != version {
            println!("[MQTT] Config changed, reconnecting...");
            drop(client);
            return Ok(());
        }

        // Publish monitor data at configured interval
        if elapsed_ms >= interval_ms {
            elapsed_ms = 0;
            let payload = SystemInfo::collect().to_json();
            if let Err(e) = client.publish(&config.mqtt.topic, QoS::AtLeastOnce, false, payload.as_bytes()) {
                eprintln!("[MQTT] Publish failed: {}", e);
                drop(client);
                return Err(e.into());
            }
        }
    }
}
