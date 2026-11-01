use std::time::Duration;

use aacp::routing::{self, Mac};
use aacp::{ControlId, Incoming, ListeningMode};
use tokio::sync::{mpsc, watch};

use crate::{DeviceState, Transport};

/// Something the UI or CLI asks the AirPods to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SetListeningMode(ListeningMode),
    SetFlag(ControlId, bool),
    /// Raw value for any control id.
    SetControl(ControlId, [u8; 4]),
    Rename(String),
    /// Take the AirPods from the device playing on them, like Apple devices do
    /// when you press play.
    TakeOver,
}

impl Command {
    fn packet(&self) -> Vec<u8> {
        match self {
            Command::SetListeningMode(m) => aacp::set_listening_mode(*m),
            Command::SetFlag(id, on) => aacp::set_flag(*id, *on),
            Command::SetControl(id, data) => aacp::control_command(*id, *data),
            Command::Rename(name) => aacp::rename(name),
            Command::TakeOver => unreachable!("expanded by the session"),
        }
    }

    /// The state change to show right away; the AirPods confirm control commands
    /// by echoing them, which then arrives through the normal path.
    fn optimistic(&self) -> Option<Incoming> {
        match self {
            Command::Rename(_) | Command::TakeOver => None,
            _ => Some(aacp::parse(&self.packet())),
        }
    }
}

#[derive(Debug)]
pub enum SessionEnd {
    /// The AirPods closed the channel.
    Closed,
    /// Every command sender was dropped.
    Stopped,
    Io(std::io::Error),
}

/// Who is talking to whom.
#[derive(Debug, Clone, Default)]
pub struct Peers {
    /// This device's name as other devices show it.
    pub name: String,
    /// "Linux", "Windows": tells our other apps what kind of device this is.
    pub platform: &'static str,
    /// The AirPods, as `AA:BB:CC:DD:EE:FF`.
    pub airpods: String,
    /// This computer's Bluetooth address; smart routing needs it.
    pub local: Option<Mac>,
}

/// Drive one connection: handshake, then fold incoming packets into `state` and
/// forward `commands` until the link closes.
pub async fn run_session<T: Transport>(
    link: T,
    peers: Peers,
    mut commands: mpsc::Receiver<Command>,
    state: watch::Sender<DeviceState>,
) -> SessionEnd {
    state.send_modify(|s| {
        *s = DeviceState { connected: true, address: peers.airpods.clone(), local: peers.local, ..Default::default() }
    });
    let end = drive(&link, &peers, &mut commands, &state).await;
    state.send_modify(|s| *s = DeviceState::default());
    end
}

/// AirPods stop answering when packets arrive back to back, so space them out.
async fn send_all<T: Transport>(link: &T, packets: &[Vec<u8>]) -> std::io::Result<()> {
    for (i, p) in packets.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        link.send(p).await?;
    }
    Ok(())
}

/// What to answer a packet with, beyond folding it into the state.
fn replies(packet: &Incoming, peers: &Peers, before: &DeviceState) -> Vec<Vec<u8>> {
    match packet {
        Incoming::HandshakeAck => vec![aacp::SET_SPECIFIC_FEATURES.to_vec()],
        Incoming::FeaturesAck => vec![aacp::REQUEST_NOTIFICATIONS.to_vec()],
        // Introduce ourselves to devices that just joined, so they include us in routing.
        Incoming::ConnectedDevices(devices) => {
            let Some(local) = &peers.local else { return Vec::new() };
            let mac = routing::format_mac(local);
            let me = routing::Me { mac: &mac, name: &peers.name, platform: peers.platform };
            devices
                .iter()
                .filter(|d| *d != local && !before.connected_devices.contains(d))
                .flat_map(|d| [routing::media_information(&me, d, false), routing::add_tipi_device(&me, d)])
                .collect()
        }
        // Another device took the AirPods: confirm we no longer own them.
        Incoming::SmartRouting { text, .. } if text.contains("SetOwnershipToFalse") => {
            vec![aacp::control_command(ControlId::OwnsConnection, [0x00, 0, 0, 0])]
        }
        _ => Vec::new(),
    }
}

/// The packets that take the AirPods from whoever plays on them, or nothing
/// if no other device does.
fn takeover(peers: &Peers, s: &DeviceState) -> Vec<Vec<u8>> {
    let Some(local) = &peers.local else { return Vec::new() };
    let Some((source, _)) = s.audio_source.filter(|_| s.other_device_playing(Some(local))) else { return Vec::new() };
    if !s.connected_devices.contains(&source) {
        return Vec::new();
    }
    let mac = routing::format_mac(local);
    let me = routing::Me { mac: &mac, name: &peers.name, platform: peers.platform };
    vec![
        aacp::control_command(ControlId::OwnsConnection, [0x01, 0, 0, 0]),
        routing::media_information(&me, &source, true),
        routing::show_ui(&source),
        routing::hijack_request(&source),
    ]
}

async fn drive<T: Transport>(
    link: &T,
    peers: &Peers,
    commands: &mut mpsc::Receiver<Command>,
    state: &watch::Sender<DeviceState>,
) -> SessionEnd {
    if let Err(e) = link.send(&aacp::HANDSHAKE).await {
        return SessionEnd::Io(e);
    }
    loop {
        tokio::select! {
            frame = link.recv() => {
                let frame = match frame {
                    Ok(Some(f)) => f,
                    Ok(None) => return SessionEnd::Closed,
                    Err(e) => return SessionEnd::Io(e),
                };
                let packet = aacp::parse(&frame);
                log::debug!("<- {packet:?}");
                if packet == Incoming::FeaturesAck {
                    state.send_modify(|s| s.ready = true);
                }
                let out = replies(&packet, peers, &state.borrow());
                state.send_if_modified(|s| s.apply(&packet));
                if let Err(e) = send_all(link, &out).await {
                    return SessionEnd::Io(e);
                }
            }
            cmd = commands.recv() => {
                let Some(cmd) = cmd else { return SessionEnd::Stopped };
                log::debug!("-> {cmd:?}");
                let out = match cmd {
                    Command::TakeOver => takeover(peers, &state.borrow()),
                    _ => vec![cmd.packet()],
                };
                if let Err(e) = send_all(link, &out).await {
                    return SessionEnd::Io(e);
                }
                if let Some(p) = cmd.optimistic() {
                    state.send_if_modified(|s| s.apply(&p));
                }
            }
        }
    }
}
