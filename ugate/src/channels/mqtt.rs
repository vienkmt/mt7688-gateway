//! Kênh MQTT publisher chạy trên OS thread riêng (sync)
//! Dùng rumqttc sync Client vì AsyncClient có vấn đề trên MIPS
//! Hỗ trợ: TLS (rustls), auth (username/password), QoS cấu hình được
//! Tự động reconnect khi mất kết nối hoặc thay đổi config

use crate::config::AppState;
use rumqttc::{Client, MqttOptions, QoS, Transport};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

/// Chạy MQTT publisher trong vòng lặp vô hạn
/// Tự khởi động lại khi lỗi hoặc config thay đổi
pub fn run_sync(
    state: Arc<AppState>,
    data_rx: std::sync::mpsc::Receiver<Vec<u8>>,
    config_rx: std::sync::mpsc::Receiver<()>,
    cmd_tx: std::sync::mpsc::Sender<crate::commands::Command>,
    stats: Arc<crate::web::status::SharedStats>,
) {
    loop {
        let config = state.get();
        if !config.mqtt.enabled {
            stats.mqtt_state.store(0, Ordering::Relaxed); // disabled
            let _ = config_rx.recv();
            while data_rx.try_recv().is_ok() {}
            continue;
        }
        stats.mqtt_state.store(1, Ordering::Relaxed); // disconnected
        if let Err(e) = run_publish_loop(&state, &data_rx, &config_rx, &cmd_tx, &stats) {
            log::error!("[MQTT] Lỗi: {}. Thử lại sau 10s...", e);
            stats.mqtt_state.store(1, Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(10));
        }
    }
}

/// Vòng lặp publish chính: kết nối broker, nhận dữ liệu từ channel, publish
fn run_publish_loop(
    state: &AppState,
    data_rx: &std::sync::mpsc::Receiver<Vec<u8>>,
    config_rx: &std::sync::mpsc::Receiver<()>,
    cmd_tx: &std::sync::mpsc::Sender<crate::commands::Command>,
    stats: &crate::web::status::SharedStats,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();

    // Tạo client ID ngẫu nhiên để broker không giữ session cũ
    let random_suffix: u32 = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0) as u32)
        ^ (std::process::id() << 16);
    let client_id = format!("{}-{:08x}", config.mqtt.client_id, random_suffix);

    let mut opts = MqttOptions::new(&client_id, &config.mqtt.broker, config.mqtt.port);
    opts.set_keep_alive(Duration::from_secs(30));

    // Xác thực (tuỳ chọn)
    if !config.mqtt.username.is_empty() {
        opts.set_credentials(&config.mqtt.username, &config.mqtt.password);
    }

    // TLS qua rustls (không phụ thuộc OpenSSL)
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
    log::info!("[MQTT] Đang kết nối {}:{} ({})...", config.mqtt.broker, config.mqtt.port, proto);

    // Theo dõi trạng thái kết nối: 0=đang kết nối, 1=đã kết nối, 2=mất kết nối
    let conn_state = Arc::new(std::sync::atomic::AtomicU8::new(0));
    let conn_state_clone = conn_state.clone();

    // Thread xử lý I/O mạng cho MQTT + nhận message từ subscribe topic
    let cmd_tx_clone = cmd_tx.clone();
    std::thread::spawn(move || {
        for notification in connection.iter() {
            match notification {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    log::info!("[MQTT] Đã kết nối!");
                    conn_state_clone.store(1, Ordering::Relaxed);
                }
                // Xử lý message nhận từ subscribe topic → chuyển thành Command
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(msg))) => {
                    let payload = String::from_utf8_lossy(&msg.payload);
                    log::debug!("[MQTT] Nhận từ '{}': {}", msg.topic, payload);
                    if let Some(cmd) = crate::commands::parse_json_command(&payload) {
                        let _ = cmd_tx_clone.send(cmd);
                    } else {
                        // Nếu không phải JSON command, gửi raw xuống UART
                        let _ = cmd_tx_clone.send(crate::commands::Command::UartTx {
                            data: payload.into_owned(),
                        });
                    }
                }
                Ok(_) => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(e) => {
                    log::error!("[MQTT] Lỗi kết nối: {}", e);
                    conn_state_clone.store(2, Ordering::Relaxed);
                    return;
                }
            }
        }
        conn_state_clone.store(2, Ordering::Relaxed);
    });

    // Chờ ConnAck tối đa 10 giây
    for _ in 0..100 {
        match conn_state.load(Ordering::Relaxed) {
            1 => break,
            2 => return Err("Kết nối thất bại".into()),
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    if conn_state.load(Ordering::Relaxed) != 1 {
        return Err("Hết thời gian chờ kết nối".into());
    }
    stats.mqtt_state.store(2, Ordering::Relaxed); // connected

    // Chuyển QoS từ config
    let qos = match config.mqtt.qos {
        0 => QoS::AtMostOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtLeastOnce,
    };

    // Subscribe topic để nhận lệnh từ broker → MCU
    if !config.mqtt.sub_topic.is_empty() {
        match client.subscribe(&config.mqtt.sub_topic, qos) {
            Ok(()) => log::info!("[MQTT] Subscribe '{}'", config.mqtt.sub_topic),
            Err(e) => log::error!("[MQTT] Subscribe lỗi: {}", e),
        }
    }

    log::info!("[MQTT] Publish tới '{}' (QoS={})", config.mqtt.topic, config.mqtt.qos);

    let topic = config.mqtt.topic.clone();

    loop {
        // Nhận dữ liệu với timeout ngắn để phản hồi config nhanh
        match data_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(data) => {
                log::debug!("[MQTT] Gửi {} bytes", data.len());
                if let Err(e) = client.publish(&topic, qos, false, data) {
                    log::error!("[MQTT] Lỗi publish: {}", e);
                    stats.mqtt_failed.fetch_add(1, Ordering::Relaxed);
                } else {
                    stats.mqtt_published.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Kênh dữ liệu đã đóng".into());
            }
        }

        // Kiểm tra thay đổi config
        if config_rx.try_recv().is_ok() {
            log::info!("[MQTT] Config thay đổi, kết nối lại...");
            return Ok(());
        }

        // Kiểm tra kết nối còn sống không
        if conn_state.load(Ordering::Relaxed) == 2 {
            return Err("Mất kết nối".into());
        }
    }
}
