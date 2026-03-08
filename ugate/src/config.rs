//! Quản lý cấu hình qua UCI (hệ thống config gốc của OpenWrt)
//! File config: /etc/config/ugate
//! Hỗ trợ hot-reload: thay đổi config sẽ thông báo tới tất cả subscriber qua watch channel

use crate::uci::Uci;
use std::sync::{mpsc, RwLock};
use tokio::sync::watch;

const UCI_PKG: &str = "ugate";

#[derive(Clone, Debug)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub http: HttpConfig,
    pub tcp: TcpConfig,
    pub uart: UartConfig,
    pub gpio: GpioConfig,
    pub web: WebConfig,
    pub general: GeneralConfig,
}

#[derive(Clone, Debug)]
pub struct MqttConfig {
    pub enabled: bool,
    pub broker: String,
    pub port: u16,
    pub tls: bool,
    pub topic: String,
    pub sub_topic: String,
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub qos: u8,
}

#[derive(Clone, Debug)]
pub struct HttpConfig {
    pub enabled: bool,
    pub url: String,
    pub method: HttpMethod,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HttpMethod {
    Post,
    Get,
}

#[derive(Clone, Debug)]
pub struct TcpConfig {
    pub enabled: bool,
    pub mode: TcpMode,
    pub server_port: u16,
    pub client_host: String,
    pub client_port: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TcpMode {
    Server,
    Client,
    Both,
}

#[derive(Clone, Debug)]
pub struct UartConfig {
    pub enabled: bool,
    pub port: String,
    pub baudrate: u32,
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: u8,
    pub frame_mode: FrameMode,
    pub frame_length: u16,
    pub frame_timeout_ms: u16,
    pub gap_ms: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameMode {
    None,
    Frame,
    Modbus,
}

#[derive(Clone, Debug)]
pub struct GpioConfig {
    pub pins: Vec<u8>,
    pub led_pin: u8,
}

#[derive(Clone, Debug)]
pub struct WebConfig {
    pub port: u16,
    pub password: String,
    pub max_ws_connections: u8,
}

#[derive(Clone, Debug)]
pub struct GeneralConfig {
    pub interval_secs: u64,
    pub device_name: String,
}

// --- Defaults ---

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt: MqttConfig::default(),
            http: HttpConfig::default(),
            tcp: TcpConfig::default(),
            uart: UartConfig::default(),
            gpio: GpioConfig::default(),
            web: WebConfig::default(),
            general: GeneralConfig::default(),
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            broker: "broker.emqx.io".into(),
            port: 8883,
            tls: true,
            topic: "ugate/data".into(),
            sub_topic: "ugate/cmd".into(),
            client_id: "ugate-01".into(),
            username: String::new(),
            password: String::new(),
            qos: 1,
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self { enabled: false, url: String::new(), method: HttpMethod::Post }
    }
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: TcpMode::Server,
            server_port: 9000,
            client_host: String::new(),
            client_port: 9000,
        }
    }
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: "/dev/ttyS1".into(),
            baudrate: 115200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: 1,
            frame_mode: FrameMode::None,
            frame_length: 256,
            frame_timeout_ms: 50,
            gap_ms: 20,
        }
    }
}

impl Default for GpioConfig {
    fn default() -> Self {
        Self {
            pins: vec![],
            led_pin: 44,
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: 8888,
            password: "admin".into(),
            max_ws_connections: 4,
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            interval_secs: 3,
            device_name: "ugate".into(),
        }
    }
}

// --- UCI loading ---

#[allow(dead_code)]
fn uci_get_or(key: &str, default: &str) -> String {
    Uci::get(&format!("{}.@general[0].{}", UCI_PKG, key))
        .unwrap_or_else(|_| default.to_string())
}

fn uci_section_get(section: &str, key: &str, default: &str) -> String {
    Uci::get(&format!("{}.@{}[0].{}", UCI_PKG, section, key))
        .unwrap_or_else(|_| default.to_string())
}

impl Config {
    /// Tạo file UCI mặc định nếu chưa có
    pub fn ensure_uci_file() {
        let path = "/etc/config/ugate";
        if std::path::Path::new(path).exists() {
            return;
        }
        log::info!("[Config] Tạo file UCI mặc định: {}", path);
        let content = r#"
config general
    option device_name 'ugate'
    option interval_secs '3'

config mqtt
    option enabled '0'
    option broker 'broker.emqx.io'
    option port '8883'
    option tls '1'
    option topic 'ugate/data'
    option sub_topic 'ugate/cmd'
    option client_id 'ugate-01'
    option username ''
    option password ''
    option qos '1'

config http
    option enabled '0'
    option url ''
    option method 'post'

config tcp
    option enabled '0'
    option mode 'server'
    option server_port '9000'
    option client_host ''
    option client_port '9000'

config uart
    option enabled '1'
    option baudrate '115200'
    option data_bits '8'
    option parity 'none'
    option stop_bits '1'
    option frame_mode 'none'
    option frame_length '256'
    option frame_timeout_ms '50'
    option gap_ms '20'

config gpio
    option led_pin '44'

config web
    option port '8888'
    option password 'admin'
    option max_ws_connections '4'

config upgrade
    option url 'https://example.com/ugate/latest.json'
    option auto_check '0'
"#;
        let _ = std::fs::write(path, content.trim_start());
    }

