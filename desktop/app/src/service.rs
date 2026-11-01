//! The long-running part of the app: owns the AirPods session, settings and the
//! automatic behaviour (ear detection, conversational awareness, notifications),
//! and answers the CLI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aacp::{Component, ControlId, EarState, ListeningMode};
use betterpods_core::ear::{EarAction, EarPolicy};
use betterpods_core::{BudBattery, Command, ConnectPolicy, DeviceState, Settings};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, mpsc, watch};

use crate::ipc::{exit, BudJson, Event, Request, Response, StatusJson};
use crate::keys::{self, Change, KeyError};
use crate::platform::Platform;

/// Battery level at which the iPhone warns, and so do we.
const LOW_BATTERY: u8 = 20;

pub struct Service {
    pub state: watch::Receiver<DeviceState>,
    pub settings: watch::Sender<Settings>,
    cmd: mpsc::Sender<Command>,
    events: broadcast::Sender<Event>,
    /// Asks the UI thread to show the window.
    show: mpsc::UnboundedSender<()>,
    pub platform: Platform,
    /// When we last took the AirPods; they briefly report losing ownership afterwards.
    took_at: Mutex<Option<Instant>>,
    /// When another device last took them, to report it once.
    lost_at: Mutex<Option<Instant>>,
    /// Playback started while the AirPods were elsewhere: take them once connected.
    take_on_connect: AtomicBool,
}

