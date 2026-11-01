//! Hardware check: list paired AirPods and, if one is connected, open the AACP
//! channel and print what it reports.
//!
//!     cargo run -p droidpods-linux --example probe [seconds] [off|transparency|adaptive|anc]
//!
//! With a mode, it switches to it once the session is ready, waits for the
//! AirPods to confirm, then switches back.

use std::time::Duration;

use droidpods_core::{run_session, Command, DeviceState};
use droidpods_linux::{paired_airpods, L2capLink};
use tokio::sync::{mpsc, watch};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(8);
    let switch_to = std::env::args().nth(2).and_then(|s| aacp::ListeningMode::parse(&s));

    let bt = bluer::Session::new().await?;
    let pods = paired_airpods(&bt).await?;
    if pods.is_empty() {
        println!("no paired AirPods found");
        return Ok(());
    }
    for p in &pods {
        println!("{}  {}  {}", p.address, if p.connected { "connected" } else { "not connected" }, p.name);
    }
    let Some(p) = pods.into_iter().find(|p| p.connected) else {
        println!("none connected — connect them and run again");
        return Ok(());
    };

    let link = L2capLink::connect(p.address).await?;
    println!("L2CAP channel open to {}", p.address);
    let (cmd_tx, cmd_rx) = mpsc::channel(1);
    let (state_tx, mut state_rx) = watch::channel(DeviceState::default());
    let session = tokio::spawn(run_session(link, cmd_rx, state_tx));

    let deadline = tokio::time::sleep(Duration::from_secs(secs));
    tokio::pin!(deadline);
    let mut original = None;
    loop {
        tokio::select! {
            _ = &mut deadline => break,
            changed = state_rx.changed() => {
                if changed.is_err() { break; }
                let s = state_rx.borrow().clone();
                println!("{s:#?}");
                if let (Some(target), None, Some(current)) = (switch_to, original, s.listening_mode) {
                    original = Some(current);
                    println!(">>> switching {current:?} -> {target:?}");
                    cmd_tx.send(Command::SetListeningMode(target)).await?;
                }
            }
        }
    }
    if let Some(m) = original {
        println!(">>> restoring {m:?}");
        cmd_tx.send(Command::SetListeningMode(m)).await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    session.abort();
    Ok(())
}
