//! Named settings for `droidpods get` / `droidpods set`. Device keys map to AACP
//! controls and only appear for models that have them; app keys live in `Settings`.

use aacp::{Capability, ControlId, ListeningMode};
use droidpods_core::{Command, ConnectPolicy, DeviceState, Settings, TrayBattery};

pub enum Change {
    Device(Command),
    App(Settings),
}

#[derive(Debug)]
pub enum KeyError {
    Unknown(String),
    BadValue { key: &'static str, expected: String },
    NotConnected,
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::Unknown(k) => write!(f, "unknown setting '{k}' (see: droidpods get)"),
            KeyError::BadValue { key, expected } => write!(f, "{key} takes {expected}"),
            KeyError::NotConnected => write!(f, "no AirPods connected"),
        }
    }
}

enum Kind {
    Flag(ControlId),
    /// Value in `data[0]`, one name per raw value starting at `base`.
    Choice(ControlId, u8, &'static [&'static str]),
    Percent,
}

struct DeviceKey {
    key: &'static str,
    cap: Option<Capability>,
    kind: Kind,
}

const DEVICE_KEYS: &[DeviceKey] = &[
    DeviceKey {
        key: "conversational-awareness",
        cap: Some(Capability::ConversationAwareness),
        kind: Kind::Flag(ControlId::ConversationalAwareness),
    },
    DeviceKey {
        key: "personalized-volume",
        cap: Some(Capability::AdaptiveVolume),
        kind: Kind::Flag(ControlId::AdaptiveVolume),
    },
    DeviceKey { key: "adaptive-strength", cap: Some(Capability::AdaptiveAudio), kind: Kind::Percent },
    DeviceKey {
        key: "off-mode",
        cap: Some(Capability::LoudSoundReduction),
        kind: Kind::Flag(ControlId::AllowOffOption),
    },
    DeviceKey { key: "one-bud-anc", cap: Some(Capability::ListeningMode), kind: Kind::Flag(ControlId::OneBudAnc) },
    DeviceKey { key: "microphone", cap: None, kind: Kind::Choice(ControlId::MicMode, 0, &["auto", "right", "left"]) },
    DeviceKey {
        key: "press-speed",
        cap: None,
        kind: Kind::Choice(ControlId::DoubleClickInterval, 0, &["default", "slower", "slowest"]),
    },
    DeviceKey {
        key: "hold-duration",
        cap: None,
        kind: Kind::Choice(ControlId::ClickHoldInterval, 0, &["default", "longer", "longest"]),
    },
    DeviceKey {
        key: "swipe-volume",
        cap: Some(Capability::SwipeForVolume),
        kind: Kind::Flag(ControlId::VolumeSwipeMode),
    },
    DeviceKey {
        key: "swipe-interval",
        cap: Some(Capability::SwipeForVolume),
        kind: Kind::Choice(ControlId::VolumeSwipeInterval, 1, &["default", "longer", "longest"]),
    },
    DeviceKey {
        key: "sleep-detection",
        cap: Some(Capability::SleepDetection),
        kind: Kind::Flag(ControlId::SleepDetection),
    },
    DeviceKey { key: "optimized-charging", cap: None, kind: Kind::Flag(ControlId::DynamicEndOfCharge) },
];

fn on_off(v: bool) -> String {
    if v { "on" } else { "off" }.into()
}

fn parse_bool(key: &'static str, v: &str) -> Result<bool, KeyError> {
    match v {
        "on" | "true" | "1" | "yes" => Ok(true),
        "off" | "false" | "0" | "no" => Ok(false),
        _ => Err(KeyError::BadValue { key, expected: "on or off".into() }),
    }
}

fn parse_percent(key: &'static str, v: &str) -> Result<u8, KeyError> {
    v.trim_end_matches('%')
        .parse::<u8>()
        .ok()
        .filter(|p| *p <= 100)
        .ok_or(KeyError::BadValue { key, expected: "0–100".into() })
}

fn choice<'a>(key: &'static str, names: &[&'a str], v: &str) -> Result<usize, KeyError> {
    names.iter().position(|n| *n == v).ok_or(KeyError::BadValue { key, expected: names.join(" | ") })
}

/// Modes press-and-hold cycles through, as `transparency,adaptive,anc`.
fn hold_cycle(bits: u8) -> String {
    ListeningMode::ALL.iter().filter(|m| bits & m.cycle_bit() != 0).map(|m| m.as_str()).collect::<Vec<_>>().join(",")
}

