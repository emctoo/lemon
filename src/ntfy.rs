use arboard::Clipboard;
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info};

pub async fn ntfy_subscriber(topic: String) {
    let url_string = format!("wss://ntfy.sh/{}/ws", topic);

    loop {
        match connect_async(&url_string).await {
            Ok((ws_stream, _)) => {
                info!("WebSocket connected to ntfy topic: {}", topic);
                let (_, mut read) = ws_stream.split();

                while let Some(message) = read.next().await {
                    match message {
                        Ok(msg) => {
                            if let Message::Text(text) = msg {
                                let json: serde_json::Value = match serde_json::from_str(&text) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error!("Failed to parse JSON: {}", e);
                                        continue;
                                    }
                                };

                                if let Some(message) = json["message"].as_str() {
                                    if let Ok(mut clipboard) = Clipboard::new() {
                                        if let Err(e) = clipboard.set_text(message.to_string()) {
                                            error!("Failed to set clipboard: {}", e);
                                        } else {
                                            info!(
                                                "Received message from ntfy, copied to clipboard"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Error receiving message: {}", e),
                    }
                }
            }
            Err(e) => {
                error!("WebSocket connection error: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}

