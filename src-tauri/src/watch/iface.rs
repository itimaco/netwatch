use super::{Trigger, Tx};
use futures::StreamExt;
use if_watch::tokio::IfWatcher;
use if_watch::IfEvent;
use std::time::{Duration, Instant};

pub async fn run(tx: Tx) {
    let mut watcher = match IfWatcher::new() { Ok(watcher) => watcher, Err(error) => { log::warn!("if-watch unavailable: {error}"); return; } };
    let started = Instant::now();
    while let Some(event) = watcher.next().await {
        let Ok(event) = event else { continue };
        if started.elapsed() < Duration::from_millis(800) { continue; }
        let network = match &event { IfEvent::Up(network) | IfEvent::Down(network) => *network };
        if network.addr().is_loopback() || network.addr().is_unspecified() { continue; }
        if let std::net::IpAddr::V6(address) = network.addr() { if (address.segments()[0] & 0xffc0) == 0xfe80 { continue; } }
        log::info!("iface event: {event:?}");
        let _ = tx.try_send(Trigger::IfaceChange);
    }
}