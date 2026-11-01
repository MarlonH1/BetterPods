//! Device session and state for DroidPods, shared by every platform and the UI.

pub mod fake;
mod session;
mod state;

pub use session::{run_session, Command, SessionEnd};
pub use state::{BudBattery, DeviceState};

use std::future::Future;
use std::io;

/// A message-oriented link to the AirPods (an L2CAP SEQPACKET socket on Linux,
/// the driver handle on Windows, or an in-memory pair in tests).
pub trait Transport: Send + Sync + 'static {
    /// Send one AACP frame.
    fn send(&self, frame: &[u8]) -> impl Future<Output = io::Result<()>> + Send;
    /// Receive one AACP frame. `Ok(None)` means the link closed cleanly.
    fn recv(&self) -> impl Future<Output = io::Result<Option<Vec<u8>>>> + Send;
}