    /// Lưu config hiện tại vào UCI
    pub fn save_to_uci(&self) {
        let pkg = "ugate";
        // uci set ugate.@section[0].key=value (Uci::set tự ghép key=value)
        let uci_set = |section: &str, key: &str, val: &str| {
            let full_key = format!("{}.@{}[0].{}", pkg, section, key);
            let _ = Uci::set(&full_key, val);
        };

        // General
        uci_set("general", "device_name", &self.general.device_name);
        uci_set("general", "interval_secs", &self.general.interval_secs.to_string());
        // Sync hostname với device_name
        Uci::set("system.@system[0].hostname", &self.general.device_name).ok();
        Uci::commit("system").ok();

        // MQTT
        uci_set("mqtt", "enabled", if self.mqtt.enabled { "1" } else { "0" });
        uci_set("mqtt", "broker", &self.mqtt.broker);
        uci_set("mqtt", "port", &self.mqtt.port.to_string());
        uci_set("mqtt", "tls", if self.mqtt.tls { "1" } else { "0" });
        uci_set("mqtt", "topic", &self.mqtt.topic);
        uci_set("mqtt", "sub_topic", &self.mqtt.sub_topic);
        uci_set("mqtt", "client_id", &self.mqtt.client_id);
        uci_set("mqtt", "username", &self.mqtt.username);
        uci_set("mqtt", "password", &self.mqtt.password);
        uci_set("mqtt", "qos", &self.mqtt.qos.to_string());

        // HTTP
        uci_set("http", "enabled", if self.http.enabled { "1" } else { "0" });
        uci_set("http", "url", &self.http.url);
        uci_set("http", "method", match self.http.method {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
        });

        // TCP
        uci_set("tcp", "enabled", if self.tcp.enabled { "1" } else { "0" });
        uci_set("tcp", "mode", match self.tcp.mode {
            TcpMode::Server => "server",
            TcpMode::Client => "client",
            TcpMode::Both => "both",
        });
        uci_set("tcp", "server_port", &self.tcp.server_port.to_string());
        uci_set("tcp", "client_host", &self.tcp.client_host);
        uci_set("tcp", "client_port", &self.tcp.client_port.to_string());

        // UART
        uci_set("uart", "enabled", if self.uart.enabled { "1" } else { "0" });
        uci_set("uart", "baudrate", &self.uart.baudrate.to_string());
        uci_set("uart", "data_bits", &self.uart.data_bits.to_string());
        uci_set("uart", "parity", match self.uart.parity {
            Parity::None => "none",
            Parity::Even => "even",
            Parity::Odd => "odd",
        });
        uci_set("uart", "stop_bits", &self.uart.stop_bits.to_string());
        uci_set("uart", "frame_mode", match self.uart.frame_mode {
            FrameMode::None => "none",
            FrameMode::Frame => "frame",
            FrameMode::Modbus => "modbus",
        });
        uci_set("uart", "frame_length", &self.uart.frame_length.to_string());
        uci_set("uart", "frame_timeout_ms", &self.uart.frame_timeout_ms.to_string());
        uci_set("uart", "gap_ms", &self.uart.gap_ms.to_string());

        // Web
        uci_set("web", "port", &self.web.port.to_string());
        uci_set("web", "password", &self.web.password);
        uci_set("web", "max_ws_connections", &self.web.max_ws_connections.to_string());

        let _ = Uci::commit(pkg);
        log::info!("[Config] Đã lưu vào UCI");
    }

