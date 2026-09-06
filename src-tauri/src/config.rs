use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomTarget { pub name:String, pub host:String, pub port:u16, pub kind:String, pub domestic:bool, pub affects_state:bool, pub enabled:bool }
impl Default for CustomTarget { fn default()->Self { Self { name:String::new(),host:String::new(),port:443,kind:"tcp".into(),domestic:false,affects_state:false,enabled:true } } }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings { pub interval_ok:u64, pub interval_bad:u64, pub popup_enabled:bool, pub popup_only_outages:bool, pub popup_cooldown_secs:u64, pub popup_seconds:u64, pub toast_online:bool, pub quiet:bool, pub anchor_tray:bool, pub custom_targets:Vec<CustomTarget>, pub speedtest_down:String, pub speedtest_up:String, pub speed_in_bytes:bool }
impl Default for Settings { fn default()->Self { Self { interval_ok:15,interval_bad:5,popup_enabled:true,popup_only_outages:false,popup_cooldown_secs:30,popup_seconds:4,toast_online:true,quiet:false,anchor_tray:true,custom_targets:vec![],speedtest_down:"https://speed.cloudflare.com/__down?bytes=30000000".into(),speedtest_up:"https://speed.cloudflare.com/__up".into(),speed_in_bytes:false } } }
pub fn load(path:&Path)->Settings { std::fs::read_to_string(path).ok().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default() }
pub fn save(path:&Path,s:&Settings)->anyhow::Result<()> { std::fs::write(path,serde_json::to_string_pretty(s)?)?; Ok(()) }
