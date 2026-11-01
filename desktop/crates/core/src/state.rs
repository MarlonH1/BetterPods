use aacp::{BatteryStatus, Component, ControlId, EarState, Incoming, ListeningMode, Metadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudBattery {
    pub level: u8,
    pub charging: bool,
}

/// Everything the UI and CLI show about the connected AirPods.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceState {
    pub connected: bool,
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
    pub listening_mode: Option<ListeningMode>,
    pub conversational_awareness: Option<bool>,
    pub adaptive_volume: Option<bool>,
    pub adaptive_strength: Option<u8>,
    pub allow_off_mode: Option<bool>,
    pub owns_connection: Option<bool>,
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
            Incoming::Control { .. } => {
                if let Some(m) = packet.listening_mode() {
                    self.listening_mode = Some(m);
                }
                self.apply_flag(packet, ControlId::ConversationalAwareness, |s| &mut s.conversational_awareness);
                self.apply_flag(packet, ControlId::AdaptiveVolume, |s| &mut s.adaptive_volume);
                self.apply_flag(packet, ControlId::AllowOffOption, |s| &mut s.allow_off_mode);
                if let Incoming::Control { id, data } = packet {
                    match ControlId::from_u8(*id) {
                        Some(ControlId::AdaptiveStrength) => self.adaptive_strength = Some(data[0]),
                        Some(ControlId::OwnsConnection) => self.owns_connection = Some(data[0] == 0x01),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        *self != before
    }

    fn apply_flag(&mut self, p: &Incoming, id: ControlId, field: impl FnOnce(&mut Self) -> &mut Option<bool>) {
        if let Some(v) = p.flag(id) {
            *field(self) = Some(v);
        }
    }

    /// Lowest bud level, as shown on the tray icon and by `droidpods status --format {min}`.
    pub fn min_bud_level(&self) -> Option<u8> {
        [self.left, self.right, self.headset].into_iter().flatten().map(|b| b.level).min()
    }

    /// Whether any bud is currently in an ear.
    pub fn any_in_ear(&self) -> bool {
        self.ear_primary == Some(EarState::InEar) || self.ear_secondary == Some(EarState::InEar)
    }
}
