//! Smart routing: how Apple devices sharing one pair of AirPods agree on who
//! plays. Ported from the Android `AACPManager.kt` and the old Linux app.
//!
//! Addresses in [`Incoming::ConnectedDevices`] are in display order; the other
//! smart-routing packets carry them byte-reversed.

/// A Bluetooth address as six bytes in display order (`AA:BB:…`).
pub type Mac = [u8; 6];

pub fn format_mac(mac: &Mac) -> String {
    mac.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

pub fn parse_mac(s: &str) -> Option<Mac> {
    let mut out = [0u8; 6];
    let mut parts = s.split(':');
    for b in &mut out {
        *b = u8::from_str_radix(parts.next()?, 16).ok()?;
    }
    parts.next().is_none().then_some(out)
}

/// Minimal OPACK (Apple's compact binary plist) encoder for the values smart routing uses.
mod opack {
    pub enum Value<'a> {
        Str(&'a str),
        Int(i32),
        Bool(bool),
    }

    fn string(out: &mut Vec<u8>, s: &str) {
        let b = s.as_bytes();
        if b.len() <= 32 {
            out.push(0x40 + b.len() as u8);
        } else {
            out.extend_from_slice(&[0x61, b.len() as u8]);
        }
        out.extend_from_slice(b);
    }

    pub fn dict(entries: &[(&str, Value)]) -> Vec<u8> {
        let mut out = vec![0xE0 + entries.len() as u8];
        for (k, v) in entries {
            string(&mut out, k);
            match v {
                Value::Str(s) => string(&mut out, s),
                Value::Int(i @ 0..=39) => out.push(0x08 + *i as u8),
                Value::Int(i @ -128..=127) => out.extend_from_slice(&[0x30, *i as u8]),
                Value::Int(i) => out.extend_from_slice(&[0x31, (*i & 0xFF) as u8, ((*i >> 8) & 0xFF) as u8]),
                Value::Bool(b) => out.push(if *b { 0x01 } else { 0x02 }),
            }
        }
        out
    }
}

use opack::Value::{Bool, Int, Str};

/// `04 00 04 00 10 00 [target reversed] [len LE] 01 [opack]`
fn packet(target: &Mac, body: Vec<u8>) -> Vec<u8> {
    let mut p = vec![0x04, 0x00, 0x04, 0x00, 0x10, 0x00];
    p.extend(target.iter().rev());
    let len = body.len() as u16 + 1;
    p.extend_from_slice(&len.to_le_bytes());
    p.push(0x01);
    p.extend(body);
    p
}

/// How this device introduces itself to the others: `name` is what their
/// "moved to …" banner shows, `platform` lets our own apps tell the kind.
pub struct Me<'a> {
    pub mac: &'a str,
    pub name: &'a str,
    pub platform: &'a str,
}

/// Tell `target` what this device is playing.
pub fn media_information(me: &Me, target: &Mac, playing: bool) -> Vec<u8> {
    packet(
        target,
        opack::dict(&[
            ("playingApp", Str(if playing { "com.google.ios.youtube" } else { "Unknown" })),
            ("hostStreamingState", Str(if playing { "YES" } else { "NO" })),
            ("btAddress", Str(me.mac)),
            ("btName", Str(me.name)),
            ("btPlatform", Str(me.platform)),
            ("otherDeviceAudioCategory", Int(if playing { 301 } else { 100 })),
        ]),
    )
}

/// Ask `target` to show its "moved to …" banner.
pub fn show_ui(target: &Mac) -> Vec<u8> {
    packet(
        target,
        opack::dict(&[
            ("SmartRoutingKeyShowNearbyUI", Bool(true)),
            ("localscore", Int(301)),
            ("reason", Str("Hijackv2")),
            ("audioRoutingScore", Int(301)),
            ("audioRoutingSetOwnershipToFalse", Bool(true)),
            ("remotescore", Int(100)),
        ]),
    )
}

/// Ask `target` to give up the audio.
pub fn hijack_request(target: &Mac) -> Vec<u8> {
    packet(
        target,
        opack::dict(&[
            ("localscore", Int(301)),
            ("reason", Str("Hijackv2")),
            ("audioRoutingScore", Int(301)),
            ("audioRoutingSetOwnershipToFalse", Bool(true)),
            ("remotescore", Int(100)),
        ]),
    )
}

/// Introduce this device to `target` so it takes part in routing.
pub fn add_tipi_device(me: &Me, target: &Mac) -> Vec<u8> {
    packet(
        target,
        opack::dict(&[
            ("idleTime", Int(0)),
            ("newTipi", Bool(true)),
            ("btAddress", Str(me.mac)),
            ("btName", Str(me.name)),
            ("btPlatform", Str(me.platform)),
            ("nearbyAudioScore", Int(6)),
        ]),
    )
}

/// Which kind of device another participant is, guessed from its smart-routing text.
pub fn device_kind(text: &str) -> Option<&'static str> {
    ["iPad", "Mac", "iPhone", "Linux", "Windows", "Android"].into_iter().find(|k| text.contains(k))
}

/// The `btName` another device announced in a smart-routing message.
pub fn bt_name(body: &[u8]) -> Option<String> {
    const KEY: &[u8] = b"\x46btName";
    let at = body.windows(KEY.len()).position(|w| w == KEY)? + KEY.len();
    let (len, start) = match *body.get(at)? {
        t @ 0x40..=0x60 => ((t - 0x40) as usize, at + 1),
        0x61 => (*body.get(at + 1)? as usize, at + 2),
        _ => return None,
    };
    let name = std::str::from_utf8(body.get(start..start + len)?).ok()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}
