//! DroidPods desktop app: companion window bound to a live AirPods session.
//!
//!     droidpods          connect to the AirPods paired with this PC
//!     droidpods --demo   run against virtual AirPods

use std::time::Duration;

use aacp::{ControlId, ListeningMode};
use droidpods_core::{fake, Command, DeviceState};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tokio::sync::{mpsc, watch};

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let demo = std::env::args().any(|a| a == "--demo");

    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(16);
    let (state_tx, state_rx) = watch::channel(DeviceState::default());

    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
    if demo {
        rt.spawn(run_demo(cmd_rx, state_tx));
    } else {
        rt.spawn(run_platform(cmd_rx, state_tx));
    }

    let ui = AppWindow::new()?;
    follow_system_theme(&ui);
    wire_callbacks(&ui, cmd_tx, rt.handle().clone());
    rt.spawn(push_state(ui.as_weak(), state_rx));
    if let Ok(path) = std::env::var("DROIDPODS_SNAPSHOT") {
        snapshot_then_quit(&ui, path);
    }

    ui.run()?;
    rt.shutdown_timeout(Duration::from_millis(200));
    Ok(())
}

/// Light only when the system explicitly asks for it; otherwise dark.
/// `DROIDPODS_THEME=light|dark` overrides the system.
fn follow_system_theme(ui: &AppWindow) {
    let forced = std::env::var("DROIDPODS_THEME").ok();
    let apply = move |ui: &AppWindow| {
        let light = match forced.as_deref() {
            Some("light") => true,
            Some("dark") => false,
            _ => matches!(dark_light::detect(), Ok(dark_light::Mode::Light)),
        };
        ui.global::<Theme>().set_dark(!light);
    };
    apply(ui);
    let weak = ui.as_weak();
    let timer = Box::leak(Box::new(slint::Timer::default()));
    timer.start(slint::TimerMode::Repeated, Duration::from_secs(2), move || {
        if let Some(ui) = weak.upgrade() {
            apply(&ui);
        }
    });
}

fn wire_callbacks(ui: &AppWindow, cmd: mpsc::Sender<Command>, rt: tokio::runtime::Handle) {
    let send = move |c: Command| {
        if cmd.try_send(c).is_err() {
            log::warn!("command dropped: no session or queue full");
        }
    };
    let s = send.clone();
    ui.on_set_mode(move |i| {
        let mode = match i {
            0 => ListeningMode::Off,
            1 => ListeningMode::Transparency,
            2 => ListeningMode::Adaptive,
            _ => ListeningMode::NoiseCancellation,
        };
        s(Command::SetListeningMode(mode));
    });
    let s = send.clone();
    ui.on_set_ca(move |on| s(Command::SetFlag(ControlId::ConversationalAwareness, on)));
    let s = send;
    ui.on_set_pv(move |on| s(Command::SetFlag(ControlId::AdaptiveVolume, on)));
    ui.on_disconnect(move || disconnect_bluetooth(&rt));

    let weak = ui.as_weak();
    ui.on_start_drag(move || {
        use slint::winit_030::WinitWindowAccessor;
        if let Some(ui) = weak.upgrade() {
            ui.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    });
    let weak = ui.as_weak();
    ui.on_minimize(move || {
        if let Some(ui) = weak.upgrade() {
            ui.window().set_minimized(true);
        }
    });
    ui.on_close_window(|| {
        // Phase 0: no tray yet, so closing quits.
        let _ = slint::quit_event_loop();
    });
}

/// Dev aid: render the window to a PPM file once state has settled, then quit.
fn snapshot_then_quit(ui: &AppWindow, path: String) {
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(1500), move || {
        let Some(ui) = weak.upgrade() else { return };
        match ui.window().take_snapshot() {
            Ok(buf) => {
                let mut out = format!("P6\n{} {}\n255\n", buf.width(), buf.height()).into_bytes();
                out.extend(buf.as_slice().iter().flat_map(|p| [p.r, p.g, p.b]));
                if let Err(e) = std::fs::write(&path, out) {
                    log::error!("snapshot: {e}");
                }
            }
            Err(e) => log::error!("snapshot: {e}"),
        }
        let _ = slint::quit_event_loop();
    });
}

