use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Serialize, Deserialize, StructOpt)]
pub enum Command {
    Copy {
        text: String,
    },
    Paste,
    Open {
        url: String,
    },
    #[structopt(name = "client")]
    Client,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
}
