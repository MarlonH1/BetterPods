//! Output routing through the PulseAudio API (`pactl`, served by pipewire-pulse
//! on most systems): release the AirPods' audio profile when they come off, and
//! bring it back when they're worn again — the desktop version of an iPhone
//! switching to its speaker.

use std::process::Command;

use bluer::Address;
use serde::Deserialize;

/// A2DP profiles in order of preference; which exist depends on the codecs.
const A2DP_PROFILES: [&str; 3] = ["a2dp-sink-sbc_xq", "a2dp-sink-sbc", "a2dp-sink"];

#[derive(Debug, Clone)]
pub struct Sink {
    pub name: String,
    pub description: String,
}

fn card_name(addr: Address) -> String {
    format!("bluez_card.{}", addr.to_string().replace(':', "_"))
}

fn pactl(args: &[&str]) -> Option<String> {
    let out = Command::new("pactl").args(args).output().ok()?;
    if !out.status.success() {
        log::debug!("pactl {args:?}: {}", String::from_utf8_lossy(&out.stderr).trim());
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Every output except the AirPods' own, for the fallback picker.
pub fn sinks() -> Vec<Sink> {
    #[derive(Deserialize)]
    struct Raw {
        name: String,
        description: String,
    }
    let Some(json) = pactl(&["-f", "json", "list", "sinks"]) else { return Vec::new() };
    serde_json::from_str::<Vec<Raw>>(&json)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.name.starts_with("bluez_output."))
        .map(|s| Sink { name: s.name, description: s.description })
        .collect()
}

/// Make the AirPods the output again. `reconnect` rebuilds their sink: needed after
/// another device had them, since the AirPods drop our A2DP stream then and PipeWire
/// leaves the old sink in an error state.
pub fn route_to_airpods(addr: Address, reconnect: bool) {
    let card = card_name(addr);
    let prefix = format!("bluez_output.{}", addr.to_string().replace(':', "_"));
    let Some((active, best)) = a2dp_state(&card) else {
        log::warn!("{card}: not found");
        return;
    };
    // Keep the codec in use; switching codecs also rebuilds the sink.
    let Some(profile) = active.or(best) else {
        log::warn!("{card}: no A2DP profile available");
        return;
    };
    if reconnect && active.is_some() {
        pactl(&["set-card-profile", &card, "off"]);
        for _ in 0..20 {
            if !sink_names().iter().any(|n| n.starts_with(&prefix)) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    if reconnect || active.is_none() {
        pactl(&["set-card-profile", &card, profile]);
    }
    // The sink's suffix differs between PipeWire versions, and it shows up a moment
    // after the profile switch.
    for _ in 0..20 {
        if let Some(sink) = sink_names().into_iter().find(|n| n.starts_with(&prefix)) {
            pactl(&["set-default-sink", &sink]);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    log::warn!("{prefix}: sink did not appear");
}

fn sink_names() -> Vec<String> {
    pactl(&["list", "sinks", "short"])
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split('\t').nth(1).map(str::to_owned))
        .collect()
}

/// Move playback to `fallback` (or another output), like an iPhone switching to its
/// speaker. The AirPods stay a selectable output.
pub fn route_away(addr: Address, fallback: Option<&str>) {
    let prefix = format!("bluez_output.{}", addr.to_string().replace(':', "_"));
    let default = pactl(&["get-default-sink"]).unwrap_or_default();
    if !default.trim().starts_with(&prefix) {
        return;
    }
    let others = sinks();
    let target = fallback.filter(|f| others.iter().any(|s| s.name == *f)).or(others.first().map(|s| s.name.as_str()));
    if let Some(t) = target {
        pactl(&["set-default-sink", t]);
    }
}

/// The A2DP profile `card` is on, if any, and the best available one.
fn a2dp_state(card: &str) -> Option<(Option<&'static str>, Option<&'static str>)> {
    #[derive(Deserialize)]
    struct Card {
        name: String,
        active_profile: String,
        profiles: std::collections::HashMap<String, Profile>,
    }
    #[derive(Deserialize)]
    struct Profile {
        available: bool,
    }
    let json = pactl(&["-f", "json", "list", "cards"])?;
    let cards: Vec<Card> = serde_json::from_str(&json).ok()?;
    let card = cards.into_iter().find(|c| c.name == card)?;
    let best = A2DP_PROFILES.into_iter().find(|p| card.profiles.get(*p).is_some_and(|p| p.available));
    let active = A2DP_PROFILES.into_iter().find(|p| *p == card.active_profile);
    Some((active, best))
}

/// Volume of the default output in percent, for conversational awareness.
pub fn default_volume() -> Option<u32> {
    let out = pactl(&["get-sink-volume", "@DEFAULT_SINK@"])?;
    out.split('/').nth(1)?.trim().trim_end_matches('%').parse().ok()
}

pub fn set_default_volume(percent: u32) {
    pactl(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{percent}%")]);
}
