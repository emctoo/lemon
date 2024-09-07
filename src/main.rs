use structopt::StructOpt;
use tracing::info;

mod client;
mod clipboard;
mod commands;
mod hub;
mod ntfy;
mod server;

#[derive(Debug, StructOpt)]
#[structopt(name = "lemonade", about = "A Rust implementation of Lemonade")]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,

    #[structopt(long, default_value = "12489")]
    port: u16,

    #[structopt(long, default_value = "localhost")]
    host: String,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "server")]
    Server {
        #[structopt(long)]
        ntfy_topic: Option<String>,
    },

    #[structopt(name = "open")]
    Open { url: String },

    #[structopt(name = "copy")]
    Copy { text: String },

    #[structopt(name = "paste")]
    Paste,

    #[structopt(name = "client")]
    Client,

    #[structopt(name = "hub")]
    Hub,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let opt = Opt::from_args();

    match opt.cmd {
        Command::Server { ntfy_topic } => {
            info!("Starting server on port {}", opt.port);
            server::run_server(opt.port, ntfy_topic).await?;
        }
        Command::Open { url } => {
            info!("Opening URL: {}", url);
            client::LemonadeClient::new(opt.host, opt.port)
                .open(&url)
                .await?;
        }
        Command::Copy { text } => {
            info!("Copying text to clipboard");
            client::LemonadeClient::new(opt.host, opt.port)
                .copy(&text)
                .await?;
        }
        Command::Paste => {
            info!("Pasting from clipboard");
            let text = client::LemonadeClient::new(opt.host, opt.port)
                .paste()
                .await?;
            println!("{}", text);
        }
        Command::Client => {
            info!("Starting client in listening mode");
            client::LemonadeClient::new(opt.host, 12490).start().await?;
        }
        Command::Hub => {
            info!("Starting hub server on port 12491");
            hub::run_hub_server().await?;
        }
    }

    Ok(())
}
