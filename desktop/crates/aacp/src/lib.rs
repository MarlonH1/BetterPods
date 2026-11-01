//! Apple Accessory Communication Protocol (AACP) — the AirPods control channel
//! that runs over Bluetooth Classic L2CAP PSM 0x1001.
//!
//! This crate only builds and parses packets; it does no I/O. Byte layouts
//! follow `docs/` and the Android `AACPManager.kt`.

mod control;
mod packets;

pub use control::{ControlId, ListeningMode};
pub use packets::*;

/// L2CAP PSM of the AACP control channel.
pub const PSM: u16 = 0x1001;
/// Service UUID AirPods advertise over SDP for the control channel.
pub const SERVICE_UUID: &str = "74ec2172-0bad-4d01-8f77-997b2be0722a";
