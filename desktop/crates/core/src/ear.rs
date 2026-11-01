//! What to do when AirPods go in or out of the ears, like an iPhone does.
//! A port of `processEarDetectionChange` in the Android `AirPodsService.kt`;
//! the platform carries out the returned actions.

use aacp::EarState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarAction {
    /// Pause whatever is playing.
    Pause,
    /// Resume media that [`EarAction::Pause`] paused, never anything else.
    Resume,
    /// Make the AirPods the output again.
    RouteToAirPods,
    /// Move the output to the fallback device. Sent a moment after the pause so
    /// players don't stall rerouting mid-write.
    RouteToFallback,
}

/// Tracks ear state across ear-detection packets.
#[derive(Debug, Default)]
pub struct EarPolicy {
    /// In-ear flags from the last packet; `None` until the first one after a connect.
    last: Option<[bool; 2]>,
}

impl EarPolicy {
    /// Forget the previous state, e.g. after the AirPods disconnect.
    pub fn reset(&mut self) {
        self.last = None;
    }

    pub fn update(&mut self, primary: EarState, secondary: EarState) -> Vec<EarAction> {
        let now = [primary == EarState::InEar, secondary == EarState::InEar];
        let Some(before) = self.last.replace(now) else {
            // First packet after a (re)connect. AirPods Max reconnect on their own a few
            // seconds after being taken off, so there's no in→out transition: if they
            // aren't worn, keep the audio where it is.
            return if now.contains(&true) { Vec::new() } else { vec![EarAction::RouteToFallback] };
        };
        let (was, is) = (count(before), count(now));
        if was == is {
            return Vec::new();
        }
        match (was, is) {
            // Worn again: take the output back, then play if we paused.
            (0, _) => vec![EarAction::RouteToAirPods, EarAction::Resume],
            // Taken off entirely: pause, then fall back to the other output.
            (_, 0) => vec![EarAction::Pause, EarAction::RouteToFallback],
            // One of two taken out.
            (2, 1) => vec![EarAction::Pause],
            // The second one put back in.
            (1, 2) => vec![EarAction::Resume],
            _ => Vec::new(),
        }
    }
}

fn count(flags: [bool; 2]) -> u8 {
    flags.iter().filter(|f| **f).count() as u8
}
