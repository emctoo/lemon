use crate::clipboard::watch_clipboard;
use crate::commands::{Command, Response};
use crate::ntfy::ntfy_subscriber;
use arboard::Clipboard;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

pub async fn run_server(
    port: u16,
    ntfy_topic: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (clipboard_tx, _) = broadcast::channel::<String>(100);

    // Start clipboard watcher
    let clipboard_tx_clone = clipboard_tx.clone();
    tokio::spawn(watch_clipboard(clipboard_tx_clone));

    if let Some(topic) = ntfy_topic {
        tokio::spawn(ntfy_subscriber(topic));
    }

    // Start the original server on the specified port
    let original_server = tokio::spawn(run_original_server(port, clipboard_tx.clone()));

    // Start the new client mode server on port 12490
    let new_client_server = tokio::spawn(run_new_client_server(12490, clipboard_tx));

    // Wait for both servers to complete (which they shouldn't unless there's an error)
    tokio::try_join!(original_server, new_client_server)?;

    Ok(())
}

async fn run_original_server(
    port: u16,
    clipboard_tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Original server listening on port {}", port);

    loop {
        let (socket, _) = listener.accept().await?;
        let clipboard_tx = clipboard_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_original_client(socket, clipboard_tx).await {
                error!("Error handling original client: {}", e);
            }
        });
    }
}

async fn run_new_client_server(
    port: u16,
    clipboard_tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("New client mode server listening on port {}", port);

    loop {
        let (socket, _) = listener.accept().await?;
        let clipboard_tx = clipboard_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_new_client(socket, clipboard_tx).await {
                error!("Error handling new client: {}", e);
            }
        });
    }
}

async fn handle_original_client(
    mut socket: TcpStream,
    clipboard_tx: broadcast::Sender<String>,
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

            info!("Text copied to clipboard");
            Response {
                success: true,
                message: "Text copied to clipboard".to_string(),
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

async fn handle_new_client(
    mut socket: TcpStream,
    clipboard_tx: broadcast::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut clipboard_rx = clipboard_tx.subscribe();
    let mut buffer = [0u8; 1024];

    loop {
        tokio::select! {
            clipboard_content = clipboard_rx.recv() => {
                match clipboard_content {
                    Ok(content) => {
                        let response = Response {
                            success: true,
                            message: content,
                        };
                        let response_json = serde_json::to_string(&response)?;
                        socket.write_all(response_json.as_bytes()).await?;
                    },
                    Err(e) => {
                        error!("Error receiving clipboard content: {}", e);
                    }
                }
            },
            result = socket.read(&mut buffer) => {
                match result {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        // Process the received data if needed
                        info!("Received {} bytes from new client", n);
                    },
                    Err(e) => {
                        error!("Error reading from new client: {}", e);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
