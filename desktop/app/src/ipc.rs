//! Requests the CLI sends to the running app: one JSON object per line over a
//! Unix socket in `$XDG_RUNTIME_DIR`, answered by one line (or a stream for `watch`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Status,
    /// No value: report the mode. `next`/`toggle` are resolved by the service.
    Mode {
        value: Option<String>,
        toggle: Vec<String>,
    },
    Get {
        key: Option<String>,
    },
    Set {
        key: String,
        value: String,
    },
    Connect,
    Disconnect,
    Devices,
    /// Stream [`Event`]s until the client hangs up.
    Watch,
    /// Show the companion window.
    Show,
}

/// Exit codes the CLI uses, also carried in error responses.
pub mod exit {
    pub const NO_DAEMON: i32 = 1;
    pub const INVALID: i32 = 2;
    pub const NOT_CONNECTED: i32 = 3;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Exit code for errors.
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl Response {
    pub fn ok(data: impl Serialize) -> Self {
        Self { data: serde_json::to_value(data).unwrap_or_default(), ..Default::default() }
    }

    pub fn err(code: i32, msg: impl ToString) -> Self {
        Self { error: Some(msg.to_string()), code, ..Default::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    Connect { name: String, battery: String },
    Disconnect,
    Battery { battery: String, low: bool },
    Ear { left: Option<bool>, right: Option<bool> },
    Media { action: String },
    Mode { from: Option<String>, to: String },
    Route { text: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BudJson {
    pub level: u8,
    pub charging: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_ear: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusJson {
    pub model: String,
    pub name: String,
    pub connected: bool,
    pub battery: std::collections::BTreeMap<String, BudJson>,
    pub mode: Option<String>,
}

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    dir.join("betterpods.sock")
}
