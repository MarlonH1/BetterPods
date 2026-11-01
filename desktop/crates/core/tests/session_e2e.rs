//! End-to-end: a real session against virtual AirPods over an in-memory link.

use std::time::Duration;

use aacp::{ControlId, EarState, ListeningMode};
use droidpods_core::fake::{ear_packet, mem_pair, run_fake, FakeConfig, FakeEvent};
use droidpods_core::{run_session, BudBattery, Command, DeviceState, Peers, SessionEnd};
use tokio::sync::{mpsc, watch};
use tokio::time::timeout;

async fn wait_for(rx: &mut watch::Receiver<DeviceState>, what: &str, f: impl Fn(&DeviceState) -> bool) -> DeviceState {
    timeout(Duration::from_secs(2), rx.wait_for(|s| f(s)))
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
        .expect("state sender dropped")
        .clone()
}

#[tokio::test]
async fn full_session_against_fake_airpods() {
    let (host, pods) = mem_pair();
    let (ev_tx, ev_rx) = mpsc::unbounded_channel();
    let (seen_tx, mut seen_rx) = mpsc::unbounded_channel();
    let fake = tokio::spawn(run_fake(pods, FakeConfig::default(), ev_rx, Some(seen_tx)));

    let (cmd_tx, cmd_rx) = mpsc::channel(8);
    let (state_tx, mut state_rx) = watch::channel(DeviceState::default());
    let session = tokio::spawn(run_session(
        host,
        Peers { airpods: "00:00:00:00:00:01".into(), local: None, name: "Test".into(), platform: "Linux" },
        cmd_rx,
        state_tx,
    ));

    // Handshake → features → notifications, in that order.
    assert_eq!(seen_rx.recv().await.unwrap(), aacp::HANDSHAKE);
    assert_eq!(seen_rx.recv().await.unwrap(), aacp::SET_SPECIFIC_FEATURES);
    assert_eq!(seen_rx.recv().await.unwrap(), aacp::REQUEST_NOTIFICATIONS);

    let s = wait_for(&mut state_rx, "initial state", |s| {
        s.ready && s.case.is_some() && s.flag(ControlId::ConversationalAwareness).is_some()
    })
    .await;
    assert_eq!(s.metadata.name, "AirPods Pro 2");
    assert_eq!(s.metadata.model_number, "A3047");
    assert_eq!(s.left, Some(BudBattery { level: 82, charging: false }));
    assert_eq!(s.right, Some(BudBattery { level: 79, charging: false }));
    assert_eq!(s.case, Some(BudBattery { level: 31, charging: true }));
    assert_eq!(s.listening_mode(), Some(ListeningMode::Adaptive));
    assert_eq!(s.flag(ControlId::ConversationalAwareness), Some(true));
    assert_eq!(s.min_bud_level(), Some(79));
    assert!(s.any_in_ear());

    // Switch to noise cancellation: the exact packet goes out and the state follows.
    cmd_tx.send(Command::SetListeningMode(ListeningMode::NoiseCancellation)).await.unwrap();
    assert_eq!(seen_rx.recv().await.unwrap(), aacp::set_listening_mode(ListeningMode::NoiseCancellation));
    wait_for(&mut state_rx, "ANC", |s| s.listening_mode() == Some(ListeningMode::NoiseCancellation)).await;

    cmd_tx.send(Command::SetFlag(ControlId::ConversationalAwareness, false)).await.unwrap();
    assert_eq!(seen_rx.recv().await.unwrap(), aacp::set_flag(ControlId::ConversationalAwareness, false));
    wait_for(&mut state_rx, "CA off", |s| s.flag(ControlId::ConversationalAwareness) == Some(false)).await;

    // Both buds taken out.
    ev_tx.send(FakeEvent::Raw(ear_packet(1, 1))).unwrap();
    let s = wait_for(&mut state_rx, "out of ear", |s| s.ear_primary == Some(EarState::OutOfEar)).await;
    assert!(!s.any_in_ear());

    // AirPods go away: session ends and the state resets.
    ev_tx.send(FakeEvent::Disconnect).unwrap();
    let end = timeout(Duration::from_secs(2), session).await.unwrap().unwrap();
    assert!(matches!(end, SessionEnd::Closed), "{end:?}");
    assert_eq!(*state_rx.borrow(), DeviceState::default());
    fake.await.unwrap();
}

#[tokio::test]
async fn session_stops_when_ui_goes_away() {
    let (host, pods) = mem_pair();
    let (_ev_tx, ev_rx) = mpsc::unbounded_channel();
    tokio::spawn(run_fake(pods, FakeConfig::default(), ev_rx, None));
    let (cmd_tx, cmd_rx) = mpsc::channel(1);
    let (state_tx, mut state_rx) = watch::channel(DeviceState::default());
    let session = tokio::spawn(run_session(
        host,
        Peers { airpods: "00:00:00:00:00:01".into(), local: None, name: "Test".into(), platform: "Linux" },
        cmd_rx,
        state_tx,
    ));
    wait_for(&mut state_rx, "ready", |s| s.ready).await;
    drop(cmd_tx);
    let end = timeout(Duration::from_secs(2), session).await.unwrap().unwrap();
    assert!(matches!(end, SessionEnd::Stopped), "{end:?}");
}
