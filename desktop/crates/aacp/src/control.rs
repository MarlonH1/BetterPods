/// Identifiers for opcode 0x0009 control commands (see `docs/control_commands.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ControlId {
    MicMode = 0x01,
    OwnsConnection = 0x06,
    EarDetection = 0x0A,
    ListeningMode = 0x0D,
    ListeningModeConfigs = 0x1A,
    OneBudAnc = 0x1B,
    ChimeVolume = 0x1F,
    AutomaticConnection = 0x20,
    VolumeSwipeInterval = 0x23,
    VolumeSwipeMode = 0x25,
    AdaptiveVolume = 0x26,
    ConversationalAwareness = 0x28,
    AdaptiveStrength = 0x2E,
    AllowOffOption = 0x34,
    SleepDetection = 0x35,
    DynamicEndOfCharge = 0x3B,
}

impl ControlId {
    pub fn from_u8(v: u8) -> Option<Self> {
        use ControlId::*;
        Some(match v {
            0x01 => MicMode,
            0x06 => OwnsConnection,
            0x0A => EarDetection,
            0x0D => ListeningMode,
            0x1A => ListeningModeConfigs,
            0x1B => OneBudAnc,
            0x1F => ChimeVolume,
            0x20 => AutomaticConnection,
            0x23 => VolumeSwipeInterval,
            0x25 => VolumeSwipeMode,
            0x26 => AdaptiveVolume,
            0x28 => ConversationalAwareness,
            0x2E => AdaptiveStrength,
            0x34 => AllowOffOption,
            0x35 => SleepDetection,
            0x3B => DynamicEndOfCharge,
            _ => return None,
        })
    }
}

/// Value of [`ControlId::ListeningMode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ListeningMode {
    Off = 0x01,
    NoiseCancellation = 0x02,
    Transparency = 0x03,
    Adaptive = 0x04,
}

impl ListeningMode {
    pub const ALL: [ListeningMode; 4] = [
        ListeningMode::Off,
        ListeningMode::Transparency,
        ListeningMode::Adaptive,
        ListeningMode::NoiseCancellation,
    ];

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Off),
            0x02 => Some(Self::NoiseCancellation),
            0x03 => Some(Self::Transparency),
            0x04 => Some(Self::Adaptive),
            _ => None,
        }
    }

    /// Stable name used by the CLI and config files.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::NoiseCancellation => "anc",
            Self::Transparency => "transparency",
            Self::Adaptive => "adaptive",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.as_str() == s)
    }
}