async fn push_state(ui: slint::Weak<AppWindow>, mut rx: watch::Receiver<DeviceState>) {
    loop {
        let s = rx.borrow_and_update().clone();
        let ui = ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui.upgrade() {
                apply_state(&ui, &s);
            }
        });
        if rx.changed().await.is_err() {
            return;
        }
    }
}

fn apply_state(ui: &AppWindow, s: &DeviceState) {
    ui.set_connected(s.connected && s.ready);
    let name = if s.metadata.name.is_empty() { "AirPods" } else { &s.metadata.name };
    ui.set_device_name(name.into());

    let mut bats = Vec::new();
    let mut line = Vec::new();
    let mut push = |kind, label: &str, short: &str, b: Option<droidpods_core::BudBattery>| {
        if let Some(b) = b {
            bats.push(Bat { kind, label: label.into(), level: b.level as i32, charging: b.charging });
            line.push(format!("{short} {}", b.level));
        }
    };
    push(3, "Kopfhörer", "", s.headset);
    // Left/right follow the physical side, not the primary bud.
    push(0, "Links", "L", s.left);
    push(1, "Rechts", "R", s.right);
    push(2, "Case", "Case", s.case);
    ui.set_bats(ModelRc::new(VecModel::from(bats)));
    ui.set_status_line(SharedString::from(line.join(" · ").trim()));

    ui.set_mode(match s.listening_mode {
        Some(ListeningMode::Off) => 0,
        Some(ListeningMode::Transparency) => 1,
        Some(ListeningMode::Adaptive) => 2,
        Some(ListeningMode::NoiseCancellation) => 3,
        None => -1,
    });
    // AirPods without the "Off" option configured hide it, like the Android app.
    ui.set_show_off(s.allow_off_mode.unwrap_or(true) || s.listening_mode == Some(ListeningMode::Off));
    ui.set_ca(s.conversational_awareness.unwrap_or(false));
    ui.set_ca_known(s.conversational_awareness.is_some());
    ui.set_pv(s.adaptive_volume.unwrap_or(false));
    ui.set_pv_known(s.adaptive_volume.is_some());
}

async fn run_demo(cmd_rx: mpsc::Receiver<Command>, state_tx: watch::Sender<DeviceState>) {
    let (host, pods) = fake::mem_pair();
    let (_ev_tx, ev_rx) = mpsc::unbounded_channel();
    tokio::spawn(fake::run_fake(pods, fake::FakeConfig::default(), ev_rx, None));
    droidpods_core::run_session(host, cmd_rx, state_tx).await;
    std::future::pending::<()>().await;
}

#[cfg(target_os = "linux")]
async fn run_platform(cmd_rx: mpsc::Receiver<Command>, state_tx: watch::Sender<DeviceState>) {
    if let Err(e) = droidpods_linux::supervise(cmd_rx, state_tx, Duration::from_secs(3)).await {
        log::error!("Bluetooth unavailable: {e}");
    }
}

#[cfg(not(target_os = "linux"))]
async fn run_platform(_cmd_rx: mpsc::Receiver<Command>, _state_tx: watch::Sender<DeviceState>) {
    log::error!("this platform has no AirPods transport yet; try --demo");
}

#[cfg(target_os = "linux")]
fn disconnect_bluetooth(rt: &tokio::runtime::Handle) {
    rt.spawn(async {
        let result: bluer::Result<()> = async {
            let bt = bluer::Session::new().await?;
            let adapter = bt.default_adapter().await?;
            for p in droidpods_linux::paired_airpods(&bt).await? {
                if p.connected {
                    adapter.device(p.address)?.disconnect().await?;
                }
            }
            Ok(())
        }
        .await;
        if let Err(e) = result {
            log::warn!("disconnect failed: {e}");
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn disconnect_bluetooth(_rt: &tokio::runtime::Handle) {}
