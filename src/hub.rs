use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{error, info};

pub async fn run_hub_server() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:12491").await?;
    info!("Hub server listening on port 12491");

    let (tx, _) = broadcast::channel::<String>(100);

    loop {
        let (socket, addr) = listener.accept().await?;
        info!("New server connected: {}", addr);

        let tx = tx.clone();
        let mut rx = tx.subscribe();

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

