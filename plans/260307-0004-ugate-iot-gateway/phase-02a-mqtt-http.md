# Phase 2a: MQTT + HTTP Channels

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** Phase 1

## Objective

Implement 2 outbound channels: MQTT publish và HTTP POST.
Copy patterns từ vgateway, minimal changes.

## Module Structure

```
ugate/src/channels/
├── mod.rs
├── mqtt.rs       # std::thread sync (from vgateway)
└── http_pub.rs   # tokio::spawn (from vgateway)
```

## Implementation

### 1. channels/mod.rs

```rust
pub mod mqtt;
pub mod http_pub;

pub use mqtt::run_mqtt;
pub use http_pub::run_http_publisher;
```

### 2. channels/mqtt.rs

Copy từ vgateway/mqtt_publisher.rs với additions:
- MQTT auth (username/password optional)
- TLS via rustls

```rust
pub fn run_sync(
    state: Arc<AppState>,
    data_rx: std::sync::mpsc::Receiver<String>,
    config_notify_rx: std::sync::mpsc::Receiver<()>,
) {
    loop {
        let config = state.get();
        if !config.mqtt.enabled {
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }

        let mut options = MqttOptions::new(
            &config.mqtt.client_id,
            &config.mqtt.broker,
            config.mqtt.port,
        );

        // Auth (optional)
        if let (Some(user), Some(pass)) = (&config.mqtt.username, &config.mqtt.password) {
            if !user.is_empty() {
                options.set_credentials(user, pass);
            }
        }

        // TLS (rustls, no OpenSSL)
        if config.mqtt.tls {
            let root_store = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let tls_config = rustls::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            options.set_transport(Transport::tls_with_config(
                TlsConfiguration::Rustls(Arc::new(tls_config)),
            ));
        }

        let (client, mut connection) = Client::new(options, 10);
        // ... publish loop (same as vgateway)
    }
}
```

### 3. channels/http_pub.rs

Copy từ vgateway/http_publisher.rs:

```rust
pub async fn run(
    state: Arc<AppState>,
    mut rx: tokio::sync::mpsc::Receiver<String>,
) {
    while let Some(data) = rx.recv().await {
        let config = state.get();
        if !config.http.enabled {
            continue;
        }

        // spawn_blocking for sync ureq
        let url = config.http.url.clone();
        let _ = tokio::task::spawn_blocking(move || {
            ureq::post(&url)
                .set("Content-Type", "application/json")
                .send_string(&data)
        }).await;
    }
}
```

### 4. Wire in main.rs

```rust
// Channels
let (mqtt_tx, mqtt_rx) = std::sync::mpsc::channel::<String>();
let (http_tx, http_rx) = tokio::sync::mpsc::channel::<String>(64);
let (config_notify_tx, config_notify_rx) = std::sync::mpsc::channel::<()>();

// MQTT in std::thread (workaround MIPS issues)
let mqtt_state = state.clone();
std::thread::spawn(move || {
    channels::run_mqtt(mqtt_state, mqtt_rx, config_notify_rx);
});

// HTTP publisher (tokio task)
tokio::spawn(channels::run_http_publisher(state.clone(), http_rx));
```

## UCI Config

```
config mqtt
    option enabled '1'
    option broker 'mqtt.example.com'
    option port '8883'
    option tls '1'
    option username ''
    option password ''
    option client_id 'ugate_001'
    option topic 'ugate/data'

config http
    option enabled '1'
    option url 'https://api.example.com/data'
```

## Files to Create

| File | Action |
|------|--------|
| ugate/src/channels/mod.rs | Create |
| ugate/src/channels/mqtt.rs | Create (copy from vgateway) |
| ugate/src/channels/http_pub.rs | Create (copy from vgateway) |
| ugate/src/main.rs | Modify - wire channels |

## Dependencies

```toml
rumqttc = "0.24"
ureq = { version = "2", features = ["tls"] }
rustls = "0.22"
webpki-roots = "0.26"
```

## Todo

- [ ] Create channels/mod.rs
- [ ] Copy mqtt.rs từ vgateway, add auth + TLS
- [ ] Copy http_pub.rs từ vgateway
- [ ] Wire channels in main.rs
- [ ] Test MQTT publish (no auth)
- [ ] Test MQTT publish (with auth)
- [ ] Test HTTP POST (HTTPS)
- [ ] Verify config reload works

## Success Criteria

- [ ] MQTT publishes UART data
- [ ] MQTT auth works
- [ ] MQTT TLS works
- [ ] HTTP POSTs UART data
- [ ] HTTPS works
- [ ] Config change triggers reconnect

## Next Phase

Phase 2b: TCP + Reliability