impl Service {
    pub async fn start(demo: bool, show: mpsc::UnboundedSender<()>) -> Arc<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel(16);
        let (state_tx, state_rx) = watch::channel(DeviceState::default());
        let settings = watch::Sender::new(Settings::load());
        if demo {
            tokio::spawn(run_demo(cmd_rx, state_tx));
        } else {
            tokio::spawn(Platform::run_connection(cmd_rx, state_tx, settings.subscribe()));
        }
        let svc = Arc::new(Self {
            state: state_rx,
            settings,
            cmd: cmd_tx,
            events: broadcast::channel(64).0,
            show,
            platform: Platform::new().await,
            took_at: Mutex::new(None),
            lost_at: Mutex::new(None),
            take_on_connect: AtomicBool::new(false),
        });
        tokio::spawn(svc.clone().run_behaviour());
        tokio::spawn(svc.clone().follow_playback());
        svc
    }

    pub fn send(&self, c: Command) {
        if self.cmd.try_send(c).is_err() {
            log::warn!("command dropped: no session or queue full");
        }
    }

    pub fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
        self.settings.send_modify(f);
        if let Err(e) = self.settings.borrow().save() {
            log::warn!("saving settings: {e}");
        }
    }

    fn emit(&self, e: Event) {
        let _ = self.events.send(e);
    }

    /// React to every state change like an iPhone would.
    async fn run_behaviour(self: Arc<Self>) {
        let mut rx = self.state.clone();
        let mut prev = DeviceState::default();
        let mut ear = EarPolicy::default();
        let mut ducked_from: Option<u32> = None;
        loop {
            if rx.changed().await.is_err() {
                return;
            }
            let now = rx.borrow_and_update().clone();
            let settings = self.settings.borrow().clone();

            if now.ready && !prev.ready {
                ear.reset();
                self.emit(Event::Connect { name: now.metadata.name.clone(), battery: battery_line(&now) });
                if settings.notify_connected {
                    let (title, body) = (now.metadata.name.clone(), battery_line(&now));
                    tokio::spawn(Platform::notify(title, format!("Verbunden · {body}"), None));
                }
            }
            if !now.connected && prev.connected {
                ear.reset();
                ducked_from = None;
                self.platform.forget_paused().await;
                self.emit(Event::Disconnect);
            }

            if (now.ear_primary, now.ear_secondary) != (prev.ear_primary, prev.ear_secondary) {
                if let (Some(p), Some(s)) = (now.ear_primary, now.ear_secondary) {
                    let (l, r) = left_right(&now);
                    self.emit(Event::Ear { left: l, right: r });
                    if settings.ear_detection {
                        for action in ear.update(p, s) {
                            self.ear_action(action, &now.address, &settings).await;
                        }
                    }
                }
            }

            if now.ready && battery_levels(&now) != battery_levels(&prev) && prev.ready {
                let low = now.min_bud_level().is_some_and(|l| l <= LOW_BATTERY)
                    && prev.min_bud_level().is_some_and(|l| l > LOW_BATTERY);
                self.emit(Event::Battery { battery: battery_line(&now), low });
                if low {
                    let name = now.metadata.name.clone();
                    tokio::spawn(Platform::notify(name, format!("Akku niedrig · {}", battery_line(&now)), None));
                }
            }

            if now.listening_mode() != prev.listening_mode() {
                if let Some(m) = now.listening_mode() {
                    self.emit(Event::Mode {
                        from: prev.listening_mode().map(|m| m.as_str().into()),
                        to: m.as_str().into(),
                    });
                }
            }

            // An explicit SetOwnershipToFalse always counts. The audio source alone lags behind
            // our own takeover for a few seconds, so it only counts once that has settled.
            let local_was_playing = prev.audio_source.is_some_and(|(m, k)| k != 0 && Some(m) == prev.local);
            let told = now.taken_count > prev.taken_count;
            let source_moved = local_was_playing
                && now.other_device_playing(now.local.as_ref())
                && !recent(&self.took_at, 10);
            if (told || source_moved) && now.ready && !recent(&self.lost_at, 5) {
                *self.lost_at.lock().unwrap() = Some(Instant::now());
                tokio::spawn(self.clone().lost_audio(now.clone()));
            }
            if now.ready && now.audio_source.is_some() && self.take_on_connect.swap(false, Ordering::Relaxed) {
                self.take_over(&now).await;
            }

            if now.conversation_level != prev.conversation_level {
                match now.conversation_level {
                    // Like the Android app: 1–2 means you're speaking, 6/8/9 that you stopped.
                    Some(1 | 2) if ducked_from.is_none() => {
                        if let Some(v) = self.platform.volume().await {
                            ducked_from = Some(v);
                            let target = v.min(settings.conversation_volume as u32);
                            self.fade_volume(v, target).await;
                        }
                    }
                    Some(6 | 8 | 9) => {
                        if let Some(v) = ducked_from.take() {
                            let current = self.platform.volume().await.unwrap_or(v);
                            self.fade_volume(current, v).await;
                        }
                    }
                    _ => {}
                }
            }
            prev = now;
        }
    }

    /// Take the AirPods when this computer starts playing, like a Mac does.
    async fn follow_playback(self: Arc<Self>) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let svc = self.clone();
        tokio::spawn(async move {
            svc.platform
                .watch_playback(move || {
                    let _ = tx.send(());
                })
                .await
        });
        while rx.recv().await.is_some() {
            if self.settings.borrow().connect != ConnectPolicy::Auto {
                continue;
            }
            let s = self.state.borrow().clone();
            if s.ready {
                // Not worn: keep playing on whatever output is active.
                if !s.any_in_ear() {
                    log::debug!("playback started, AirPods not worn");
                } else if s.should_take_for_playback(s.local.as_ref()) {
                    self.take_over(&s).await;
                } else {
                    log::info!("playback started, not taking the AirPods: {:?}", s.audio_source);
                }
            } else if !s.connected {
                let Some(d) = Platform::known_devices().await.into_iter().next() else { continue };
                log::info!("playback started, connecting {}", d.name);
                self.take_on_connect.store(true, Ordering::Relaxed);
                if let Err(e) = Platform::connect(&d.address).await {
                    log::info!("connect failed: {e}");
                    self.take_on_connect.store(false, Ordering::Relaxed);
                }
            }
        }
    }

    async fn take_over(&self, s: &DeviceState) {
        if !s.should_take_for_playback(s.local.as_ref()) || recent(&self.took_at, 10) {
            return;
        }
        let from = s.audio_source.and_then(|(m, _)| s.device_name(&m)).unwrap_or_else(|| "anderes Gerät".into());
        log::info!("taking the AirPods from {from}");
        *self.took_at.lock().unwrap() = Some(Instant::now());
        self.platform.route_to_airpods(&s.address, true).await;
        self.send(Command::TakeOver);
        self.emit(Event::Route { text: format!("{from} spielt ab → übernommen") });
    }

    /// Another device took the AirPods: pause here, move to the fallback output and
    /// offer to take them back.
    async fn lost_audio(self: Arc<Self>, s: DeviceState) {
        let other = s.taken_by.or(s.audio_source.map(|(m, _)| m));
        let name = other.and_then(|m| s.device_name(&m));
        log::info!("AirPods taken by {}", name.as_deref().unwrap_or("another device"));
        self.emit(Event::Route { text: format!("übernommen von {}", name.as_deref().unwrap_or("anderem Gerät")) });
        let paused = self.platform.playing().await;
        if paused {
            self.platform.pause().await;
        }
        let fallback = self.settings.borrow().fallback_output.clone();
        self.platform.route_away(&s.address, fallback).await;
        if !self.settings.borrow().notify_takeover {
            self.platform.forget_paused().await;
            return;
        }
        let title = format!("{} → {}", s.metadata.name, name.as_deref().unwrap_or("anderes Gerät"));
        let body = match &name {
            Some(n) => format!("Wiedergabe läuft jetzt auf „{n}“"),
            None => "Wiedergabe läuft jetzt auf einem anderen Gerät".into(),
        };
        if Platform::notify(title, body, Some("Zurückholen")).await {
            let now = self.state.borrow().clone();
            if now.ready {
                *self.took_at.lock().unwrap() = None;
                self.take_over(&now).await;
                if paused {
                    self.platform.resume().await;
                }
            }
        } else {
            // Like an iPhone: no automatic resume once someone else plays.
            self.platform.forget_paused().await;
        }
    }

    async fn fade_volume(&self, from: u32, to: u32) {
        const STEPS: u32 = 8;
        for i in 1..=STEPS {
            let v = from as i64 + (to as i64 - from as i64) * i as i64 / STEPS as i64;
            self.platform.set_volume(v as u32).await;
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    }

    async fn ear_action(&self, action: EarAction, addr: &str, settings: &Settings) {
        log::info!("ear detection: {action:?}");
        match action {
            EarAction::Pause => {
                if self.platform.playing().await {
                    self.platform.pause().await;
                    self.emit(Event::Media { action: "paused".into() });
                }
            }
            EarAction::Resume => {
                self.platform.resume().await;
                self.emit(Event::Media { action: "resumed".into() });
            }
            EarAction::RouteToAirPods => self.platform.route_to_airpods(addr, false).await,
            EarAction::RouteToFallback => {
                // Let the player actually pause before its output moves.
                tokio::time::sleep(Duration::from_millis(350)).await;
                if !self.state.borrow().any_in_ear() {
                    self.platform.route_away(addr, settings.fallback_output.clone()).await;
                }
            }
        }
    }

    // ------------------------------------------------------------------ IPC

    pub async fn serve(self: Arc<Self>) {
        let path = crate::ipc::socket_path();
        let _ = std::fs::remove_file(&path);
        let listener = match tokio::net::UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                log::error!("CLI socket {}: {e}", path.display());
                return;
            }
        };
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            let svc = self.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut line = String::new();
                if BufReader::new(read).read_line(&mut line).await.is_err() {
                    return;
                }
                let req: Request = match serde_json::from_str(&line) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = reply(&mut write, &Response::err(exit::INVALID, e)).await;
                        return;
                    }
                };
                if let Request::Watch = req {
                    let mut events = svc.events.subscribe();
                    while let Ok(e) = events.recv().await {
                        let mut text = serde_json::to_string(&e).unwrap_or_default();
                        text.push('\n');
                        if write.write_all(text.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                    return;
                }
                let resp = svc.handle(req).await;
                let _ = reply(&mut write, &resp).await;
            });
        }
    }

    async fn handle(&self, req: Request) -> Response {
        let state = self.state.borrow().clone();
        match req {
            Request::Status => Response::ok(status_json(&state)),
            Request::Show => {
                let _ = self.show.send(());
                Response::ok(())
            }
            Request::Mode { value, toggle } => self.mode(&state, value, toggle),
            Request::Get { key } => {
                let all = keys::get_all(&state, &self.settings.borrow());
                match key {
                    None => Response::ok(all),
                    Some(k) => match all.into_iter().find(|(name, _)| *name == k) {
                        Some((_, v)) => Response::ok(v),
                        None if !state.ready => Response::err(exit::NOT_CONNECTED, KeyError::NotConnected),
                        None => Response::err(exit::INVALID, KeyError::Unknown(k)),
                    },
                }
            }
            Request::Set { key, value } => {
                let change = keys::set(&state, &self.settings.borrow(), &key, &value);
                match change {
                    Ok(Change::Device(c)) => {
                        self.send(c);
                        Response::ok(())
                    }
                    Ok(Change::App(s)) => {
                        self.update_settings(|old| *old = s);
                        Response::ok(())
                    }
                    Err(e @ KeyError::NotConnected) => Response::err(exit::NOT_CONNECTED, e),
                    Err(e) => Response::err(exit::INVALID, e),
                }
            }
            Request::Devices => Response::ok(
                Platform::known_devices()
                    .await
                    .into_iter()
                    .map(|d| serde_json::json!({"address": d.address, "name": d.name, "connected": d.connected}))
                    .collect::<Vec<_>>(),
            ),
            Request::Connect => {
                let devices = Platform::known_devices().await;
                let Some(d) = devices.first() else { return Response::err(exit::INVALID, "no paired AirPods") };
                match Platform::connect(&d.address).await {
                    Ok(()) => Response::ok(()),
                    Err(e) => Response::err(exit::NOT_CONNECTED, e),
                }
            }
            Request::Disconnect => {
                if !state.connected {
                    return Response::err(exit::NOT_CONNECTED, "no AirPods connected");
                }
                match Platform::disconnect(&state.address).await {
                    Ok(()) => Response::ok(()),
                    Err(e) => Response::err(exit::INVALID, e),
                }
            }
            Request::Watch => unreachable!("handled by the connection loop"),
        }
    }

    fn mode(&self, state: &DeviceState, value: Option<String>, toggle: Vec<String>) -> Response {
        if !state.ready {
            return Response::err(exit::NOT_CONNECTED, "no AirPods connected");
        }
        let current = state.listening_mode();
        let Some(value) = value else {
            return Response::ok(current.map(|m| m.as_str()));
        };
        let target = match value.as_str() {
            "next" => {
                // Like press-and-hold: the next mode in the configured cycle.
                let bits = state.control(ControlId::ListeningModeConfigs).map_or(0x0E, |d| d[0]);
                let cycle: Vec<_> = ListeningMode::ALL.into_iter().filter(|m| bits & m.cycle_bit() != 0).collect();
                let pos = cycle.iter().position(|m| Some(*m) == current);
                pos.map_or(cycle.first(), |p| cycle.get((p + 1) % cycle.len())).copied()
            }
            "toggle" => {
                let modes: Vec<_> = toggle.iter().filter_map(|t| ListeningMode::parse(t)).collect();
                if modes.len() != 2 || modes.len() != toggle.len() {
                    return Response::err(exit::INVALID, "toggle takes two modes, e.g. toggle anc off");
                }
                Some(if current == Some(modes[0]) { modes[1] } else { modes[0] })
            }
            other => match ListeningMode::parse(other) {
                Some(m) => Some(m),
                None => {
                    return Response::err(
                        exit::INVALID,
                        format!("unknown mode '{other}' (off, transparency, adaptive, anc, next, toggle)"),
                    )
                }
            },
        };
        let Some(target) = target else { return Response::err(exit::INVALID, "no modes to cycle through") };
        self.send(Command::SetListeningMode(target));
        // `next`/`toggle` print what they switched to; a plain mode stays quiet.
        Response::ok(if value == "next" || value == "toggle" { Some(target.as_str()) } else { None })
    }
}

