//! BetterPods desktop app.
//!
//!     betterpods open         show the window (starts the app if needed)
//!     betterpods daemon       run in the background with the tray icon
//!     betterpods <command>    control the running app from the shell, see --help
//!
//! `--demo` runs against virtual AirPods.

mod autostart;
mod cli;
mod ipc;
mod keys;
mod platform;
mod service;
mod tray;
mod ui;

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = cli::Cli::parse();
    match args.command {
        Some(cli::Cmd::Open) => {
            if cli::daemon_running() {
                return cli::run(cli::Cli { command: Some(cli::Cmd::Show), ..args });
            }
            run_app(args.demo, true)
        }
        Some(cli::Cmd::Daemon { hidden }) => {
            if cli::daemon_running() {
                eprintln!("BetterPods is already running");
                return ExitCode::SUCCESS;
            }
            run_app(args.demo, !hidden)
        }
        _ => cli::run(args),
    }
}

/// The app itself: service, tray and CLI socket on a small Tokio runtime, the
/// window on the main thread.
fn run_app(demo: bool, show: bool) -> ExitCode {
    let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let (show_tx, mut show_rx) = tokio::sync::mpsc::unbounded_channel();
    let svc = rt.block_on(service::Service::start(demo, show_tx));
    rt.spawn(svc.clone().serve());
    rt.spawn(tray::run(svc.clone()));
    rt.spawn(async move {
        while show_rx.recv().await.is_some() {
            ui::show();
        }
    });

    // zbus (used by dark-light for the theme) runs on Tokio, also from the UI thread.
    let _rt_context = rt.enter();
    ui::init(svc, rt.handle().clone());
    if show {
        ui::show_now();
    }
    if let Ok(path) = std::env::var("BETTERPODS_SNAPSHOT") {
        // Dev aid: render the window to a PPM file once state has settled, then quit.
        slint::Timer::single_shot(Duration::from_millis(1500), move || {
            ui::snapshot(&path);
            let _ = slint::quit_event_loop();
        });
    }
    let result = slint::run_event_loop_until_quit();
    let _ = std::fs::remove_file(ipc::socket_path());
    rt.shutdown_timeout(Duration::from_millis(200));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
