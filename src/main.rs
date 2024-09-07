use structopt::StructOpt;
use tracing::info;

mod client;
mod clipboard;
mod commands;
mod hub;
mod server;

#[derive(Debug, StructOpt)]
#[structopt(name = "lemonade", about = "A Rust implementation of Lemonade")]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "server")]
    Server {
        #[structopt(long, default_value = "0.0.0.0")]
        host: String,
        #[structopt(long, default_value = "12489")]
        port: u16,
        #[structopt(long, default_value = "localhost")]
        hub_host: String,
        #[structopt(long, default_value = "12491")]
        hub_port: u16,
    },

    #[structopt(name = "hub")]
    Hub {
        #[structopt(long, default_value = "0.0.0.0")]
        host: String,
        #[structopt(long, default_value = "12491")]
        port: u16,
        #[structopt(long)]
        ntfy_topic: Option<String>,
        #[structopt(long, default_value = "ntfy.sh")]
        ntfy_host: String,
    },

    #[structopt(name = "open")]
    Open {
        url: String,
        #[structopt(long, default_value = "localhost")]
        host: String,
        #[structopt(long, default_value = "12489")]
        port: u16,
    },

    #[structopt(name = "copy")]
    Copy {
        text: String,
        #[structopt(long, default_value = "localhost")]
        host: String,
        #[structopt(long, default_value = "12489")]
        port: u16,
    },

    #[structopt(name = "paste")]
    Paste {
        #[structopt(long, default_value = "localhost")]
        host: String,
        #[structopt(long, default_value = "12489")]
        port: u16,
    },

    #[structopt(name = "client")]
    Client {
        #[structopt(long, default_value = "localhost")]
        host: String,
        #[structopt(long, default_value = "12490")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let opt = Opt::from_args();

    match opt.cmd {
        Command::Server {
            host,
            port,
            hub_host,
            hub_port,
        } => {
            info!("Starting server on {}:{}", host, port);
            server::run_server(host, port, hub_host, hub_port).await?;
        }
        Command::Hub {
            host,
            port,
            ntfy_topic,
            ntfy_host,
        } => {
            info!("Starting hub server on {}:{}", host, port);
            hub::run_hub_server(host, port, ntfy_topic, ntfy_host).await?;
        }
        Command::Open { url, host, port } => {
            info!("Opening URL: {}", url);
            client::LemonadeClient::new(host, port).open(&url).await?;
        }
        Command::Copy { text, host, port } => {
            info!("Copying text to clipboard");
            client::LemonadeClient::new(host, port).copy(&text).await?;
        }
        Command::Paste { host, port } => {
            info!("Pasting from clipboard");
            let text = client::LemonadeClient::new(host, port).paste().await?;
            println!("{}", text);
        }
        Command::Client { host, port } => {
            info!("Starting client in listening mode");
            client::LemonadeClient::new(host, port).start().await?;
        }
    }

    Ok(())
}

