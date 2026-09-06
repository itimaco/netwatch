use super::{Trigger, Tx};
use crate::probes::net;
use crate::{tray, AppState};
use std::net::SocketAddr;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const INTERNATIONAL: &[&str] = &["1.1.1.1:443", "8.8.8.8:443", "9.9.9.9:443"];
const DOMESTIC: &[&str] = &["178.22.122.100:53", "185.51.200.2:53"];

struct Track { fails: u8, up: bool, ewma: f32, slow: u8 }
impl Track {
    fn new() -> Self { Self { fails: 0, up: true, ewma: 0.0, slow: 0 } }
    fn feed(&mut self, milliseconds: Option<u32>) -> (Option<bool>, bool) {
        match milliseconds {
            Some(milliseconds) => { let milliseconds = milliseconds as f32; let spike = self.ewma > 0.0 && milliseconds > 300.0 && milliseconds > self.ewma * 2.5; self.slow = if spike { self.slow.saturating_add(1) } else { 0 }; self.ewma = if self.ewma == 0.0 { milliseconds } else { self.ewma * 0.8 + milliseconds * 0.2 }; self.fails = 0; let recovered = !self.up; self.up = true; (recovered.then_some(true), self.slow == 5) }
            None => { self.fails = self.fails.saturating_add(1); if self.up && self.fails >= 2 { self.up = false; return (Some(false), false); } (None, false) }
        }
    }
}

pub async fn run(app: AppHandle, tx: Tx) {
    let mut index = 0usize;
    let mut international = Track::new();
    let mut domestic = Track::new();
    loop {
        let state = app.state::<AppState>();
        let every = if state.snapshot.lock().state == crate::state::NetState::Online { 2000 } else { 1500 };
        tokio::time::sleep(Duration::from_millis(every)).await;
        if state.checking.load(std::sync::atomic::Ordering::SeqCst) { continue; }
        let international_address: SocketAddr = INTERNATIONAL[index % INTERNATIONAL.len()].parse().unwrap();
        let domestic_address: SocketAddr = DOMESTIC[index % DOMESTIC.len()].parse().unwrap();
        index += 1;
        let (international_result, domestic_result) = tokio::join!(net::tcp(international_address), net::tcp(domestic_address));
        let (international_change, slow) = international.feed(international_result);
        let (domestic_change, _) = domestic.feed(domestic_result);
        let tooltip_snapshot = { let mut snapshot = state.snapshot.lock(); let ewma = |old: Option<u32>, new: Option<u32>| match (old, new) { (Some(old), Some(new)) => Some(((old as f32) * 0.7 + (new as f32) * 0.3) as u32), (None, new) => new, (old, None) => old }; snapshot.international_ping = ewma(snapshot.international_ping, international_result); snapshot.domestic_ping = ewma(snapshot.domestic_ping, domestic_result); snapshot.clone() }; tray::update_tooltip(&app, &tooltip_snapshot);
        let trigger = match (international_change, domestic_change) { (Some(false), _) | (_, Some(false)) => Some(Trigger::HeartbeatDown), (Some(true), _) | (_, Some(true)) => Some(Trigger::HeartbeatUp), _ if slow => Some(Trigger::HeartbeatSlow), _ => None };
        if let Some(trigger) = trigger { let _ = tx.try_send(trigger); }
    }
}