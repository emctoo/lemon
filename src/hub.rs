use futures_util::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info};

pub async fn run_hub_server(
    host: String,
    port: u16,
    ntfy_topic: Option<String>,
    ntfy_host: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("Hub server listening on {}:{}", host, port);

    let (tx, _) = broadcast::channel::<String>(100);

    if let Some(topic) = ntfy_topic {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = run_ntfy_subscriber(topic, ntfy_host, tx_clone).await {
                error!("Error in ntfy subscriber: {}", e);
            }
        });
    }

    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New server connected: {}", addr);

        let tx = tx.clone();
        let rx = tx.subscribe();

        tokio::spawn(async move {
            if let Err(e) = handle_server(socket, addr, tx, rx).await {
                error!("Error handling server {}: {}", addr, e);
            }
        });
    }
}

async fn handle_server(
    mut socket: TcpStream,
    addr: std::net::SocketAddr,
    tx: broadcast::Sender<String>,
    mut rx: broadcast::Receiver<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut reader, mut writer) = socket.split();

    loop {
        tokio::select! {
            result = reader.read_u32() => {
                match result {
                    Ok(size) => {
                        let mut buffer = vec![0; size as usize];
                        if reader.read_exact(&mut buffer).await.is_ok() {
                            let message = String::from_utf8_lossy(&buffer).to_string();
                            info!("Received message from {}: {}", addr, message);
                            tx.send(message)?;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from server {}: {}", addr, e);
                        break;
                    }
                }
            }
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let size = msg.len() as u32;
                        writer.write_u32(size).await?;
                        writer.write_all(msg.as_bytes()).await?;
                    }
                    Err(e) => {
                        error!("Error receiving broadcast message: {}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_ntfy_subscriber(
    topic: String,
    ntfy_host: String,
    tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url_string = format!("wss://{}/{}/ws", ntfy_host, topic);

    loop {
        match connect_async(&url_string).await {
            Ok((ws_stream, _)) => {
                info!("WebSocket connected to ntfy topic: {}", topic);
                let (_, mut read) = ws_stream.split();

                while let Some(message) = read.next().await {
                    match message {
                        Ok(msg) => {
                            if let Message::Text(text) = msg {
                                let json: serde_json::Value = serde_json::from_str(&text)?;
                                if let Some(message) = json["message"].as_str() {
                                    info!("Received message from ntfy, broadcasting to servers");
                                    tx.send(message.to_string())?;
                                }
                            }
                        }
                        Err(e) => error!("Error receiving message from ntfy: {}", e),
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
