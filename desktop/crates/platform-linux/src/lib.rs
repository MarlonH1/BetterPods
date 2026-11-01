//! Linux platform layer: finds connected AirPods through BlueZ and opens the
//! AACP control channel (L2CAP PSM 0x1001) to them, plus the desktop services
//! the app needs: audio routing, media control, notifications and the tray.

pub mod audio;
pub mod media;
pub mod notify;
pub mod tray;

use std::io;
use std::time::Duration;

use bluer::l2cap::{SeqPacket, SocketAddr};
use bluer::{Address, AddressType, Uuid};
use droidpods_core::{run_session, Command, DeviceState, Peers, SessionEnd, Transport};
use tokio::sync::{mpsc, watch};

/// The AACP channel over an L2CAP SEQPACKET socket (message boundaries are kept).
pub struct L2capLink {
    socket: SeqPacket,
    mtu: usize,
}

impl L2capLink {
    pub async fn connect(addr: Address) -> io::Result<Self> {
        let socket = SeqPacket::connect(SocketAddr::new(addr, AddressType::BrEdr, aacp::PSM)).await?;
        let mtu = socket.recv_mtu().unwrap_or(1024) as usize;
        Ok(Self { socket, mtu })
    }
}

impl Transport for L2capLink {
    async fn send(&self, frame: &[u8]) -> io::Result<()> {
        self.socket.send(frame).await.map(|_| ())
    }

    async fn recv(&self) -> io::Result<Option<Vec<u8>>> {
        let mut buf = vec![0u8; self.mtu.max(64)];
        let n = self.socket.recv(&mut buf).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.truncate(n);
        Ok(Some(buf))
    }
}

/// A paired device that offers the AACP service.
#[derive(Debug, Clone)]
pub struct PairedAirPods {
    pub address: Address,
    pub name: String,
    pub connected: bool,
}

/// Paired devices that expose the AirPods control service.
pub async fn paired_airpods(session: &bluer::Session) -> bluer::Result<Vec<PairedAirPods>> {
    let service: Uuid = aacp::SERVICE_UUID.parse().expect("valid uuid");
    let adapter = session.default_adapter().await?;
    let mut out = Vec::new();
    for address in adapter.device_addresses().await? {
        let dev = adapter.device(address)?;
        let uuids = dev.uuids().await?.unwrap_or_default();
        if !uuids.contains(&service) || !dev.is_paired().await.unwrap_or(false) {
            continue;
        }
        out.push(PairedAirPods {
            address,
            name: dev.alias().await.unwrap_or_default(),
            connected: dev.is_connected().await.unwrap_or(false),
        });
    }
    Ok(out)
}

/// Keep a session open to whichever paired AirPods are connected to this PC.
/// Polls BlueZ every `interval`; returns when `commands` is closed.
pub async fn supervise(
    mut commands: mpsc::Receiver<Command>,
    state: watch::Sender<DeviceState>,
    interval: Duration,
    name: impl Fn() -> String,
) -> bluer::Result<()> {
    let bt = bluer::Session::new().await?;
    let local = bt.default_adapter().await?.address().await.ok().map(|a| a.0);
    loop {
        let target = paired_airpods(&bt).await?.into_iter().find(|p| p.connected);
        if let Some(p) = target {
            log::info!("opening AACP channel to {} ({})", p.name, p.address);
            match L2capLink::connect(p.address).await {
                Ok(link) => {
                    let (fwd_tx, fwd_rx) = mpsc::channel(16);
                    // Forward UI commands into this session; a fresh channel per session
                    // lets the supervisor keep `commands` across reconnects.
                    let fwd = async {
                        while let Some(c) = commands.recv().await {
                            if fwd_tx.send(c).await.is_err() {
                                return true;
                            }
                        }
                        false
                    };
                    let end = tokio::select! {
                        end = run_session(
                            link,
                            Peers { airpods: p.address.to_string(), local, name: name(), platform: "Linux" },
                            fwd_rx,
                            state.clone(),
                        ) => end,
                        session_alive = fwd => if session_alive { SessionEnd::Closed } else { return Ok(()) },
                    };
                    log::info!("session ended: {end:?}");
                }
                Err(e) => log::warn!("L2CAP connect to {} failed: {e}", p.address),
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            closed = commands.recv() => if closed.is_none() { return Ok(()) },
        }
    }
}
