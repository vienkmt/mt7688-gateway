//! Các kênh truyền dữ liệu (outbound + bidirectional)
//! MQTT: publish dữ liệu UART tới broker
//! HTTP: POST dữ liệu UART tới server
//! TCP: server + client song hướng (gửi dữ liệu + nhận lệnh)
//! Buffer: lưu dữ liệu offline khi mất kết nối
//! Reconnect: tự kết nối lại với exponential backoff

pub mod buffer;
pub mod http_pub;
pub mod mqtt;
pub mod reconnect;
pub mod tcp;
