use crate::commands::{Command, Response};
use arboard::Clipboard;
use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info};

pub async fn run_server(
    port: u16,
    ntfy_topic: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Server listening on port {}", port);

    if let Some(topic) = ntfy_topic {
        tokio::spawn(ntfy_subscriber(topic));
    }

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket).await {
                error!("Error handling client: {}", e);
            }
        });
    }
}

async fn handle_client(mut socket: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    let mut clipboard = Clipboard::new()?;

    let n = socket.read(&mut buffer).await?;
    let command: Command = serde_json::from_slice(&buffer[..n])?;

    let response = match command {
        Command::Copy { text } => match clipboard.set_text(text) {
            Ok(_) => {
                info!("Text copied to clipboard");
                Response {
                    success: true,
                    message: "Text copied to clipboard".to_string(),
                }
            }
            Err(e) => {
                error!("Failed to copy to clipboard: {}", e);
                Response {
                    success: false,
                    message: format!("Failed to copy: {}", e),
                }
            }
        },
        Command::Paste => match clipboard.get_text() {
            Ok(text) => {
                info!("Text pasted from clipboard");
                Response {
                    success: true,
                    message: text,
                }
            }
            Err(e) => {
                error!("Failed to paste from clipboard: {}", e);
                Response {
                    success: false,
                    message: format!("Failed to paste: {}", e),
                }
            }
        },
        Command::Open { url } => match open::that(url) {
            Ok(_) => {
                info!("URL opened");
                Response {
                    success: true,
                    message: "URL opened".to_string(),
                }
            }
            Err(e) => {
                error!("Failed to open URL: {}", e);
                Response {
                    success: false,
                    message: format!("Failed to open URL: {}", e),
                }
            }
        },
    };

    let response_json = serde_json::to_string(&response)?;
    socket.write_all(response_json.as_bytes()).await?;

    Ok(())
}
async fn ntfy_subscriber(topic: String) {
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
