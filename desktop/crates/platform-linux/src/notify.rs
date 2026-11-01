//! Desktop notifications (freedesktop), optionally with one action button.

use notify_rust::{Notification, Timeout};

/// Show a notification and wait until it closes. Returns `true` if the action
/// button was clicked. Blocks, so call it from a blocking task.
pub fn show(summary: &str, body: &str, action: Option<&str>) -> bool {
    let mut n = Notification::new();
    n.appname("BetterPods").summary(summary).body(body).icon("audio-headphones").timeout(Timeout::Default);
    if let Some(label) = action {
        n.action("act", label);
    }
    let handle = match n.show() {
        Ok(h) => h,
        Err(e) => {
            log::warn!("notification failed: {e}");
            return false;
        }
    };
    if action.is_none() {
        return false;
    }
    let mut clicked = false;
    handle.wait_for_action(|a| clicked = a == "act");
    clicked
}
