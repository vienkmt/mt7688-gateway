//! WiFi LED control for MT7688AN (GPIO44 = Linux GPIO460)
//! Blinks to indicate app is running

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

const WIFI_LED_GPIO: u32 = 460;

/// Initialize WiFi LED GPIO as output
pub fn init() {
    let gpio_path = format!("/sys/class/gpio/gpio{}", WIFI_LED_GPIO);

    // Export if not exists
    if !Path::new(&gpio_path).exists() {
        let _ = fs::write("/sys/class/gpio/export", WIFI_LED_GPIO.to_string());
        thread::sleep(Duration::from_millis(50)); // Wait for sysfs
    }

    // Set as output
    let _ = fs::write(format!("{}/direction", gpio_path), "out");
}

/// Turn LED ON (active-low: write 0)
fn on() {
    let _ = fs::write(format!("/sys/class/gpio/gpio{}/value", WIFI_LED_GPIO), "0");
}

/// Turn LED OFF (write 1)
fn off() {
    let _ = fs::write(format!("/sys/class/gpio/gpio{}/value", WIFI_LED_GPIO), "1");
}

/// Start background thread that blinks LED every interval_ms
pub fn start_heartbeat(interval_ms: u64) {
    thread::spawn(move || {
        loop {
            on();
            thread::sleep(Duration::from_millis(100)); // Short flash
            off();
            thread::sleep(Duration::from_millis(interval_ms - 100));
        }
    });
}
