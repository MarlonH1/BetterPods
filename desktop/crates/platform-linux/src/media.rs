//! Media players over MPRIS. Ear detection pauses whatever plays and later
//! resumes exactly those players, never something the user paused themselves.

use std::collections::HashSet;

use tokio::sync::Mutex;
use zbus::Connection;

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";

pub struct Media {
    bus: Connection,
    paused_by_us: Mutex<HashSet<String>>,
}

impl Media {
    pub async fn new() -> zbus::Result<Self> {
        Ok(Self { bus: Connection::session().await?, paused_by_us: Mutex::new(HashSet::new()) })
    }

    async fn players(&self) -> Vec<String> {
        let Ok(dbus) = zbus::fdo::DBusProxy::new(&self.bus).await else { return Vec::new() };
        let Ok(names) = dbus.list_names().await else { return Vec::new() };
        names
            .into_iter()
            .map(|n| n.to_string())
            // playerctld only mirrors the other players.
            .filter(|n| n.starts_with(PREFIX) && n != "org.mpris.MediaPlayer2.playerctld")
            .collect()
    }

    async fn status(&self, player: &str) -> Option<String> {
        let proxy = zbus::Proxy::new(&self.bus, player, PATH, PLAYER).await.ok()?;
        proxy.get_property::<String>("PlaybackStatus").await.ok()
    }

    async fn call(&self, player: &str, method: &str) {
        let Ok(proxy) = zbus::Proxy::new(&self.bus, player, PATH, PLAYER).await else { return };
        if let Err(e) = proxy.call_method(method, &()).await {
            log::debug!("{player}.{method}: {e}");
        }
    }

    /// Whether anything is playing right now.
    pub async fn playing(&self) -> bool {
        for p in self.players().await {
            if self.status(&p).await.as_deref() == Some("Playing") {
                return true;
            }
        }
        false
    }

    /// Pause every playing player and remember which ones they were.
    pub async fn pause(&self) {
        let mut paused = self.paused_by_us.lock().await;
        for p in self.players().await {
            if self.status(&p).await.as_deref() == Some("Playing") {
                self.call(&p, "Pause").await;
                paused.insert(p);
            }
        }
    }

    /// Resume the players [`Media::pause`] paused, if they're still paused.
    pub async fn resume(&self) {
        let paused: Vec<String> = self.paused_by_us.lock().await.drain().collect();
        for p in paused {
            if self.status(&p).await.as_deref() == Some("Paused") {
                self.call(&p, "Play").await;
            }
        }
    }

    /// Calls `on_start` whenever a player switches from not playing to playing. Browsers
    /// repeat "Playing" on every metadata or output change, which isn't a start.
    /// Runs until the bus goes away.
    pub async fn watch_starts(&self, on_start: impl Fn()) -> zbus::Result<()> {
        use futures_util::StreamExt;
        use std::collections::HashMap;
        use zbus::zvariant::OwnedValue;

        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.DBus.Properties")?
            .member("PropertiesChanged")?
            .path(PATH)?
            .arg(0, PLAYER)?
            .build();
        let mut stream = zbus::MessageStream::for_match_rule(rule, &self.bus, None).await?;
        let mut playing_now: HashSet<String> = HashSet::new();
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { continue };
            let Some(sender) = msg.header().sender().map(|s| s.to_string()) else { continue };
            let Ok((_, changed, _)) = msg.body().deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
            else {
                continue;
            };
            let Some(status) = changed.get("PlaybackStatus").and_then(|v| <&str>::try_from(v).ok()) else { continue };
            if status != "Playing" {
                playing_now.remove(&sender);
            } else if playing_now.insert(sender) {
                on_start();
            }
        }
        Ok(())
    }

    /// Forget what we paused, e.g. after another device took the AirPods.
    pub async fn forget(&self) {
        self.paused_by_us.lock().await.clear();
    }
}
