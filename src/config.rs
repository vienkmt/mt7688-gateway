use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

const CONFIG_PATH: &str = "/etc/v3s-monitor.toml";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub http: HttpConfig,
    pub general: GeneralConfig,
    pub uart: UartConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MqttConfig {
    pub enabled: bool,
    pub broker: String,
    pub port: u16,
    pub tls: bool,
    pub topic: String,
    pub client_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct HttpConfig {
    pub enabled: bool,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct GeneralConfig {
    pub interval_secs: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct UartConfig {
    pub enabled: bool,
    pub port: String,
    pub baudrate: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt: MqttConfig::default(),
            http: HttpConfig::default(),
            general: GeneralConfig::default(),
            uart: UartConfig::default(),
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            broker: "broker.emqx.io".into(),
            port: 8883,
            tls: true,
            topic: "vienkmt/v3s".into(),
            client_id: "v3s-monitor".into(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self { enabled: false, url: String::new() }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self { interval_secs: 3 }
    }
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: "/dev/ttyS2".into(),
            baudrate: 115200,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        fs::read_to_string(CONFIG_PATH)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(s) = toml::to_string_pretty(self) {
            if let Err(e) = fs::write(CONFIG_PATH, s) {
                eprintln!("[Config] Save failed: {}", e);
            }
        }
    }
}

/// Shared app state with config hot-reload support
pub struct AppState {
    config: RwLock<Config>,
    version: AtomicU32,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::load();
        println!("[Config] Loaded: MQTT={}:{} TLS={} HTTP={} UART={}@{}",
            config.mqtt.broker, config.mqtt.port, config.mqtt.tls,
            if config.http.enabled { &config.http.url } else { "disabled" },
            config.uart.port, config.uart.baudrate);
        Self { config: RwLock::new(config), version: AtomicU32::new(0) }
    }

    pub fn get(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    pub fn version(&self) -> u32 {
        self.version.load(Ordering::Relaxed)
    }

    pub fn update(&self, new_config: Config) {
        new_config.save();
        *self.config.write().unwrap() = new_config;
        self.version.fetch_add(1, Ordering::Relaxed);
        println!("[Config] Updated, publishers will reconnect");
    }
}