/// Whether `at` holds a time less than `secs` seconds ago.
fn recent(at: &Mutex<Option<Instant>>, secs: u64) -> bool {
    at.lock().unwrap().is_some_and(|t| t.elapsed() < Duration::from_secs(secs))
}

async fn reply(w: &mut tokio::net::unix::OwnedWriteHalf, r: &Response) -> std::io::Result<()> {
    let mut text = serde_json::to_string(r).unwrap_or_default();
    text.push('\n');
    w.write_all(text.as_bytes()).await
}

/// In-ear flags by physical side; ear packets list the primary bud first.
pub fn left_right(s: &DeviceState) -> (Option<bool>, Option<bool>) {
    let p = s.ear_primary.map(|e| e == EarState::InEar);
    let q = s.ear_secondary.map(|e| e == EarState::InEar);
    match s.primary {
        Some(Component::Right) => (q, p),
        _ => (p, q),
    }
}

fn battery_levels(s: &DeviceState) -> [Option<BudBattery>; 4] {
    [s.left, s.right, s.case, s.headset]
}

/// `L 82  R 79  C 31`, or the headset level for AirPods Max.
pub fn battery_line(s: &DeviceState) -> String {
    if let Some(h) = s.headset {
        return format!("{} %", h.level);
    }
    [("L", s.left), ("R", s.right), ("C", s.case)]
        .into_iter()
        .filter_map(|(k, b)| b.map(|b| format!("{k} {}", b.level)))
        .collect::<Vec<_>>()
        .join("  ")
}