/// Every key with its current value. Device keys only while connected.
pub fn get_all(state: &DeviceState, settings: &Settings) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if state.ready {
        out.push(("name".into(), format!("\"{}\"", state.metadata.name)));
        for k in DEVICE_KEYS.iter().filter(|k| k.cap.is_none_or(|c| state.has(c))) {
            let value = match k.kind {
                Kind::Flag(id) => state.flag(id).map(on_off),
                Kind::Choice(id, base, names) => {
                    state.control(id).and_then(|d| names.get(d[0].wrapping_sub(base) as usize)).map(|s| s.to_string())
                }
                Kind::Percent => state.control(ControlId::AdaptiveStrength).map(|d| (100 - d[0].min(100)).to_string()),
            };
            // Like an iPhone, settings the model doesn't report aren't listed.
            if let Some(v) = value {
                out.push((k.key.into(), v));
            }
        }
        if state.has(Capability::ListeningMode) {
            if let Some(d) = state.control(ControlId::ListeningModeConfigs) {
                out.push(("hold-cycle".into(), hold_cycle(d[0])));
            }
        }
        if let Some(d) = state.control(ControlId::ChimeVolume) {
            out.push(("tone-volume".into(), d[0].to_string()));
        }
        if !state.model().is_some_and(|m| m.headphones) {
            if let Some(d) = state.control(ControlId::CallManagement) {
                out.push(("call-press-once".into(), if d[1] == 0x02 { "hangup" } else { "mute" }.into()));
            }
        }
    }
    out.push(("ear-detection".into(), on_off(settings.ear_detection)));
    out.push(("fallback-output".into(), settings.fallback_output.clone().unwrap_or_else(|| "auto".into())));
    out.push((
        "connect".into(),
        match settings.connect {
            ConnectPolicy::Auto => "auto",
            ConnectPolicy::LastUsed => "last-used",
        }
        .into(),
    ));
    out.push((
        "tray-battery".into(),
        match settings.tray_battery {
            TrayBattery::Off => "off",
            TrayBattery::Percent => "percent",
            TrayBattery::Ring => "ring",
        }
        .into(),
    ));
    out.push(("notify-connected".into(), on_off(settings.notify_connected)));
    out.push(("notify-takeover".into(), on_off(settings.notify_takeover)));
    out.push(("conversation-volume".into(), settings.conversation_volume.to_string()));
    out.push(("device-name".into(), format!("\"{}\"", settings.device_name())));
    out
}

pub fn set(state: &DeviceState, settings: &Settings, key: &str, value: &str) -> Result<Change, KeyError> {
    let mut s = settings.clone();
    match key {
        "ear-detection" => s.ear_detection = parse_bool("ear-detection", value)?,
        "fallback-output" => s.fallback_output = (value != "auto").then(|| value.to_owned()),
        "connect" => {
            s.connect =
                [ConnectPolicy::Auto, ConnectPolicy::LastUsed][choice("connect", &["auto", "last-used"], value)?]
        }
        "tray-battery" => {
            s.tray_battery = [TrayBattery::Off, TrayBattery::Percent, TrayBattery::Ring]
                [choice("tray-battery", &["off", "percent", "ring"], value)?]
        }
        "notify-connected" => s.notify_connected = parse_bool("notify-connected", value)?,
        "notify-takeover" => s.notify_takeover = parse_bool("notify-takeover", value)?,
        "conversation-volume" => s.conversation_volume = parse_percent("conversation-volume", value)?,
        // Empty or the default name follows the hostname again.
        "device-name" => {
            let v = value.trim();
            s.device_name = (!v.is_empty() && v != droidpods_core::default_device_name()).then(|| v.to_owned());
        }
        _ => return set_device(state, key, value).map(Change::Device),
    }
    Ok(Change::App(s))
}

fn set_device(state: &DeviceState, key: &str, value: &str) -> Result<Command, KeyError> {
    let device_key = DEVICE_KEYS.iter().find(|k| k.key == key);
    let known = device_key.is_some()
        || matches!(key, "name" | "hold-cycle" | "hold-cycle-bits" | "tone-volume" | "call-press-once");
    if !known {
        return Err(KeyError::Unknown(key.into()));
    }
    if !state.ready {
        return Err(KeyError::NotConnected);
    }
    match key {
        "name" => return Ok(Command::Rename(value.into())),
        "tone-volume" => {
            return Ok(Command::SetControl(ControlId::ChimeVolume, [parse_percent("tone-volume", value)?, 0x50, 0, 0]))
        }
        "call-press-once" => {
            let once_hangs_up = choice("call-press-once", &["mute", "hangup"], value)? == 1;
            return Ok(Command::SetControl(
                ControlId::CallManagement,
                [0x00, if once_hangs_up { 0x02 } else { 0x03 }, 0, 0],
            ));
        }
        // The UI's checkboxes send the bitmask directly.
        "hold-cycle-bits" => {
            let bits =
                value.parse::<u8>().ok().filter(|b| *b <= 0x0F && b.count_ones() >= 2).ok_or(KeyError::BadValue {
                    key: "hold-cycle-bits",
                    expected: "a mask with at least two modes".into(),
                })?;
            return Ok(Command::SetControl(ControlId::ListeningModeConfigs, [bits, 0, 0, 0]));
        }
        "hold-cycle" => {
            let mut bits = 0;
            for name in value.split(',') {
                let m = ListeningMode::parse(name.trim()).ok_or(KeyError::BadValue {
                    key: "hold-cycle",
                    expected: "a comma list of off, transparency, adaptive, anc".into(),
                })?;
                bits |= m.cycle_bit();
            }
            // Like iOS: at least two modes to switch between.
            if bits.count_ones() < 2 {
                return Err(KeyError::BadValue { key: "hold-cycle", expected: "at least two modes".into() });
            }
            return Ok(Command::SetControl(ControlId::ListeningModeConfigs, [bits, 0, 0, 0]));
        }
        _ => {}
    }
    let k = device_key.expect("checked above");
    Ok(match k.kind {
        Kind::Flag(id) => Command::SetFlag(id, parse_bool(k.key, value)?),
        Kind::Choice(id, base, names) => Command::SetControl(id, [choice(k.key, names, value)? as u8 + base, 0, 0, 0]),
        Kind::Percent => {
            Command::SetControl(ControlId::AdaptiveStrength, [100 - parse_percent(k.key, value)?, 0, 0, 0])
        }
    })
}
