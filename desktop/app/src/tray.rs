//! Keeps the tray icon in sync with the AirPods and the tray setting.

use std::sync::Arc;

use crate::service::{battery_line, Service};

#[cfg(target_os = "linux")]
pub async fn run(svc: Arc<Service>) {
    use droidpods_linux::tray::{Tray, TrayView};

    let view = |svc: &Service| {
        let s = svc.state.borrow();
        TrayView {
            connected: s.ready,
            level: s.min_bud_level(),
            style: svc.settings.borrow().tray_battery,
            light: crate::ui::system_is_light(),
            tooltip: if s.ready {
                format!("{} · {}", s.metadata.name, battery_line(&s))
            } else {
                "Nicht verbunden".into()
            },
        }
    };
    let handle = match Tray::spawn(view(&svc), crate::ui::toggle, || {
        let _ = slint::quit_event_loop();
    })
    .await
    {
        Ok(h) => h,
        Err(e) => {
            log::warn!("no tray available: {e}");
            return;
        }
    };
    let mut state = svc.state.clone();
    let mut settings = svc.settings.subscribe();
    let mut last = view(&svc);
    loop {
        // The panel's theme can change without any other event; re-check now and then.
        tokio::select! {
            r = state.changed() => if r.is_err() { return },
            r = settings.changed() => if r.is_err() { return },
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
        }
        let v = view(&svc);
        if v != last {
            last = v.clone();
            handle.update(|t| t.set_view(v)).await;
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn run(_svc: Arc<Service>) {}
