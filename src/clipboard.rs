use arboard::Clipboard;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info};

pub async fn watch_clipboard(
    tx: broadcast::Sender<String>,
    hub_writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    local_ip: String,
) {
    let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");
    let mut last_content = String::new();

    loop {
        if let Ok(content) = clipboard.get_text() {
            if content != last_content {
                last_content = content.clone();
                let _ = tx.send(content.clone());

                // Send to hub
                let hub_message = format!("{}:{}", local_ip, content);
                let mut hub_writer = hub_writer.lock().await;
                if let Err(e) = hub_writer.write_u32(hub_message.len() as u32).await {
                    error!("Failed to write message length to hub: {}", e);
                } else if let Err(e) = hub_writer.write_all(hub_message.as_bytes()).await {
                    error!("Failed to write message to hub: {}", e);
                } else {
                    info!("Sent clipboard content to hub");
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

