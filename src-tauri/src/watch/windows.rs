use super::{Trigger, Tx};
use std::ffi::c_void;
use windows_sys::Win32::Foundation::{HANDLE, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{NotifyNetworkConnectivityHintChange, NotifyRouteChange2, MIB_IPFORWARD_ROW2, MIB_NOTIFICATION_TYPE};
use windows_sys::Win32::Networking::WinSock::{AF_UNSPEC, NL_NETWORK_CONNECTIVITY_HINT};
use windows_sys::Win32::System::Registry::{RegNotifyChangeKeyValue, REG_NOTIFY_CHANGE_LAST_SET};

unsafe extern "system" fn on_hint(context: *const c_void, _hint: NL_NETWORK_CONNECTIVITY_HINT) { let tx = &*(context as *const Tx); let _ = tx.try_send(Trigger::RouteChange); }
unsafe extern "system" fn on_route(context: *const c_void, row: *const MIB_IPFORWARD_ROW2, _kind: MIB_NOTIFICATION_TYPE) { if !row.is_null() && (*row).DestinationPrefix.PrefixLength != 0 { return; } let tx = &*(context as *const Tx); let _ = tx.try_send(Trigger::RouteChange); }

pub fn spawn(tx: Tx) {
    let leaked: &'static Tx = Box::leak(Box::new(tx.clone()));
    unsafe { let mut handle: HANDLE = std::ptr::null_mut(); let result = NotifyNetworkConnectivityHintChange(Some(on_hint), leaked as *const Tx as *const c_void, 0, &mut handle); if result != NO_ERROR { log::warn!("connectivity hint notify failed: {result}"); } let mut route_handle: HANDLE = std::ptr::null_mut(); let result = NotifyRouteChange2(AF_UNSPEC as u16, Some(on_route), leaked as *const Tx as *const c_void, 0, &mut route_handle); if result != NO_ERROR { log::warn!("route notify failed: {result}"); } }
    std::thread::Builder::new().name("proxy-watch".into()).spawn(move || { use winreg::enums::{HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ}; use winreg::RegKey; let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", KEY_READ | KEY_NOTIFY) { Ok(key) => key, Err(error) => { log::warn!("proxy key: {error}"); return; } }; loop { let result = unsafe { RegNotifyChangeKeyValue(key.raw_handle() as _, 0, REG_NOTIFY_CHANGE_LAST_SET, std::ptr::null_mut(), 0) }; if result != NO_ERROR { std::thread::sleep(std::time::Duration::from_secs(5)); continue; } let _ = tx.blocking_send(Trigger::ProxyChange); } }).ok();
}