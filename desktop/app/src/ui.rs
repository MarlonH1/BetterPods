//! The companion window. It only exists while it's open: closing destroys it so
//! the background process stays small, and the tray brings it back.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use aacp::{Capability, ControlId, ListeningMode};
use betterpods_core::{Command, ConnectPolicy, DeviceState, Settings, TrayBattery};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::keys::{self, Change};
use crate::platform::{Output, Platform};
use crate::service::Service;

slint::include_modules!();

const REPO: &str = "https://github.com/kavishdevar/librepods";

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

struct Host {
    svc: Arc<Service>,
    rt: tokio::runtime::Handle,
    window: Option<AppWindow>,
    /// Outputs in the order the fallback picker shows them (after "Automatisch").
    outputs: Rc<RefCell<Vec<Output>>>,
    theme_timer: slint::Timer,
}

/// Set up the UI side on the main thread. Call before running the event loop.
pub fn init(svc: Arc<Service>, rt: tokio::runtime::Handle) {
    HOST.with(|h| {
        *h.borrow_mut() =
            Some(Host { svc, rt, window: None, outputs: Rc::default(), theme_timer: slint::Timer::default() })
    });
}

/// Show the window, creating it if needed. Callable from any thread.
pub fn show() {
    let _ = slint::invoke_from_event_loop(|| HOST.with(|h| h.borrow_mut().as_mut().map(Host::show)).unwrap_or(()));
}

/// Show the window now. Main thread only; used at startup, before the event
/// loop runs and `invoke_from_event_loop` would work.
pub fn show_now() {
    HOST.with(|h| h.borrow_mut().as_mut().map(Host::show));
}

/// Show the window, or close it if it's open. Callable from any thread.
pub fn toggle() {
    let _ = slint::invoke_from_event_loop(|| {
        HOST.with(|h| {
            if let Some(host) = h.borrow_mut().as_mut() {
                if host.window.is_some() {
                    host.close();
                } else {
                    host.show();
                }
            }
        })
    });
}

fn close() {
    // Deferred: the window must not be dropped from inside its own callback.
    slint::Timer::single_shot(Duration::ZERO, || HOST.with(|h| h.borrow_mut().as_mut().map(Host::close).unwrap_or(())));
}

impl Host {
    fn show(&mut self) {
        if let Some(w) = &self.window {
            let _ = w.show();
            return;
        }
        let Ok(ui) = AppWindow::new() else {
            log::error!("could not create the window");
            return;
        };
        apply_theme(&ui);
        let weak = ui.as_weak();
        self.theme_timer.start(slint::TimerMode::Repeated, Duration::from_secs(2), move || {
            if let Some(ui) = weak.upgrade() {
                apply_theme(&ui);
            }
        });
        self.wire(&ui);
        self.refresh_outputs(&ui);
        apply_state(&ui, &self.svc.state.borrow());
        apply_settings(&ui, &self.svc.settings.borrow(), &self.outputs.borrow());
        self.rt.spawn(follow(ui.as_weak(), self.svc.clone()));
        self.rt.spawn(load_known(ui.as_weak()));
        if let Some(page) = std::env::var("BETTERPODS_PAGE").ok().and_then(|p| p.parse().ok()) {
            ui.set_page(page);
        }
        let _ = ui.show();
        self.window = Some(ui);
    }

    fn close(&mut self) {
        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        self.theme_timer.stop();
    }

    fn refresh_outputs(&self, ui: &AppWindow) {
        *self.outputs.borrow_mut() = Platform::outputs();
        apply_settings(ui, &self.svc.settings.borrow(), &self.outputs.borrow());
    }

