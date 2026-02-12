use crate::config::AppState;
use crate::system_info::SystemInfo;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Start HTTP POST publisher in background thread
pub fn start_background(state: Arc<AppState>, uart_rx: Receiver<String>) {
    thread::spawn(move || loop {
        let config = state.get();
        if !config.http.enabled || config.http.url.is_empty() {
            thread::sleep(Duration::from_secs(5));
            while uart_rx.try_recv().is_ok() {}
            continue;
        }
        if let Err(e) = run_publish_loop(&state, &uart_rx) {
            eprintln!("[HTTP] Error: {}. Retrying in 10s...", e);
            thread::sleep(Duration::from_secs(10));
        }
    });
}

/// POST system stats + UART data as JSON to configured URL
fn run_publish_loop(state: &AppState, uart_rx: &Receiver<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config = state.get();
    let version = state.version();
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();

    println!("[HTTP] Publishing to '{}' every {}s", config.http.url, config.general.interval_secs);

    let tick = Duration::from_millis(100);
    let mut elapsed_ms: u64 = 0;
    let interval_ms = config.general.interval_secs.max(1) * 1000;

    loop {
        thread::sleep(tick);
        elapsed_ms += 100;

        // POST any pending UART messages
        while let Ok(uart_json) = uart_rx.try_recv() {
            if let Err(e) = agent
                .post(&config.http.url)
                .set("Content-Type", "application/json")
                .send_string(&uart_json)
            {
                eprintln!("[HTTP] UART POST failed: {}", e);
            }
        }

        if state.version() != version {
            println!("[HTTP] Config changed, reloading...");
            return Ok(());
        }

        // POST monitor data at configured interval
        if elapsed_ms >= interval_ms {
            elapsed_ms = 0;
            let payload = SystemInfo::collect().to_json();
            if let Err(e) = agent
                .post(&config.http.url)
                .set("Content-Type", "application/json")
                .send_string(&payload)
            {
                eprintln!("[HTTP] POST failed: {}", e);
            }
        }
    }
}
