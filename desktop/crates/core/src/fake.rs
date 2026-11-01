//! Virtual AirPods that speak AACP over an in-memory link. Used by the
//! end-to-end tests and by `betterpods --demo`.

use std::io;

use aacp::{ControlId, ListeningMode};
use tokio::sync::{mpsc, Mutex};

use crate::Transport;

/// One end of an in-memory, message-preserving link.
pub struct MemLink {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: Mutex<mpsc::UnboundedReceiver<Vec<u8>>>,
}

/// Two connected ends.
pub fn mem_pair() -> (MemLink, MemLink) {
    let (a_tx, a_rx) = mpsc::unbounded_channel();
    let (b_tx, b_rx) = mpsc::unbounded_channel();
    (MemLink { tx: a_tx, rx: Mutex::new(b_rx) }, MemLink { tx: b_tx, rx: Mutex::new(a_rx) })
}

impl Transport for MemLink {
    async fn send(&self, frame: &[u8]) -> io::Result<()> {
        self.tx.send(frame.to_vec()).map_err(|_| io::ErrorKind::BrokenPipe.into())
    }

    async fn recv(&self) -> io::Result<Option<Vec<u8>>> {
        Ok(self.rx.lock().await.recv().await)
    }
}

/// What the virtual AirPods report.
#[derive(Debug, Clone)]
pub struct FakeConfig {
    pub name: String,
    pub model_number: String,
    pub left: u8,
    pub right: u8,
    pub case: u8,
    pub case_charging: bool,
    pub mode: ListeningMode,
    pub conversational_awareness: bool,
}

impl Default for FakeConfig {
    fn default() -> Self {
        Self {
            name: "AirPods Pro 2".into(),
            model_number: "A3047".into(),
            left: 82,
            right: 79,
            case: 31,
            case_charging: true,
            mode: ListeningMode::Adaptive,
            conversational_awareness: true,
        }
    }
}

/// Events the test (or demo) can trigger on the virtual AirPods.
#[derive(Debug, Clone)]
pub enum FakeEvent {
    /// Push a raw frame to the host.
    Raw(Vec<u8>),
    /// Close the link like AirPods going out of range.
    Disconnect,
}

/// Run virtual AirPods on `link` until the host disconnects or `Disconnect` is sent.
/// Every frame the host sends is also forwarded to `seen` for assertions.
pub async fn run_fake(
    link: MemLink,
    mut cfg: FakeConfig,
    mut events: mpsc::UnboundedReceiver<FakeEvent>,
    seen: Option<mpsc::UnboundedSender<Vec<u8>>>,
) {
    loop {
        tokio::select! {
            frame = link.recv() => {
                let Ok(Some(frame)) = frame else { return };
                if let Some(s) = &seen { let _ = s.send(frame.clone()); }
                for reply in respond(&mut cfg, &frame) {
                    if link.send(&reply).await.is_err() { return; }
                }
            }
            ev = events.recv() => match ev {
                Some(FakeEvent::Raw(f)) => { if link.send(&f).await.is_err() { return; } }
                Some(FakeEvent::Disconnect) | None => return,
            }
        }
    }
}

fn respond(cfg: &mut FakeConfig, frame: &[u8]) -> Vec<Vec<u8>> {
    if frame == aacp::HANDSHAKE {
        return vec![vec![0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00]];
    }
    if frame == aacp::SET_SPECIFIC_FEATURES {
        return vec![vec![0x04, 0x00, 0x04, 0x00, 0x2B, 0x00, 0x00, 0x00]];
    }
    if frame == aacp::REQUEST_NOTIFICATIONS {
        return vec![
            metadata(cfg),
            battery(cfg),
            vec![0x04, 0x00, 0x04, 0x00, 0x06, 0x00, 0x00, 0x00],
            aacp::set_listening_mode(cfg.mode),
            aacp::set_flag(ControlId::ConversationalAwareness, cfg.conversational_awareness),
            // The rest of a Pro 2's settings at their factory defaults.
            aacp::set_flag(ControlId::AdaptiveVolume, false),
            aacp::set_flag(ControlId::AllowOffOption, true),
            aacp::set_flag(ControlId::OneBudAnc, true),
            aacp::set_flag(ControlId::VolumeSwipeMode, true),
            aacp::set_flag(ControlId::SleepDetection, true),
            aacp::set_flag(ControlId::DynamicEndOfCharge, true),
            aacp::control_command(ControlId::AdaptiveStrength, [38, 0, 0, 0]),
            aacp::control_command(ControlId::MicMode, [0, 0, 0, 0]),
            aacp::control_command(ControlId::DoubleClickInterval, [0, 0, 0, 0]),
            aacp::control_command(ControlId::ClickHoldInterval, [0, 0, 0, 0]),
            aacp::control_command(ControlId::VolumeSwipeInterval, [1, 0, 0, 0]),
            aacp::control_command(ControlId::ChimeVolume, [70, 0x50, 0, 0]),
            aacp::control_command(ControlId::ListeningModeConfigs, [0x0E, 0, 0, 0]),
            aacp::control_command(ControlId::CallManagement, [0x00, 0x03, 0, 0]),
        ];
    }
    // Control commands: real AirPods apply and echo them back.
    if frame.len() == 11 && frame[..6] == [0x04, 0x00, 0x04, 0x00, 0x09, 0x00] {
        if let Some(m) = ListeningMode::from_u8(frame[7]).filter(|_| frame[6] == ControlId::ListeningMode as u8) {
            cfg.mode = m;
        }
        return vec![frame.to_vec()];
    }
    Vec::new()
}

fn metadata(cfg: &FakeConfig) -> Vec<u8> {
    let mut p = vec![0x04, 0x00, 0x04, 0x00, 0x1D, 0x00, 0, 0, 0, 0, 0];
    for s in [cfg.name.as_str(), &cfg.model_number, "Apple Inc.", "DEMO00000000", "7A305"] {
        p.extend_from_slice(s.as_bytes());
        p.push(0);
    }
    p
}

/// Battery packet with the given levels; the left bud is reported as primary.
pub fn battery_packet(left: u8, right: u8, case: u8, case_charging: bool) -> Vec<u8> {
    let case_status = if case_charging { 0x01 } else { 0x02 };
    vec![
        0x04,
        0x00,
        0x04,
        0x00,
        0x04,
        0x00,
        0x03,
        0x04,
        0x01,
        left,
        0x02,
        0x01,
        0x02,
        0x01,
        right,
        0x02,
        0x01,
        0x08,
        0x01,
        case,
        case_status,
        0x01,
    ]
}

fn battery(cfg: &FakeConfig) -> Vec<u8> {
    battery_packet(cfg.left, cfg.right, cfg.case, cfg.case_charging)
}

/// Ear detection packet (`0` in ear, `1` out of ear, `2` in case).
pub fn ear_packet(primary: u8, secondary: u8) -> Vec<u8> {
    vec![0x04, 0x00, 0x04, 0x00, 0x06, 0x00, primary, secondary]
}
