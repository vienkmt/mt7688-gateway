use crate::config::AppState;
use crate::system_info::SystemInfo;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Async HTTP publisher - receives UART messages via mpsc and POSTs to configured URL
pub async fn run(state: Arc<AppState>, mut uart_rx: mpsc::Receiver<String>) {
    loop {
        let config = state.get();
        if !config.http.enabled || config.http.url.is_empty() {
            tokio::time::sleep(Duration::from_secs(5)).await;
            // Drain channel to avoid backpressure
            while uart_rx.try_recv().is_ok() {}
            continue;
        }
        if let Err(e) = run_publish_loop(&state, &mut uart_rx).await {
            eprintln!("[HTTP] Error: {}. Retrying in 10s...", e);
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}

async fn run_publish_loop(
    state: &AppState,
    uart_rx: &mut mpsc::Receiver<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = state.get();
    let mut config_watch = state.subscribe();

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();

    println!(
        "[HTTP] Publishing to '{}' every {}s",
        config.http.url, config.general.interval_secs
    );

    let mut interval =
        tokio::time::interval(Duration::from_secs(config.general.interval_secs.max(1)));

    loop {
        tokio::select! {
            // Config changed → reload
            _ = config_watch.changed() => {
                println!("[HTTP] Config changed, reloading...");
                return Ok(());
            }

            // UART message received → POST immediately
            Some(json) = uart_rx.recv() => {
                {
                    let url = config.http.url.clone();
                    let agent = agent.clone();

                    // Run blocking HTTP POST in spawn_blocking
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = agent
                            .post(&url)
                            .set("Content-Type", "application/json")
                            .send_string(&json)
                        {
                            eprintln!("[HTTP] UART POST failed: {}", e);
                        }
                    });
                }
            }

            // Periodic system stats
            _ = interval.tick() => {
                let payload = SystemInfo::collect().to_json();
                let url = config.http.url.clone();
                let agent = agent.clone();

                tokio::task::spawn_blocking(move || {
                    if let Err(e) = agent
                        .post(&url)
                        .set("Content-Type", "application/json")
                        .send_string(&payload)
                    {
                        eprintln!("[HTTP] POST failed: {}", e);
                    }
                });
            }
        }
    }
}
