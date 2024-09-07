use crate::clipboard::watch_clipboard;
use crate::commands::{Command, Response};
use arboard::Clipboard;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, error, info};

pub async fn run_server(
    host: String,
    port: u16,
    hub_host: String,
    hub_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let (clipboard_tx, _) = broadcast::channel::<String>(100);

    // Connect to hub
    let hub_stream = TcpStream::connect(format!("{}:{}", hub_host, hub_port)).await?;
    let (hub_reader, hub_writer) = hub_stream.into_split();
    let hub_writer = Arc::new(Mutex::new(hub_writer));

    // Get local IP address
    let local_ip = local_ip_address::local_ip()?;

    // Start clipboard watcher
    let clipboard_tx_clone = clipboard_tx.clone();
    let hub_writer_clone = Arc::clone(&hub_writer);
    tokio::spawn(watch_clipboard(
        clipboard_tx_clone,
        hub_writer_clone,
        local_ip.to_string(),
    ));

    let clipboard_tx_for_hub = clipboard_tx.clone();
    tokio::spawn(handle_hub_messages(hub_reader, clipboard_tx_for_hub));

    // Start the original server on the specified port
    let original_server = tokio::spawn(run_original_server(
        host.clone(),
        port,
        clipboard_tx.clone(),
        Arc::clone(&hub_writer),
    ));
    let _ = original_server.await?;

    // Start the new client mode server on port 12490
    // let new_client_server = tokio::spawn(run_new_client_server(host, 12490, clipboard_tx));

    // Wait for both servers to complete (which they shouldn't unless there's an error)
    // tokio::try_join!(original_server, new_client_server)?;

    Ok(())
}

async fn run_original_server(
    host: String,
    port: u16,
    clipboard_tx: broadcast::Sender<String>,
    hub_writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
    info!("Original server listening on {}:{}", host, port);

    loop {
        let (socket, addr) = listener.accept().await?;
        let clipboard_tx = clipboard_tx.clone();
        let hub_writer = Arc::clone(&hub_writer);
        tokio::spawn(async move {
            if let Err(e) = handle_original_client(socket, addr, clipboard_tx, hub_writer).await {
                error!("Error handling original client: {}", e);
            }
        });
    }
}

// async fn run_new_client_server(
//     host: String,
//     port: u16,
//     clipboard_tx: broadcast::Sender<String>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     let listener = TcpListener::bind(format!("{}:{}", host, port)).await?;
//     info!("New client mode server listening on {}:{}", host, port);
//
//     loop {
//         let (socket, _) = listener.accept().await?;
//         let clipboard_tx = clipboard_tx.clone();
//         tokio::spawn(async move {
//             if let Err(e) = handle_new_client(socket, clipboard_tx).await {
//                 error!("Error handling new client: {}", e);
//             }
//         });
//     }
// }
//
async fn handle_original_client(
    mut socket: TcpStream,
    addr: SocketAddr,
    clipboard_tx: broadcast::Sender<String>,
    hub_writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = [0; 1024];
    let mut clipboard = Clipboard::new()?;

    let n = socket.read(&mut buffer).await?;
    let command: Command = serde_json::from_slice(&buffer[..n])?;

    let response = match command {
        Command::Copy { text } => {
            debug!("set clipboard text: {}", text);
            clipboard.set_text(text.clone())?;

            debug!("send clipboard text to subscribers: {}", text);
            let _ = clipboard_tx.send(text.clone());

            // Send to hub
            let hub_message = format!("{}:{}", addr.ip(), text);
            let mut hub_writer = hub_writer.lock().await;
            hub_writer.write_u32(hub_message.len() as u32).await?;
            hub_writer.write_all(hub_message.as_bytes()).await?;

            info!("Text copied to clipboard and sent to hub");
            Response {
                success: true,
                message: "Text copied to clipboard and sent to hub".to_string(),
            }
        }
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
        Command::Client => {
            error!("Unexpected Client command received");
            Response {
                success: false,
                message: "Unexpected Client command".to_string(),
            }
        }
    };

    let response_json = serde_json::to_string(&response)?;
    socket.write_all(response_json.as_bytes()).await?;
    debug!("bytes written");

    socket.flush().await?; // Ensure all data is sent
    debug!("bytes flushed");

    Ok(())
}

// async fn handle_new_client(
//     mut socket: TcpStream,
//     clipboard_tx: broadcast::Sender<String>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     let mut clipboard_rx = clipboard_tx.subscribe();
//     let mut buffer = [0u8; 1024];
//
//     loop {
//         tokio::select! {
//             clipboard_content = clipboard_rx.recv() => {
//                 match clipboard_content {
//                     Ok(content) => {
//                         let response = Response {
//                             success: true,
//                             message: content,
//                         };
//                         let response_json = serde_json::to_string(&response)?;
//                         socket.write_all(response_json.as_bytes()).await?;
//                     },
//                     Err(e) => {
//                         error!("Error receiving clipboard content: {}", e);
//                     }
//                 }
//             },
//             result = socket.read(&mut buffer) => {
//                 match result {
//                     Ok(0) => break, // Connection closed
//                     Ok(n) => {
//                         // Process the received data if needed
//                         info!("Received {} bytes from new client", n);
//                     },
//                     Err(e) => {
//                         error!("Error reading from new client: {}", e);
//                         break;
//                     }
//                 }
//             }
//         }
//     }
//
//     Ok(())
// }

async fn handle_hub_messages(
    mut hub_reader: tokio::net::tcp::OwnedReadHalf,
    clipboard_tx: broadcast::Sender<String>,
) {
    let mut clipboard = Clipboard::new().expect("Failed to initialize clipboard");
    let local_ip = local_ip_address::local_ip().expect("Failed to get local IP");

    loop {
        let mut size_buffer = [0u8; 4];
        match hub_reader.read_exact(&mut size_buffer).await {
            Ok(_) => {
                let size = u32::from_be_bytes(size_buffer);
                let mut buffer = vec![0u8; size as usize];
                if hub_reader.read_exact(&mut buffer).await.is_ok() {
                    let message = String::from_utf8_lossy(&buffer).to_string();
                    let parts: Vec<&str> = message.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let source_ip = parts[0];
                        let content = parts[1];
                        if source_ip != local_ip.to_string() {
                            if let Err(e) = clipboard.set_text(content.to_string()) {
                                error!("Failed to set clipboard: {}", e);
                            } else {
                                info!("Received content from hub, updated clipboard");
                                let _ = clipboard_tx.send(content.to_string());
                            }
                        }
                    } else {
                        // Handle messages from ntfy (they don't have a source IP)
                        if let Err(e) = clipboard.set_text(message.clone()) {
                            error!("Failed to set clipboard: {}", e);
                        } else {
                            info!("Received content from ntfy via hub, updated clipboard");
                            let _ = clipboard_tx.send(message);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error reading from hub: {}", e);
                break;
            }
        }
    }
}
