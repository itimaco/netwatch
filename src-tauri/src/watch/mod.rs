pub mod heartbeat;
pub mod iface;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

use serde::Serialize;
use tauri::AppHandle;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Start, IfaceChange, RouteChange, ProxyChange, HeartbeatDown, HeartbeatUp, HeartbeatSlow, Followup, Manual, Timer,
}

impl Trigger {
    pub fn settle_ms(self) -> u64 { match self { Self::IfaceChange | Self::RouteChange => 1500, Self::ProxyChange => 600, _ => 0 } }
    pub fn needs_followup(self) -> bool { matches!(self, Self::IfaceChange | Self::RouteChange | Self::ProxyChange | Self::HeartbeatDown) }
}

pub type Tx = mpsc::Sender<Trigger>;

pub fn start(app: AppHandle) -> (Tx, mpsc::Receiver<Trigger>) {
    let (tx, rx) = mpsc::channel(64);
    tauri::async_runtime::spawn(iface::run(tx.clone()));
    tauri::async_runtime::spawn(heartbeat::run(app.clone(), tx.clone()));
    #[cfg(windows)] windows::spawn(tx.clone());
    #[cfg(target_os = "macos")] macos::spawn(tx.clone());
    #[cfg(target_os = "linux")] linux::spawn(tx.clone());
    (tx, rx)
}