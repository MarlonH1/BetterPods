use crate::control::{ControlId, ListeningMode};
use crate::routing::Mac;

/// Every AACP data packet starts with this, followed by a little-endian u16 opcode.
const PREFIX: [u8; 4] = [0x04, 0x00, 0x04, 0x00];

mod opcode {
    pub const BATTERY: u16 = 0x0004;
    pub const EAR_DETECTION: u16 = 0x0006;
    pub const CONTROL: u16 = 0x0009;
    pub const FEATURES_ACK: u16 = 0x002B;
    pub const METADATA: u16 = 0x001D;
    pub const RENAME: u16 = 0x001A;
    pub const MAGIC_KEYS: u16 = 0x0031;
    pub const CONVERSATION_LEVEL: u16 = 0x004B;
    pub const AUDIO_SOURCE: u16 = 0x000E;
    pub const CONNECTED_DEVICES: u16 = 0x002E;
    pub const SMART_ROUTING_RESP: u16 = 0x0011;
}

// ---------------------------------------------------------------- outgoing

/// First packet after the L2CAP channel opens.
pub const HANDSHAKE: [u8; 16] =
    [0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
/// Sent after [`Incoming::HandshakeAck`]; unlocks conversational awareness and adaptive features.
pub const SET_SPECIFIC_FEATURES: [u8; 14] =
    [0x04, 0x00, 0x04, 0x00, 0x4D, 0x00, 0xD7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
/// Sent after [`Incoming::FeaturesAck`]; the AirPods then push battery, ear and control state.
pub const REQUEST_NOTIFICATIONS: [u8; 11] = [0x04, 0x00, 0x04, 0x00, 0x0F, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
/// Asks for the IRK and encryption key used to decode BLE proximity adverts.
pub const REQUEST_MAGIC_CLOUD_KEYS: [u8; 8] = [0x04, 0x00, 0x04, 0x00, 0x30, 0x00, 0x05, 0x00];

/// `04 00 04 00 09 00 [id] [d1] [d2] [d3] [d4]`
pub fn control_command(id: ControlId, data: [u8; 4]) -> Vec<u8> {
    let mut p = header(opcode::CONTROL);
    p.push(id as u8);
    p.extend_from_slice(&data);
    p
}

pub fn set_listening_mode(mode: ListeningMode) -> Vec<u8> {
    control_command(ControlId::ListeningMode, [mode as u8, 0, 0, 0])
}

/// Boolean control commands use 0x01 for on and 0x02 for off.
pub fn set_flag(id: ControlId, on: bool) -> Vec<u8> {
    control_command(id, [if on { 0x01 } else { 0x02 }, 0, 0, 0])
}

/// `04 00 04 00 1A 00 01 [len] 00 [utf-8 name]`
pub fn rename(name: &str) -> Vec<u8> {
    let bytes = name.as_bytes();
    let len = bytes.len().min(u8::MAX as usize);
    let mut p = header(opcode::RENAME);
    p.extend_from_slice(&[0x01, len as u8, 0x00]);
    p.extend_from_slice(&bytes[..len]);
    p
}

fn header(op: u16) -> Vec<u8> {
    let mut p = PREFIX.to_vec();
    p.extend_from_slice(&op.to_le_bytes());
    p
}

// ---------------------------------------------------------------- incoming

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    /// AirPods Max report a single battery.
    Headset,
    Right,
    Left,
    Case,
}

impl Component {
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Headset),
            0x02 => Some(Self::Right),
            0x04 => Some(Self::Left),
            0x08 => Some(Self::Case),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Unknown,
    Charging,
    Discharging,
    /// The component is not connected (e.g. a bud that is in the case while the lid is closed).
    Disconnected,
}

impl BatteryStatus {
    fn from_u8(v: u8) -> Self {
        match v {
            0x01 => Self::Charging,
            0x02 => Self::Discharging,
            0x04 => Self::Disconnected,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub component: Component,
    pub level: u8,
    pub status: BatteryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarState {
    InEar,
    OutOfEar,
    InCase,
    Unknown,
}

impl EarState {
    fn from_u8(v: u8) -> Self {
        match v {
            0x00 => Self::InEar,
            0x01 => Self::OutOfEar,
            0x02 => Self::InCase,
            _ => Self::Unknown,
        }
    }
}

/// Device information sent once after connecting (opcode 0x001D).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub name: String,
    pub model_number: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub firmware: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    HandshakeAck,
    FeaturesAck,
    Battery(Vec<Battery>),
    /// Primary and secondary bud (the primary is the one holding the microphone).
    EarDetection {
        primary: EarState,
        secondary: EarState,
    },
    Control {
        id: u8,
        data: [u8; 4],
    },
    /// Conversational awareness volume level, 1 (speaking) … 9 (normal).
    ConversationLevel(u8),
    Metadata(Metadata),
    MagicCloudKeys {
        irk: [u8; 16],
        enc_key: [u8; 16],
    },
    /// Every device the AirPods are connected to right now, this one included.
    ConnectedDevices(Vec<Mac>),
    /// Which device plays; `kind` 0 means nothing does.
    AudioSource {
        mac: Mac,
        kind: u8,
    },
    /// A smart-routing message from another device, e.g. `SetOwnershipToFalse`.
    SmartRouting {
        sender: Mac,
        text: String,
        /// The sender's device name, if it announced one.
        name: Option<String>,
    },
    Unknown(Vec<u8>),
}

impl Incoming {
    /// Listening mode carried by a control packet, if this is one.
    pub fn listening_mode(&self) -> Option<ListeningMode> {
        match self {
            Incoming::Control { id, data } if *id == ControlId::ListeningMode as u8 => ListeningMode::from_u8(data[0]),
            _ => None,
        }
    }

    /// Boolean state carried by a control packet for `want`, if this is one.
    pub fn flag(&self, want: ControlId) -> Option<bool> {
        match self {
            Incoming::Control { id, data } if *id == want as u8 => match data[0] {
                0x01 => Some(true),
                0x02 => Some(false),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Decode one L2CAP frame from the AirPods.
pub fn parse(data: &[u8]) -> Incoming {
    if data.starts_with(&[0x01, 0x00, 0x04, 0x00]) {
        return Incoming::HandshakeAck;
    }
    if data.len() < 6 || data[..4] != PREFIX {
        return Incoming::Unknown(data.to_vec());
    }
    let op = u16::from_le_bytes([data[4], data[5]]);
    let body = &data[6..];
    let parsed = match op {
        opcode::FEATURES_ACK => Some(Incoming::FeaturesAck),
        opcode::BATTERY => parse_battery(body),
        opcode::EAR_DETECTION if body.len() >= 2 => {
            Some(Incoming::EarDetection { primary: EarState::from_u8(body[0]), secondary: EarState::from_u8(body[1]) })
        }
        opcode::CONTROL if body.len() >= 5 => {
            Some(Incoming::Control { id: body[0], data: [body[1], body[2], body[3], body[4]] })
        }
        opcode::CONVERSATION_LEVEL if body.len() == 4 && body[..3] == [0x02, 0x00, 0x01] => {
            Some(Incoming::ConversationLevel(body[3]))
        }
        opcode::METADATA => Some(Incoming::Metadata(parse_metadata(body))),
        opcode::MAGIC_KEYS => parse_magic_keys(body),
        opcode::CONNECTED_DEVICES if data.len() >= 9 => Some(Incoming::ConnectedDevices(
            data[9..].chunks_exact(8).take(data[8] as usize).map(|c| mac(&c[..6])).collect(),
        )),
        opcode::AUDIO_SOURCE if data.len() >= 13 => {
            Some(Incoming::AudioSource { mac: reversed_mac(&data[6..12]), kind: data[12] })
        }
        opcode::SMART_ROUTING_RESP if data.len() >= 12 => Some(Incoming::SmartRouting {
            sender: reversed_mac(&data[6..12]),
            text: String::from_utf8_lossy(&data[12..]).into_owned(),
            name: crate::routing::bt_name(&data[12..]),
        }),
        _ => None,
    };
    parsed.unwrap_or_else(|| Incoming::Unknown(data.to_vec()))
}

fn mac(b: &[u8]) -> Mac {
    [b[0], b[1], b[2], b[3], b[4], b[5]]
}

fn reversed_mac(b: &[u8]) -> Mac {
    [b[5], b[4], b[3], b[2], b[1], b[0]]
}

/// `[count] ([component] 01 [level] [status] 01) × count`
fn parse_battery(body: &[u8]) -> Option<Incoming> {
    let count = *body.first()? as usize;
    let entries = &body[1..];
    if count == 0 || count > 3 || entries.len() != count * 5 {
        return None;
    }
    entries
        .chunks_exact(5)
        .map(|e| {
            if e[1] != 0x01 || e[4] != 0x01 {
                return None;
            }
            Some(Battery { component: Component::from_u8(e[0])?, level: e[2], status: BatteryStatus::from_u8(e[3]) })
        })
        .collect::<Option<Vec<_>>>()
        .map(Incoming::Battery)
}

/// Five unknown bytes (as seen on AirPods Max), then null-terminated strings: name, model number,
/// manufacturer, serial number, firmware version, …
fn parse_metadata(body: &[u8]) -> Metadata {
    let mut strings = body
        .get(5..)
        .unwrap_or_default()
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned());
    let mut next = || strings.next().unwrap_or_default();
    Metadata { name: next(), model_number: next(), manufacturer: next(), serial_number: next(), firmware: next() }
}

/// `02 01 [len u16 BE] 00 [16 bytes irk] 04 [len u16 BE] 00 [16 bytes enc key]`
fn parse_magic_keys(body: &[u8]) -> Option<Incoming> {
    let b = body.strip_prefix(&[0x02])?;
    if b.len() < 40 || b[0] != 0x01 || b[1..3] != [0x00, 0x10] || b[20] != 0x04 || b[21..23] != [0x00, 0x10] {
        return None;
    }
    Some(Incoming::MagicCloudKeys { irk: b[4..20].try_into().ok()?, enc_key: b[24..40].try_into().ok()? })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn builds_known_packets() {
        assert_eq!(set_listening_mode(ListeningMode::Adaptive), hex("04 00 04 00 09 00 0D 04 00 00 00"));
        assert_eq!(set_listening_mode(ListeningMode::Off), hex("04 00 04 00 09 00 0D 01 00 00 00"));
        assert_eq!(set_flag(ControlId::ConversationalAwareness, true), hex("04 00 04 00 09 00 28 01 00 00 00"));
        assert_eq!(set_flag(ControlId::ConversationalAwareness, false), hex("04 00 04 00 09 00 28 02 00 00 00"));
        assert_eq!(rename("Pods"), hex("04 00 04 00 1A 00 01 04 00 50 6F 64 73"));
    }

    #[test]
    fn parses_handshake_and_features_ack() {
        assert_eq!(parse(&hex("01 00 04 00 00 00 01 00")), Incoming::HandshakeAck);
        assert_eq!(parse(&hex("04 00 04 00 2B 00 00 00")), Incoming::FeaturesAck);
    }

    /// Example packet from AirPods Pro 2 in `docs/AAP Definitions.md`.
    #[test]
    fn parses_battery_pro2() {
        let p = parse(&hex("04 00 04 00 04 00 03 02 01 64 02 01 04 01 63 01 01 08 01 11 02 01"));
        assert_eq!(
            p,
            Incoming::Battery(vec![
                Battery { component: Component::Right, level: 100, status: BatteryStatus::Discharging },
                Battery { component: Component::Left, level: 99, status: BatteryStatus::Charging },
                Battery { component: Component::Case, level: 17, status: BatteryStatus::Discharging },
            ])
        );
    }

    #[test]
    fn parses_battery_max() {
        let p = parse(&hex("04 00 04 00 04 00 01 01 01 4F 02 01"));
        assert_eq!(
            p,
            Incoming::Battery(vec![Battery {
                component: Component::Headset,
                level: 79,
                status: BatteryStatus::Discharging
            }])
        );
    }

    #[test]
    fn rejects_malformed_battery() {
        assert!(matches!(parse(&hex("04 00 04 00 04 00 02 02 01 64 02 01")), Incoming::Unknown(_)));
        assert!(matches!(parse(&hex("04 00 04 00 04 00 01 02 00 64 02 01")), Incoming::Unknown(_)));
    }

    #[test]
    fn parses_ear_detection() {
        assert_eq!(
            parse(&hex("04 00 04 00 06 00 00 02")),
            Incoming::EarDetection { primary: EarState::InEar, secondary: EarState::InCase }
        );
    }

    #[test]
    fn parses_control_values() {
        let p = parse(&hex("04 00 04 00 09 00 0D 02 00 00 00"));
        assert_eq!(p.listening_mode(), Some(ListeningMode::NoiseCancellation));
        let p = parse(&hex("04 00 04 00 09 00 28 02 00 00 00"));
        assert_eq!(p.flag(ControlId::ConversationalAwareness), Some(false));
        assert_eq!(p.listening_mode(), None);
    }

    #[test]
    fn parses_conversation_level() {
        assert_eq!(parse(&hex("04 00 04 00 4B 00 02 00 01 03")), Incoming::ConversationLevel(3));
    }

    #[test]
    fn parses_metadata() {
        let mut p = hex("04 00 04 00 1D 00 01 02 03 04 05");
        for s in ["AirPods Pro", "A3047", "Apple Inc.", "H7XQ4F2K", "7A305"] {
            p.extend_from_slice(s.as_bytes());
            p.push(0);
        }
        let Incoming::Metadata(m) = parse(&p) else { panic!("not metadata") };
        assert_eq!(m.name, "AirPods Pro");
        assert_eq!(m.model_number, "A3047");
        assert_eq!(m.manufacturer, "Apple Inc.");
        assert_eq!(m.serial_number, "H7XQ4F2K");
        assert_eq!(m.firmware, "7A305");
    }

    #[test]
    fn parses_magic_keys() {
        let mut p = hex("04 00 04 00 31 00 02 01 00 10 00");
        p.extend(0u8..16);
        p.extend(hex("04 00 10 00"));
        p.extend(100u8..116);
        let Incoming::MagicCloudKeys { irk, enc_key } = parse(&p) else { panic!("not keys") };
        assert_eq!(irk[0], 0);
        assert_eq!(irk[15], 15);
        assert_eq!(enc_key[0], 100);
    }

    #[test]
    fn unknown_is_preserved() {
        assert_eq!(parse(&[0x00, 0x01, 0x00, 0x00]), Incoming::Unknown(vec![0x00, 0x01, 0x00, 0x00]));
    }
}
