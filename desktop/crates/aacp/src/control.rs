/// Identifiers for opcode 0x0009 control commands (see `docs/control_commands.md`
/// and `ControlCommandIdentifiers` in the Android `AACPManager.kt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ControlId {
    /// 0 automatic, 1 always right, 2 always left.
    MicMode = 0x01,
    OwnsConnection = 0x06,
    EarDetection = 0x0A,
    ListeningMode = 0x0D,
    /// Double/triple press speed: 0 default, 1 slower, 2 slowest.
    DoubleClickInterval = 0x17,
    /// Press-and-hold duration: 0 default, 1 longer, 2 longest.
    ClickHoldInterval = 0x18,
    /// Bitmask of modes press-and-hold cycles through, see [`ListeningMode::cycle_bit`].
    ListeningModeConfigs = 0x1A,
    OneBudAnc = 0x1B,
    /// `[volume 0..=100, 0x50]`.
    ChimeVolume = 0x1F,
    AutomaticConnection = 0x20,
    /// 1 default, 2 longer, 3 longest.
    VolumeSwipeInterval = 0x23,
    /// `[0x00, 0x03]` press once mutes / twice hangs up; `[0x00, 0x02]` the other way round.
    CallManagement = 0x24,
    VolumeSwipeMode = 0x25,
    AdaptiveVolume = 0x26,
    ConversationalAwareness = 0x28,
    /// Raw value 0..=100 where higher lets less noise through (the UI shows `100 - value`).
    AdaptiveStrength = 0x2E,
    AllowOffOption = 0x34,
    SleepDetection = 0x35,
    StemConfig = 0x39,
    DynamicEndOfCharge = 0x3B,
}

impl ControlId {
    const ALL: [ControlId; 20] = {
        use ControlId::*;
        [
            MicMode,
            OwnsConnection,
            EarDetection,
            ListeningMode,
            DoubleClickInterval,
            ClickHoldInterval,
            ListeningModeConfigs,
            OneBudAnc,
            ChimeVolume,
            AutomaticConnection,
            VolumeSwipeInterval,
            CallManagement,
            VolumeSwipeMode,
            AdaptiveVolume,
            ConversationalAwareness,
            AdaptiveStrength,
            AllowOffOption,
            SleepDetection,
            StemConfig,
            DynamicEndOfCharge,
        ]
    };

    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|id| *id as u8 == v)
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
    pub const ALL: [ListeningMode; 4] =
        [ListeningMode::Off, ListeningMode::Transparency, ListeningMode::Adaptive, ListeningMode::NoiseCancellation];

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

    /// This mode's bit in [`ControlId::ListeningModeConfigs`].
    pub fn cycle_bit(self) -> u8 {
        match self {
            Self::Off => 0x01,
            Self::NoiseCancellation => 0x02,
            Self::Transparency => 0x04,
            Self::Adaptive => 0x08,
        }
    }
}
