#![allow(dead_code)]
//! Bộ phân tích lệnh điều khiển GPIO và gửi dữ liệu UART TX
//! Hỗ trợ 2 định dạng:
//!   - Text từ UART: "GPIO:1:ON\n"
//!   - JSON từ WebSocket/TCP/MQTT: {"cmd":"gpio","pin":1,"state":"on"}

/// Commands that can be received from any source
#[derive(Debug, Clone)]
pub enum Command {
    Gpio { pin: u8, state: GpioState },
    UartTx { data: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpioState {
    On,
    Off,
    Toggle,
}

/// Parse UART text command: "GPIO:1:ON\n" or "GPIO:2:TOGGLE\n"
pub fn parse_uart_command(line: &str) -> Option<Command> {
    let parts: Vec<&str> = line.trim().split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    match parts[0].to_uppercase().as_str() {
        "GPIO" => {
            let pin: u8 = parts[1].parse().ok()?;
            let state = match parts[2].to_uppercase().as_str() {
                "ON" | "1" => GpioState::On,
                "OFF" | "0" => GpioState::Off,
                "TOGGLE" | "T" => GpioState::Toggle,
                _ => return None,
            };
            Some(Command::Gpio { pin, state })
        }
        _ => None,
    }
}

/// Parse JSON command: {"cmd":"gpio","pin":1,"state":"on"}
/// or {"cmd":"uart_tx","data":"hello"}
/// Minimal JSON parser — no serde_json dependency
pub fn parse_json_command(json: &str) -> Option<Command> {
    let cmd = json_str_val(json, "cmd")?;
    match cmd.as_str() {
        "gpio" => {
            let pin: u8 = json_str_val(json, "pin")?.parse().ok()?;
            let state = match json_str_val(json, "state")?.to_lowercase().as_str() {
                "on" | "1" => GpioState::On,
                "off" | "0" => GpioState::Off,
                "toggle" | "t" => GpioState::Toggle,
                _ => return None,
            };
            Some(Command::Gpio { pin, state })
        }
        "uart_tx" => {
            let data = json_str_val(json, "data")?;
            Some(Command::UartTx { data })
        }
        _ => None,
    }
}

/// Extract string value for a key from JSON (minimal, no serde)
fn json_str_val(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let pos = json.find(&pattern)?;
    let rest = &json[pos + pattern.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();

    if after.starts_with('"') {
        // String value
        let end = after[1..].find('"')?;
        Some(after[1..end + 1].to_string())
    } else {
        // Number or other value
        let end = after.find(|c: char| c == ',' || c == '}' || c == ' ')?;
        Some(after[..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uart_gpio_on() {
        let cmd = parse_uart_command("GPIO:1:ON\n").unwrap();
        match cmd {
            Command::Gpio { pin, state } => {
                assert_eq!(pin, 1);
                assert_eq!(state, GpioState::On);
            }
            _ => panic!("Expected GPIO command"),
        }
    }

    #[test]
    fn test_parse_uart_gpio_toggle() {
        let cmd = parse_uart_command("gpio:44:toggle").unwrap();
        match cmd {
            Command::Gpio { pin, state } => {
                assert_eq!(pin, 44);
                assert_eq!(state, GpioState::Toggle);
            }
            _ => panic!("Expected GPIO command"),
        }
    }

    #[test]
    fn test_parse_uart_invalid() {
        assert!(parse_uart_command("hello world").is_none());
        assert!(parse_uart_command("GPIO:abc:ON").is_none());
    }

    #[test]
    fn test_parse_json_gpio() {
        let cmd = parse_json_command(r#"{"cmd":"gpio","pin":"1","state":"on"}"#).unwrap();
        match cmd {
            Command::Gpio { pin, state } => {
                assert_eq!(pin, 1);
                assert_eq!(state, GpioState::On);
            }
            _ => panic!("Expected GPIO command"),
        }
    }

    #[test]
    fn test_parse_json_uart_tx() {
        let cmd = parse_json_command(r#"{"cmd":"uart_tx","data":"hello"}"#).unwrap();
        match cmd {
            Command::UartTx { data } => assert_eq!(data, "hello"),
            _ => panic!("Expected UartTx command"),
        }
    }
}