    /// Load config from UCI `/etc/config/ugate`
    pub fn load() -> Self {
        Self::ensure_uci_file();
        let mut cfg = Config::default();

        // MQTT
        cfg.mqtt.enabled = uci_section_get("mqtt", "enabled", "0") == "1";
        cfg.mqtt.broker = uci_section_get("mqtt", "broker", &cfg.mqtt.broker);
        cfg.mqtt.port = uci_section_get("mqtt", "port", "8883").parse().unwrap_or(8883);
        cfg.mqtt.tls = uci_section_get("mqtt", "tls", "1") == "1";
        cfg.mqtt.topic = uci_section_get("mqtt", "topic", &cfg.mqtt.topic);
        cfg.mqtt.sub_topic = uci_section_get("mqtt", "sub_topic", &cfg.mqtt.sub_topic);
        cfg.mqtt.client_id = uci_section_get("mqtt", "client_id", &cfg.mqtt.client_id);
        cfg.mqtt.username = uci_section_get("mqtt", "username", "");
        cfg.mqtt.password = uci_section_get("mqtt", "password", "");
        cfg.mqtt.qos = uci_section_get("mqtt", "qos", "1").parse().unwrap_or(1);

        // HTTP
        cfg.http.enabled = uci_section_get("http", "enabled", "0") == "1";
        cfg.http.url = uci_section_get("http", "url", "");
        cfg.http.method = match uci_section_get("http", "method", "post").as_str() {
            "get" => HttpMethod::Get,
            _ => HttpMethod::Post,
        };

        // TCP
        cfg.tcp.enabled = uci_section_get("tcp", "enabled", "0") == "1";
        cfg.tcp.mode = match uci_section_get("tcp", "mode", "server").as_str() {
            "client" => TcpMode::Client,
            "both" => TcpMode::Both,
            _ => TcpMode::Server,
        };
        cfg.tcp.server_port = uci_section_get("tcp", "server_port", "9000").parse().unwrap_or(9000);
        cfg.tcp.client_host = uci_section_get("tcp", "client_host", "");
        cfg.tcp.client_port = uci_section_get("tcp", "client_port", "9000").parse().unwrap_or(9000);

        // UART
        cfg.uart.enabled = uci_section_get("uart", "enabled", "1") == "1";
        cfg.uart.port = uci_section_get("uart", "port", &cfg.uart.port);
        cfg.uart.baudrate = uci_section_get("uart", "baudrate", "115200").parse().unwrap_or(115200);
        cfg.uart.data_bits = uci_section_get("uart", "data_bits", "8").parse().unwrap_or(8);
        cfg.uart.parity = match uci_section_get("uart", "parity", "none").as_str() {
            "even" => Parity::Even,
            "odd" => Parity::Odd,
            _ => Parity::None,
        };
        cfg.uart.stop_bits = uci_section_get("uart", "stop_bits", "1").parse().unwrap_or(1);
        cfg.uart.frame_mode = match uci_section_get("uart", "frame_mode", "none").as_str() {
            "frame" => FrameMode::Frame,
            "modbus" => FrameMode::Modbus,
            _ => FrameMode::None,
        };
        cfg.uart.frame_length = uci_section_get("uart", "frame_length", "256").parse().unwrap_or(256);
        cfg.uart.frame_timeout_ms = uci_section_get("uart", "frame_timeout_ms", "50").parse().unwrap_or(50);
        cfg.uart.gap_ms = uci_section_get("uart", "gap_ms", "20").parse().unwrap_or(20);

        // GPIO
        cfg.gpio.led_pin = uci_section_get("gpio", "led_pin", "44").parse().unwrap_or(44);
        // Parse pins list from UCI
        if let Ok(pins_str) = Uci::get(&format!("{}.@gpio[0].pins", UCI_PKG)) {
            cfg.gpio.pins = pins_str.split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
        }

        // Web
        cfg.web.port = uci_section_get("web", "port", "8888").parse().unwrap_or(8888);
        cfg.web.password = uci_section_get("web", "password", "admin");
        cfg.web.max_ws_connections = uci_section_get("web", "max_ws_connections", "4").parse().unwrap_or(4);

        // General
        cfg.general.interval_secs = uci_section_get("general", "interval_secs", "3").parse().unwrap_or(3);
        cfg.general.device_name = uci_section_get("general", "device_name", "ugate");

        log::info!("[Config] Loaded: UART={}@{} MQTT={} HTTP={} TCP={}",
            cfg.uart.port, cfg.uart.baudrate,
            if cfg.mqtt.enabled { &cfg.mqtt.broker } else { "off" },
            if cfg.http.enabled { &cfg.http.url } else { "off" },
            if cfg.tcp.enabled { "on" } else { "off" });

        cfg
    }
}

/// Shared app state with config hot-reload support
pub struct AppState {
    config: RwLock<Config>,
    config_tx: watch::Sender<()>,
    mqtt_notify: RwLock<Option<mpsc::Sender<()>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let (config_tx, _) = watch::channel(());
        Self {
            config: RwLock::new(config),
            config_tx,
            mqtt_notify: RwLock::new(None),
        }
    }

    pub fn set_mqtt_notifier(&self, tx: mpsc::Sender<()>) {
        *self.mqtt_notify.write().unwrap() = Some(tx);
    }

    pub fn get(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.config_tx.subscribe()
    }

    pub fn update(&self, new_config: Config) {
        *self.config.write().unwrap() = new_config;
        let _ = self.config_tx.send(());
        if let Some(tx) = self.mqtt_notify.read().unwrap().as_ref() {
            let _ = tx.send(());
        }
        log::info!("[Config] Updated, publishers will reconnect");
    }
}
