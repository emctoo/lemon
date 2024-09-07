use crate::commands::{Command, Response};
use arboard::Clipboard;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{error, info};

pub struct LemonadeClient {
    host: String,
    port: u16,
}

impl LemonadeClient {
    pub fn new(host: String, port: u16) -> Self {
        LemonadeClient { host, port }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;
        let mut clipboard = Clipboard::new()?;

        loop {
            let mut buffer = vec![0; 1024];
            match stream.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    let response: Response = serde_json::from_slice(&buffer[..n])?;
                    if response.success {
                        clipboard.set_text(response.message.clone())?;
                        info!("Updated local clipboard: {}", response.message);
                    } else {
                        error!("Error from server: {}", response.message);
                    }
                }
                Err(e) => {
                    error!("Failed to read from server: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn send_command(&self, command: Command) -> Result<Response, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;

        let request = serde_json::to_string(&command)?;
        stream.write_all(request.as_bytes()).await?;

        let mut buffer = vec![0; 1024];
        let n = stream.read(&mut buffer).await?;

        if n == 0 {
            return Err("Server closed the connection".into());
        }

        let response: Response = serde_json::from_slice(&buffer[..n])?;
        Ok(response)
    }

    pub async fn open(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .send_command(Command::Open {
                url: url.to_string(),
            })
            .await?;
        if response.success {
            info!("Opened URL: {}", url);
        } else {
            error!("Failed to open URL: {}", response.message);
        }
        Ok(())
    }

    pub async fn copy(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .send_command(Command::Copy {
                text: text.to_string(),
            })
            .await?;
        if response.success {
            info!("Copied text to clipboard");
        } else {
            error!("Failed to copy text: {}", response.message);
        }
        Ok(())
    }

    pub async fn paste(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.send_command(Command::Paste).await?;
        if response.success {
            info!("Pasted text from clipboard");
            Ok(response.message)
        } else {
            error!("Failed to paste: {}", response.message);
            Err(response.message.into())
        }
    }
}