fn status_json(s: &DeviceState) -> StatusJson {
    let (l, r) = left_right(s);
    let mut battery = std::collections::BTreeMap::new();
    let mut put = |k: &str, b: Option<BudBattery>, in_ear| {
        if let Some(b) = b {
            battery.insert(k.into(), BudJson { level: b.level, charging: b.charging, in_ear });
        }
    };
    put("left", s.left, l);
    put("right", s.right, r);
    put("case", s.case, None);
    put("headset", s.headset, l.or(r));
    StatusJson {
        model: s.model().map_or_else(|| s.metadata.model_number.clone(), |m| m.name.into()),
        name: s.metadata.name.clone(),
        connected: s.ready,
        battery,
        mode: s.listening_mode().map(|m| m.as_str().into()),
    }
}

async fn run_demo(cmd_rx: mpsc::Receiver<Command>, state_tx: watch::Sender<DeviceState>) {
    use betterpods_core::fake;
    let (host, pods) = fake::mem_pair();
    let (_ev_tx, ev_rx) = mpsc::unbounded_channel();
    tokio::spawn(fake::run_fake(pods, fake::FakeConfig::default(), ev_rx, None));
    betterpods_core::run_session(
        host,
        betterpods_core::Peers {
            airpods: "00:00:00:00:00:01".into(),
            local: None,
            name: "Demo".into(),
            platform: "Linux",
        },
        cmd_rx,
        state_tx,
    )
    .await;
    std::future::pending::<()>().await;
}
