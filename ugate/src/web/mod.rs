//! Web server module: HTTP API + WebSocket real-time
//! tiny-http phục vụ REST API và static files
//! tungstenite xử lý WebSocket cho dữ liệu real-time và lệnh điều khiển

pub mod auth;
pub mod server;
pub mod status;
pub mod ws;
