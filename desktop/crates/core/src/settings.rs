//! App-side settings (things the AirPods don't store themselves), kept in
//! `config.toml` under the platform's config directory.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectPolicy {
    /// Take the AirPods over whenever this PC starts playing.
    #[default]
    Auto,
    /// Only reconnect when this PC was the last one they were used with.
    LastUsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrayBattery {
    Off,
    #[default]
    Percent,
    Ring,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Pause when an AirPod comes out, resume when it goes back in.
    pub ear_detection: bool,
    /// Output device to play on while the AirPods are out, like an iPhone's speaker.
    /// `None` leaves the choice to the sound server.
    pub fallback_output: Option<String>,
    pub connect: ConnectPolicy,
    pub tray_battery: TrayBattery,
    pub notify_connected: bool,
    /// Notify with a "take back" action when another device takes the AirPods.
    pub notify_takeover: bool,
    /// Media volume while conversational awareness hears you speak, in percent.
    pub conversation_volume: u8,
    pub alt_head_tracking: bool,
    /// How other devices name this one ("Auf … übertragen"); `None` uses [`default_device_name`].
    pub device_name: Option<String>,
}

/// The computer's own name: the hostname on Linux.
pub fn default_device_name() -> String {
    let host = std::fs::read_to_string("/etc/hostname")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty());
    host.unwrap_or_else(|| "Computer".into())
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ear_detection: true,
            fallback_output: None,
            connect: ConnectPolicy::default(),
            tray_battery: TrayBattery::default(),
            notify_connected: true,
            notify_takeover: true,
            conversation_volume: 40,
            alt_head_tracking: false,
            device_name: None,
        }
    }
}

impl Settings {
    pub fn device_name(&self) -> String {
        self.device_name.clone().unwrap_or_else(default_device_name)
    }

    pub fn path() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("droidpods").join("config.toml"))
    }

    /// Missing or unreadable settings fall back to the defaults.
    pub fn load() -> Self {
        let Some(path) = Self::path() else { return Self::default() };
        match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("ignoring {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path().ok_or(std::io::ErrorKind::NotFound)?;
        std::fs::create_dir_all(path.parent().expect("config path has a parent"))?;
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}
