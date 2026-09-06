use crate::{
    history,
    popup,
    probes::{custom, dns, domestic, exit, gateway, international, net, proxy, vpn, URL_204},
    state::{NetState, Snapshot},
    tray,
    watch::{self, Trigger},
    AppState,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::Ordering::SeqCst,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use crate::state::Sample;

pub async fn probe_all(app: &AppHandle) -> Snapshot {
    let settings = app.state::<AppState>().settings.read().clone();
    let tun = tokio::task::spawn_blocking(vpn::tun_interface).await.ok().flatten();
    let nic = tokio::task::spawn_blocking(vpn::physical_nic).await.ok().flatten();
    let (route, gw_ip) = match (&tun, &nic) {
        (None, _) => (Some(net::Route::direct()), None),
        (Some(_), Some(n)) => (Some(net::Route::bound(n.ip, n.name.clone(), n.index)), n.gateway),
        (Some(_), None) => (None, None),
    };
    let raw_known = route.is_some();
    let route = route.unwrap_or_default();
    let tunnel_client = net::direct_client();
    let (gw, dom, intl, dns, prx, cst, exit, canary) = tokio::join!(
        gateway::check(&route, gw_ip),
        domestic::check(&route),
        international::check(&route),
        dns::check(),
        proxy::detect(URL_204),
        custom::check(&tunnel_client, &settings.custom_targets),
        exit::check(&tunnel_client),
        exit::canary(&tunnel_client),
    );

    let mut s = Snapshot {
        ts: chrono::Utc::now().timestamp(),
        gateway_ok: gw.0,
        gateway_ms: gw.1,
        dns,
        proxy: prx,
        tun,
        raw_known,
        canary_direct_ok: canary.is_some(),
        exit,
        ..Default::default()
    };

    s.via_vpn = s.exit.is_foreign() || s.proxy.works == Some(true);
    s.vpn_ping = if s.via_vpn { s.exit.ms.or(s.proxy.exit.ms) } else { None };
    s.samples = dom.iter().cloned().chain(intl.iter().cloned()).chain(cst.iter().cloned()).collect();

    let affecting: Vec<String> = settings
        .custom_targets
        .iter()
        .filter(|t| t.affects_state)
        .map(|t| t.name.clone())
        .collect();
    s.domestic_ok = dom.iter().any(|x| x.ok)
        || cst.iter().any(|x| x.group == "custom_domestic" && x.ok && affecting.contains(&x.name));
    s.domestic_ping = dom.iter().filter_map(|x| x.ms).min();
    let hard_ok = intl.iter().filter(|x| x.proto != "icmp" && x.ok).count();
    let icmp_ok = intl.iter().filter(|x| x.proto == "icmp" && x.ok).count();
    s.international_ok = hard_ok >= 1;
    s.international_icmp_only = !s.international_ok && icmp_ok > 0;
    let mut lat: Vec<u32> = intl.iter().filter_map(|x| x.ms).collect();
    lat.sort_unstable();
    s.international_ping = lat.get(lat.len() / 2).copied();
    if !raw_known && s.tun.is_some() && !s.via_vpn {
        s.note = Some("سنجش مستقیم خط ممکن نبود؛ برای اطمینان فیلترشکن را موقتاً ببندید.".into());
    } else if s.via_vpn && !s.international_ok {
        s.note = Some("رهیاب بین‌المللی در VPN در دسترس نیست؛ از مسیریابی و سرور جدید مطمئن شوید.".into());
    } else {
        s.note = None;
    }

    s
}

pub struct Quality {
    seen_ok: HashMap<String, Instant>,
    base: Option<f32>,
    bad: u8,
    good: u8,
    pub degraded: bool,
}

impl Quality {
    pub fn new() -> Self { Self { seen_ok: HashMap::new(), base: None, bad: 0, good: 0, degraded: false } }

    pub fn assess(&mut self, snapshot: &mut Snapshot) -> bool {
        let now = Instant::now();
        let international: Vec<&Sample> = snapshot.samples.iter().filter(|sample| sample.group == "international" && sample.proto != "icmp").collect();
        for sample in &international { if sample.ok { self.seen_ok.insert(sample.name.clone(), now); } }
        let known: Vec<&&Sample> = international.iter().filter(|sample| self.seen_ok.get(&sample.name).map_or(false, |time| now.duration_since(*time) < Duration::from_secs(900))).collect();
        snapshot.loss_pct = if known.len() >= 3 { (known.iter().filter(|sample| !sample.ok).count() * 100 / known.len()) as u8 } else { 0 };

        let ping = if snapshot.via_vpn { snapshot.vpn_ping.or(snapshot.international_ping) } else { snapshot.international_ping };
        let slow = match (ping, self.base) {
            (Some(ping), Some(base)) => (ping as f32) > (base * 2.2).max(450.0) || ping > 900,
            (Some(ping), None) => ping > 900,
            _ => false,
        };
        if let (Some(ping), false) = (ping, slow) { self.base = Some(self.base.map_or(ping as f32, |base| base * 0.85 + ping as f32 * 0.15)); }

        let bad = snapshot.international_ok && (snapshot.loss_pct >= 40 || slow);
        if bad { self.bad = self.bad.saturating_add(1); self.good = 0; } else { self.good = self.good.saturating_add(1); self.bad = 0; }
        if !self.degraded && self.bad >= 3 { self.degraded = true; }
        if self.degraded && self.good >= 2 { self.degraded = false; }
        self.degraded
    }
}

pub fn decide(s: &Snapshot, degraded: bool) -> NetState {
    let p = &s.proxy;
    let vpn_active = s.tun.is_some() || (p.configured && p.listening);
    if !s.raw_known {
        if s.via_vpn {
        } else if !s.gateway_ok {
            return NetState::NoLink;
        } else {
            return NetState::VpnBroken;
        }
    } else {
        if !s.gateway_ok && !s.domestic_ok && !s.international_ok && !s.via_vpn {
            return NetState::NoLink;
        }
        if !s.domestic_ok && !s.international_ok && !s.via_vpn {
            return NetState::Offline;
        }
        if p.configured && !p.listening {
            return NetState::ProxyStale;
        }
        if !s.international_ok && !s.via_vpn {
            return NetState::National;
        }
        if vpn_active && !s.via_vpn {
            return NetState::VpnBroken;
        }
    }
    if !s.via_vpn && (s.dns.poisoned || !s.dns.system_ok) {
        return NetState::DnsIssue;
    }
    if degraded { return NetState::Degraded; }
    NetState::Online
}

pub async fn run_loop(app: AppHandle) {
    let st = app.state::<AppState>();
    let (tx, mut rx) = watch::start(app.clone());
    let mut quality = Quality::new();
    let mut cand: Option<NetState> = None;
    let mut cand_n = 0u8;
    let mut flaps: VecDeque<Instant> = VecDeque::new();
    let mut last_popup: Option<Instant> = None;
    let mut first = true;
    let mut trig = Trigger::Start;

    loop {
        let settle = trig.settle_ms();
        if settle > 0 {
            tokio::time::sleep(Duration::from_millis(settle)).await;
        }
        while let Ok(t) = rx.try_recv() {
            if t.settle_ms() > trig.settle_ms() {
                trig = t;
            }
        }

        st.checking.store(true, SeqCst);
        let spinner = tray::spin(&app);
        let mut snap = probe_all(&app).await;
        st.checking.store(false, SeqCst);
        if let Some(handle) = spinner { let _ = handle.await; }
        *st.last_check.lock() = Instant::now();
        snap.trigger = Some(serde_json::to_value(trig).unwrap().as_str().unwrap().to_string());

        let degraded = quality.assess(&mut snap);
        let raw = decide(&snap, degraded);
        let prev = st.snapshot.lock().state;

        if cand == Some(raw) {
            cand_n = cand_n.saturating_add(1);
        } else {
            cand = Some(raw);
            cand_n = 1;
        }

        let decisive = raw == NetState::NoLink
            && matches!(trig, Trigger::IfaceChange | Trigger::RouteChange);
        let mut stable = if first || cand_n >= 2 || decisive { raw } else { prev };
        first = false;

        let now = Instant::now();
        while flaps.front().map_or(false, |time| now.duration_since(*time) > Duration::from_secs(300)) { flaps.pop_front(); }
        let is_major = |state: NetState| !matches!(state, NetState::Online | NetState::Degraded);
        if stable != prev && prev != NetState::Unstable && (is_major(stable) || is_major(prev)) { flaps.push_back(now); }
        let calm = flaps.back().map_or(true, |time| now.duration_since(*time) > Duration::from_secs(120));
        if flaps.len() > 4 && !calm && !matches!(stable, NetState::NoLink | NetState::Offline) { stable = NetState::Unstable; }
        if prev == NetState::Unstable && !calm && !matches!(stable, NetState::NoLink | NetState::Offline) { stable = NetState::Unstable; }

        snap.state = stable;
        *st.snapshot.lock() = snap.clone();
        tray::update(&app, &snap);
        let _ = app.emit("snapshot", &snap);

        let settings = st.settings.read().clone();
        if stable != prev {
            history::record_event(&st, prev, stable, &snap);
            let is_major = |state: NetState| !matches!(state, NetState::Online | NetState::Degraded);
            let minor = !is_major(stable) && !is_major(prev);
            let cooldown = if minor { 180 } else { settings.popup_cooldown_secs };
            let secs = if stable == NetState::Online { 3 } else { settings.popup_seconds };
            let cooldown_ok = last_popup.map_or(true, |time| time.elapsed() > Duration::from_secs(cooldown));
            let relevant = !settings.popup_only_outages || stable.is_outage() || prev.is_outage();
            let online_ok = stable != NetState::Online || settings.toast_online;
            if settings.popup_enabled && !settings.quiet && relevant && online_ok && (cooldown_ok || stable.is_outage()) {
                popup::show(&app, &snap, secs);
                last_popup = Some(Instant::now());
            }
        }
        history::record_sample(&st, &snap);

        if trig.needs_followup() {
            let tx2 = tx.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(6)).await;
                let _ = tx2.try_send(Trigger::Followup);
            });
        }

        let fallback = if stable == NetState::Online {
            settings.interval_ok.max(15) * 4
        } else {
            settings.interval_bad.max(5) * 4
        };

        trig = tokio::select! {
            Some(t) = rx.recv() => t,
            _ = st.kick.notified() => Trigger::Manual,
            _ = tokio::time::sleep(Duration::from_secs(fallback)) => Trigger::Timer,
        };
    }
}
