use aacp::{ControlId, Incoming, ListeningMode};
use tokio::sync::{mpsc, watch};

use crate::{DeviceState, Transport};

/// Something the UI or CLI asks the AirPods to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SetListeningMode(ListeningMode),
    SetFlag(ControlId, bool),
    SetAdaptiveStrength(u8),
    Rename(String),
}

impl Command {
    fn packet(&self) -> Vec<u8> {
        match self {
            Command::SetListeningMode(m) => aacp::set_listening_mode(*m),
            Command::SetFlag(id, on) => aacp::set_flag(*id, *on),
            Command::SetAdaptiveStrength(l) => aacp::set_adaptive_strength(*l),
            Command::Rename(name) => aacp::rename(name),
        }
    }

    /// The state change to show right away; the AirPods confirm control commands
    /// by echoing them, which then arrives through the normal path.
    fn optimistic(&self) -> Option<Incoming> {
        match self {
            Command::Rename(_) => None,
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

/// Drive one connection: handshake, then fold incoming packets into `state`
/// and forward `commands` until the link closes.
pub async fn run_session<T: Transport>(
    link: T,
    mut commands: mpsc::Receiver<Command>,
    state: watch::Sender<DeviceState>,
) -> SessionEnd {
    state.send_modify(|s| *s = DeviceState { connected: true, ..Default::default() });
    let end = drive(&link, &mut commands, &state).await;
    state.send_modify(|s| *s = DeviceState::default());
    end
}

async fn drive<T: Transport>(
    link: &T,
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
                let reply: Option<&[u8]> = match packet {
                    Incoming::HandshakeAck => Some(&aacp::SET_SPECIFIC_FEATURES),
                    Incoming::FeaturesAck => {
                        state.send_modify(|s| s.ready = true);
                        Some(&aacp::REQUEST_NOTIFICATIONS)
                    }
                    _ => None,
                };
                if let Some(r) = reply {
                    if let Err(e) = link.send(r).await {
                        return SessionEnd::Io(e);
                    }
                }
                state.send_if_modified(|s| s.apply(&packet));
            }
            cmd = commands.recv() => {
                let Some(cmd) = cmd else { return SessionEnd::Stopped };
                log::debug!("-> {cmd:?}");
                if let Err(e) = link.send(&cmd.packet()).await {
                    return SessionEnd::Io(e);
                }
                if let Some(p) = cmd.optimistic() {
                    state.send_if_modified(|s| s.apply(&p));
                }
            }
        }
    }
}
