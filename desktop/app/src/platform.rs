//! The OS services the app uses, behind one interface per platform.

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(not(target_os = "linux"))]
pub use other::*;

#[derive(Debug, Clone)]
pub struct Output {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct KnownDevice {
    pub address: String,
    pub name: String,
    pub connected: bool,
}

#[cfg(target_os = "linux")]
mod linux {
    use std::time::Duration;

    use betterpods_core::{Command, DeviceState, Settings};
    use betterpods_linux::{audio, media::Media, notify};
    use tokio::sync::{mpsc, watch};

    use super::{KnownDevice, Output};

    pub struct Platform {
        media: Option<Media>,
    }

    impl Platform {
        pub async fn new() -> Self {
            let media = Media::new().await.inspect_err(|e| log::warn!("no session bus, media control off: {e}")).ok();
            Self { media }
        }

        pub async fn run_connection(
            cmd: mpsc::Receiver<Command>,
            state: watch::Sender<DeviceState>,
            settings: watch::Receiver<Settings>,
        ) {
            let name = move || settings.borrow().device_name();
            if let Err(e) = betterpods_linux::supervise(cmd, state, Duration::from_secs(3), name).await {
                log::error!("Bluetooth unavailable: {e}");
            }
        }

        pub async fn playing(&self) -> bool {
            match &self.media {
                Some(m) => m.playing().await,
                None => false,
            }
        }

        pub async fn pause(&self) {
            if let Some(m) = &self.media {
                m.pause().await;
            }
        }

        pub async fn resume(&self) {
            if let Some(m) = &self.media {
                m.resume().await;
            }
        }

        pub async fn forget_paused(&self) {
            if let Some(m) = &self.media {
                m.forget().await;
            }
        }

        /// Calls `on_start` whenever something starts playing on this computer.
        pub async fn watch_playback(&self, on_start: impl Fn()) {
            if let Some(m) = &self.media {
                if let Err(e) = m.watch_starts(on_start).await {
                    log::warn!("can't follow playback: {e}");
                }
            }
        }

        pub async fn route_to_airpods(&self, addr: &str, reconnect: bool) {
            let Ok(a) = addr.parse() else { return };
            let _ = tokio::task::spawn_blocking(move || audio::route_to_airpods(a, reconnect)).await;
        }

        pub async fn route_away(&self, addr: &str, fallback: Option<String>) {
            let Ok(a) = addr.parse() else { return };
            let _ = tokio::task::spawn_blocking(move || audio::route_away(a, fallback.as_deref())).await;
        }

        pub async fn volume(&self) -> Option<u32> {
            tokio::task::spawn_blocking(audio::default_volume).await.ok().flatten()
        }

        pub async fn set_volume(&self, percent: u32) {
            let _ = tokio::task::spawn_blocking(move || audio::set_default_volume(percent)).await;
        }

        pub fn outputs() -> Vec<Output> {
            audio::sinks().into_iter().map(|s| Output { id: s.name, name: s.description }).collect()
        }

        /// Show a notification; resolves to whether its action was clicked.
        pub async fn notify(summary: String, body: String, action: Option<&'static str>) -> bool {
            tokio::task::spawn_blocking(move || notify::show(&summary, &body, action)).await.unwrap_or(false)
        }

        pub async fn known_devices() -> Vec<KnownDevice> {
            let Ok(bt) = bluer::Session::new().await else { return Vec::new() };
            betterpods_linux::paired_airpods(&bt)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|p| KnownDevice { address: p.address.to_string(), name: p.name, connected: p.connected })
                .collect()
        }

        pub async fn connect(addr: &str) -> Result<(), String> {
            let addr: bluer::Address = addr.parse().map_err(|_| "invalid address".to_string())?;
            let bt = bluer::Session::new().await.map_err(|e| e.to_string())?;
            let adapter = bt.default_adapter().await.map_err(|e| e.to_string())?;
            adapter.device(addr).map_err(|e| e.to_string())?.connect().await.map_err(|e| e.to_string())
        }

        pub async fn disconnect(addr: &str) -> Result<(), String> {
            let addr: bluer::Address = addr.parse().map_err(|_| "invalid address".to_string())?;
            let bt = bluer::Session::new().await.map_err(|e| e.to_string())?;
            let adapter = bt.default_adapter().await.map_err(|e| e.to_string())?;
            adapter.device(addr).map_err(|e| e.to_string())?.disconnect().await.map_err(|e| e.to_string())
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod other {
    use betterpods_core::{Command, DeviceState, Settings};
    use tokio::sync::{mpsc, watch};

    use super::{KnownDevice, Output};

    /// Phase 2 brings the Windows implementation; until then only `--demo` works.
    pub struct Platform;

    impl Platform {
        pub async fn new() -> Self {
            Self
        }
        pub async fn run_connection(
            _cmd: mpsc::Receiver<Command>,
            _state: watch::Sender<DeviceState>,
            _settings: watch::Receiver<Settings>,
        ) {
            log::error!("this platform has no AirPods transport yet; try --demo");
        }
        pub async fn playing(&self) -> bool {
            false
        }
        pub async fn pause(&self) {}
        pub async fn resume(&self) {}
        pub async fn forget_paused(&self) {}
        pub async fn watch_playback(&self, _on_start: impl Fn()) {}
        pub async fn route_to_airpods(&self, _addr: &str, _reconnect: bool) {}
        pub async fn route_away(&self, _addr: &str, _fallback: Option<String>) {}
        pub async fn volume(&self) -> Option<u32> {
            None
        }
        pub async fn set_volume(&self, _percent: u32) {}
        pub fn outputs() -> Vec<Output> {
            Vec::new()
        }
        pub async fn notify(_summary: String, _body: String, _action: Option<&'static str>) -> bool {
            false
        }
        pub async fn known_devices() -> Vec<KnownDevice> {
            Vec::new()
        }
        pub async fn connect(_addr: &str) -> Result<(), String> {
            Err("not supported yet".into())
        }
        pub async fn disconnect(_addr: &str) -> Result<(), String> {
            Err("not supported yet".into())
        }
    }
}