    fn wire(&self, ui: &AppWindow) {
        let svc = self.svc.clone();
        ui.global::<Dev>().on_set_mode(move |i| {
            let mode = match i {
                0 => ListeningMode::Off,
                1 => ListeningMode::Transparency,
                2 => ListeningMode::Adaptive,
                _ => ListeningMode::NoiseCancellation,
            };
            svc.send(Command::SetListeningMode(mode));
        });
        let svc = self.svc.clone();
        ui.global::<Dev>().on_set(move |k, v| set(&svc, &k, &v));
        let svc = self.svc.clone();
        ui.global::<Prefs>().on_set(move |k, v| set(&svc, &k, &v));

        let (svc, outputs) = (self.svc.clone(), self.outputs.clone());
        ui.global::<Prefs>().on_set_fallback(move |i| {
            let id = if i == 0 { None } else { outputs.borrow().get(i as usize - 1).map(|o| o.id.clone()) };
            svc.update_settings(|s| s.fallback_output = id);
        });

        let (svc, rt) = (self.svc.clone(), self.rt.clone());
        ui.global::<Dev>().on_disconnect(move || {
            let addr = svc.state.borrow().address.clone();
            rt.spawn(async move {
                if let Err(e) = Platform::disconnect(&addr).await {
                    log::warn!("disconnect failed: {e}");
                }
            });
        });
        let rt = self.rt.clone();
        ui.global::<Prefs>().on_connect(move |addr| {
            let addr = addr.to_string();
            rt.spawn(async move {
                if let Err(e) = Platform::connect(&addr).await {
                    log::warn!("connect failed: {e}");
                }
            });
        });
        ui.global::<Prefs>().on_open_link(|what| {
            let url = match what.as_str() {
                "releases" => format!("{REPO}/releases"),
                "license" => format!("{REPO}/blob/main/LICENSE"),
                _ => format!("{REPO}/issues"),
            };
            open_url(&url);
        });

        let weak = ui.as_weak();
        ui.on_start_drag(move || {
            use slint::winit_030::WinitWindowAccessor;
            if let Some(ui) = weak.upgrade() {
                ui.window().with_winit_window(|w| {
                    let _ = w.drag_window();
                });
                // The compositor swallows the button release during a move, so Slint would
                // keep the pointer grabbed by the drag area and send every later click there.
                // Release it once this press has been handled; doing it inside the handler
                // is undone when the press finishes.
                let weak = ui.as_weak();
                slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.window().dispatch_event(slint::platform::WindowEvent::PointerReleased {
                            position: slint::LogicalPosition::new(-1.0, -1.0),
                            button: slint::platform::PointerEventButton::Left,
                        });
                        ui.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
                    }
                });
            }
        });
        let weak = ui.as_weak();
        ui.on_minimize(move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().set_minimized(true);
            }
        });
        ui.on_close_window(close);
        ui.window().on_close_requested(|| {
            close();
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
}

fn set(svc: &Service, key: &str, value: &str) {
    if key == "autostart" {
        crate::autostart::set(value == "on");
        // Settings didn't change, but the UI should re-read the toggle.
        svc.settings.send_modify(|_| {});
        return;
    }
    let state = svc.state.borrow().clone();
    let change = keys::set(&state, &svc.settings.borrow(), key, value);
    match change {
        Ok(Change::Device(c)) => svc.send(c),
        Ok(Change::App(s)) => svc.update_settings(|old| *old = s),
        Err(e) => log::warn!("{key}={value}: {e}"),
    }
}

fn open_url(url: &str) {
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(url).spawn();
}

/// Light only when the system explicitly asks for it; otherwise dark.
/// `BETTERPODS_THEME=light|dark` overrides the system.
pub fn system_is_light() -> bool {
    match std::env::var("BETTERPODS_THEME").as_deref() {
        Ok("light") => true,
        Ok("dark") => false,
        _ => matches!(dark_light::detect(), Ok(dark_light::Mode::Light)),
    }
}

fn apply_theme(ui: &AppWindow) {
    ui.global::<Theme>().set_dark(!system_is_light());
}

/// Push state and settings into the window until it closes.
async fn follow(ui: slint::Weak<AppWindow>, svc: Arc<Service>) {
    let mut state = svc.state.clone();
    let mut settings = svc.settings.subscribe();
    loop {
        tokio::select! {
            r = state.changed() => if r.is_err() { return },
            r = settings.changed() => if r.is_err() { return },
        }
        let s = state.borrow_and_update().clone();
        let p = settings.borrow_and_update().clone();
        let alive = ui
            .upgrade_in_event_loop(move |ui| {
                apply_state(&ui, &s);
                HOST.with(|h| {
                    if let Some(host) = h.borrow().as_ref() {
                        apply_settings(&ui, &p, &host.outputs.borrow());
                    }
                });
            })
            .is_ok();
        if !alive {
            return;
        }
    }
}

async fn load_known(ui: slint::Weak<AppWindow>) {
    let devices = Platform::known_devices().await;
    let _ = ui.upgrade_in_event_loop(move |ui| {
        let known: Vec<Known> = devices
            .iter()
            .map(|d| Known {
                name: d.name.clone().into(),
                address: d.address.clone().into(),
                subtitle: if d.connected { "Verbunden".into() } else { "Gekoppelt".into() },
            })
            .collect();
        let prefs = ui.global::<Prefs>();
        prefs.set_last_device(devices.first().map(|d| d.name.clone()).unwrap_or_default().into());
        prefs.set_known(ModelRc::new(VecModel::from(known)));
    });
}

fn apply_settings(ui: &AppWindow, s: &Settings, outputs: &[Output]) {
    let p = ui.global::<Prefs>();
    p.set_ear_detection(s.ear_detection);
    p.set_connect_auto(s.connect == ConnectPolicy::Auto);
    p.set_tray_battery(match s.tray_battery {
        TrayBattery::Off => 0,
        TrayBattery::Percent => 1,
        TrayBattery::Ring => 2,
    });
    p.set_notify_connected(s.notify_connected);
    p.set_notify_takeover(s.notify_takeover);
    p.set_autostart(crate::autostart::enabled());
    p.set_conversation_volume(s.conversation_volume as i32);
    p.set_device_name(s.device_name().into());
    p.set_version(format!("{} · {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS).into());

    let mut names: Vec<SharedString> = vec!["Automatisch".into()];
    names.extend(outputs.iter().map(|o| SharedString::from(o.name.as_str())));
    let selected =
        s.fallback_output.as_ref().and_then(|id| outputs.iter().position(|o| &o.id == id)).map_or(0, |i| i + 1);
    p.set_outputs(ModelRc::new(VecModel::from(names)));
    p.set_fallback(selected as i32);
}

fn apply_state(ui: &AppWindow, s: &DeviceState) {
    let d = ui.global::<Dev>();
    d.set_connected(s.connected && s.ready);
    d.set_name(if s.metadata.name.is_empty() { "AirPods".into() } else { s.metadata.name.as_str().into() });
    let model = s.model();
    d.set_headphones(model.is_some_and(|m| m.headphones));
    d.set_model_name(model.map_or(s.metadata.model_number.as_str(), |m| m.name).into());
    d.set_model_number(s.metadata.model_number.as_str().into());
    d.set_firmware(s.metadata.firmware.as_str().into());
    let serial = &s.metadata.serial_number;
    let tail: String = serial.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    d.set_serial(if serial.is_empty() { "".into() } else { format!("••••••••{tail}").into() });

    let mut bats = Vec::new();
    let mut line = Vec::new();
    let mut push = |kind, label: &str, short: &str, b: Option<betterpods_core::BudBattery>| {
        if let Some(b) = b {
            bats.push(Bat { kind, label: label.into(), level: b.level as i32, charging: b.charging });
            line.push(format!("{short}{}", b.level));
        }
    };
    push(3, "Kopfhörer", "", s.headset);
    push(0, "Links", "L ", s.left);
    push(1, "Rechts", "R ", s.right);
    push(2, "Case", "Case ", s.case);
    d.set_bats(ModelRc::new(VecModel::from(bats)));
    d.set_status_line(line.join(" · ").into());

    d.set_cap_listening(s.has(Capability::ListeningMode));
    d.set_cap_ca(s.has(Capability::ConversationAwareness));
    d.set_cap_adaptive(s.has(Capability::AdaptiveAudio));
    d.set_cap_pv(s.has(Capability::AdaptiveVolume));
    d.set_cap_swipe(s.has(Capability::SwipeForVolume));
    d.set_cap_sleep(s.has(Capability::SleepDetection));
    d.set_cap_head(s.has(Capability::HeadGestures));

    d.set_mode(match s.listening_mode() {
        Some(ListeningMode::Off) => 0,
        Some(ListeningMode::Transparency) => 1,
        Some(ListeningMode::Adaptive) => 2,
        Some(ListeningMode::NoiseCancellation) => 3,
        None => -1,
    });
    let flag = |id| s.flag(id).unwrap_or(false);
    let byte = |id| s.control(id).map_or(0, |v| v[0] as i32);
    d.set_allow_off(s.flag(ControlId::AllowOffOption).unwrap_or(true));
    d.set_ca(flag(ControlId::ConversationalAwareness));
    d.set_pv(flag(ControlId::AdaptiveVolume));
    d.set_strength(s.control(ControlId::AdaptiveStrength).map_or(50, |v| 100 - v[0].min(100) as i32));
    d.set_one_bud_anc(flag(ControlId::OneBudAnc));
    d.set_swipe(flag(ControlId::VolumeSwipeMode));
    d.set_swipe_interval((byte(ControlId::VolumeSwipeInterval) - 1).clamp(0, 2));
    d.set_press_speed(byte(ControlId::DoubleClickInterval).clamp(0, 2));
    d.set_hold_duration(byte(ControlId::ClickHoldInterval).clamp(0, 2));
    d.set_tone_volume(s.control(ControlId::ChimeVolume).map_or(70, |v| v[0] as i32));
    // Raw 0 auto, 1 right, 2 left; the UI lists auto, left, right.
    d.set_mic(match byte(ControlId::MicMode) {
        2 => 1,
        1 => 2,
        _ => 0,
    });
    d.set_cycle(s.control(ControlId::ListeningModeConfigs).map_or(14, |v| v[0] as i32));
    d.set_call_once_mutes(s.control(ControlId::CallManagement).is_none_or(|v| v[1] != 0x02));
    d.set_sleep(flag(ControlId::SleepDetection));
    d.set_optimized_charging(flag(ControlId::DynamicEndOfCharge));
}

/// Dev aid for `BETTERPODS_SNAPSHOT`: write the open window to `path` as PPM.
/// `BETTERPODS_PAGE` (0–5) picks the page first.
pub fn snapshot(path: &str) {
    HOST.with(|h| {
        let host = h.borrow();
        let Some(ui) = host.as_ref().and_then(|h| h.window.as_ref()) else {
            log::error!("snapshot: no window");
            return;
        };
        match ui.window().take_snapshot() {
            Ok(buf) => {
                let mut out = format!("P6\n{} {}\n255\n", buf.width(), buf.height()).into_bytes();
                out.extend(buf.as_slice().iter().flat_map(|p| [p.r, p.g, p.b]));
                if let Err(e) = std::fs::write(path, out) {
                    log::error!("snapshot: {e}");
                }
            }
            Err(e) => log::error!("snapshot: {e}"),
        }
    });
}
