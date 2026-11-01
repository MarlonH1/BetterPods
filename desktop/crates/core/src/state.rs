use std::collections::BTreeMap;

use aacp::routing::Mac;
use aacp::{BatteryStatus, Capability, Component, ControlId, EarState, Incoming, ListeningMode, Metadata, Model};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudBattery {
    pub level: u8,
    pub charging: bool,
}

/// `audio_source` kind of a device in a call (0 idle, 2 media).
pub const AUDIO_SOURCE_CALL: u8 = 1;

/// Everything the UI and CLI show about the connected AirPods.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceState {
    pub connected: bool,
    /// Bluetooth address of the connected AirPods, as `AA:BB:CC:DD:EE:FF`.
    pub address: String,
    /// This computer's Bluetooth address, if known.
    pub local: Option<Mac>,
    /// Handshake done and notifications requested; settings are live.
    pub ready: bool,
    pub metadata: Metadata,
    pub left: Option<BudBattery>,
    pub right: Option<BudBattery>,
    pub case: Option<BudBattery>,
    /// AirPods Max report one battery for the whole headset.
    pub headset: Option<BudBattery>,
    /// Which bud currently holds the microphone; ear packets are ordered primary first.
    pub primary: Option<Component>,
    pub ear_primary: Option<EarState>,
    pub ear_secondary: Option<EarState>,
    /// Last value the AirPods reported for each control id (opcode 0x09).
    pub controls: BTreeMap<u8, [u8; 4]>,
    /// Conversational awareness level while it's active: 1 speaking … 9 normal.
    pub conversation_level: Option<u8>,
    /// Every device the AirPods are connected to, this one included.
    pub connected_devices: Vec<Mac>,
    /// What other devices said they are ("iPhone", "Mac", …), learned from smart routing.
    pub device_kinds: BTreeMap<Mac, &'static str>,
    /// Names other devices announced, for "moved to …".
    pub device_names: BTreeMap<Mac, String>,
    /// Which device plays, and whether anything does (`kind` 0 = nothing).
    pub audio_source: Option<(Mac, u8)>,
    /// Counts `SetOwnershipToFalse` requests: another device took the AirPods.
    pub taken_count: u32,
    /// Who took them last.
    pub taken_by: Option<Mac>,
}

impl DeviceState {
    /// Fold one packet from the AirPods into the state. Returns whether anything changed.
    pub fn apply(&mut self, packet: &Incoming) -> bool {
        let before = self.clone();
        match packet {
            Incoming::Metadata(m) => self.metadata = m.clone(),
            Incoming::Battery(entries) => {
                // The first bud listed is the primary one.
                if let Some(first) = entries.iter().find(|b| b.component != Component::Case) {
                    self.primary = Some(first.component);
                }
                for b in entries {
                    // A disconnected component keeps its last known level.
                    if b.status == BatteryStatus::Disconnected {
                        continue;
                    }
                    let v = Some(BudBattery { level: b.level, charging: b.status == BatteryStatus::Charging });
                    match b.component {
                        Component::Left => self.left = v,
                        Component::Right => self.right = v,
                        Component::Case => self.case = v,
                        Component::Headset => self.headset = v,
                    }
                }
            }
            Incoming::EarDetection { primary, secondary } => {
                self.ear_primary = Some(*primary);
                self.ear_secondary = Some(*secondary);
            }
            Incoming::Control { id, data } => {
                self.controls.insert(*id, *data);
            }
            Incoming::ConversationLevel(l) => self.conversation_level = Some(*l),
            Incoming::ConnectedDevices(d) => self.connected_devices = d.clone(),
            Incoming::AudioSource { mac, kind } => self.audio_source = Some((*mac, *kind)),
            Incoming::SmartRouting { sender, text, name } => {
                if let Some(k) = aacp::routing::device_kind(text) {
                    self.device_kinds.insert(*sender, k);
                }
                if let Some(n) = name {
                    self.device_names.insert(*sender, n.clone());
                }
                if text.contains("SetOwnershipToFalse") {
                    self.taken_count += 1;
                    self.taken_by = Some(*sender);
                }
            }
            _ => {}
        }
        *self != before
    }

    pub fn control(&self, id: ControlId) -> Option<[u8; 4]> {
        self.controls.get(&(id as u8)).copied()
    }

    /// Boolean controls are 0x01 on, 0x02 off.
    pub fn flag(&self, id: ControlId) -> Option<bool> {
        match self.control(id)?[0] {
            0x01 => Some(true),
            0x02 => Some(false),
            _ => None,
        }
    }

    pub fn listening_mode(&self) -> Option<ListeningMode> {
        ListeningMode::from_u8(self.control(ControlId::ListeningMode)?[0])
    }

    pub fn model(&self) -> Option<&'static Model> {
        Model::by_number(&self.metadata.model_number)
    }

    /// Whether the connected model has `c`. Unknown models get the full set,
    /// so new hardware isn't locked out before the table learns about it.
    pub fn has(&self, c: Capability) -> bool {
        self.model().is_none_or(|m| m.has(c))
    }

    /// Whether another device is playing through the AirPods right now.
    pub fn other_device_playing(&self, local: Option<&Mac>) -> bool {
        self.audio_source.is_some_and(|(mac, kind)| kind != 0 && Some(&mac) != local)
    }

    /// Whether playback starting here should take the AirPods, like on a Mac: yes unless they
    /// already play from here or another device is in a call. An idle or music-playing other
    /// device gives them up.
    pub fn should_take_for_playback(&self, local: Option<&Mac>) -> bool {
        match self.audio_source {
            Some((mac, kind)) if Some(&mac) == local => kind == 0,
            Some((_, AUDIO_SOURCE_CALL)) => false,
            _ => true,
        }
    }

    /// "iPhone", "Mac", … for `mac`, or a generic name.
    /// What another device calls itself, else its kind.
    pub fn device_name(&self, mac: &Mac) -> Option<String> {
        self.device_names.get(mac).cloned().or_else(|| self.device_kinds.get(mac).map(|k| k.to_string()))
    }

    /// Lowest bud level, as shown on the tray icon and by `betterpods status --format {min}`.
    pub fn min_bud_level(&self) -> Option<u8> {
        [self.left, self.right, self.headset].into_iter().flatten().map(|b| b.level).min()
    }

    /// Whether any bud is currently in an ear.
    pub fn any_in_ear(&self) -> bool {
        self.ear_primary == Some(EarState::InEar) || self.ear_secondary == Some(EarState::InEar)
    }
}
