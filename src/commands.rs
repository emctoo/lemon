use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Command {
    Copy { text: String },
    Paste,
    Open { url: String },
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
}
