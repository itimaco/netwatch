(() => {
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const $ = (id) => document.getElementById(id);

  if (!CanvasRenderingContext2D.prototype.roundRect) {
    CanvasRenderingContext2D.prototype.roundRect = function (x, y, w, h, r) {
      r = Math.min(r, w / 2, h / 2); this.moveTo(x + r, y); this.arcTo(x + w, y, x + w, y + h, r); this.arcTo(x + w, y + h, x, y + h, r);
      this.arcTo(x, y + h, x, y, r); this.arcTo(x, y, x + w, y, r); this.closePath();
    };
  }
  const last = (a) => a[a.length - 1];
  const C = { ONLINE: "#22c55e", DEGRADED: "#fb923c", UNSTABLE: "#fbbf24", NATIONAL: "#eab308", OFFLINE: "#dc2626", NO_LINK: "#7f1d1d", PROXY_STALE: "#2563eb", VPN_BROKEN: "#9333ea", DNS_ISSUE: "#f97316", UNKNOWN: "#3f3f46" };
  const L = { ONLINE: "آزاد", DEGRADED: "اختلال", UNSTABLE: "ناپایدار", NATIONAL: "ملی", OFFLINE: "قطع", NO_LINK: "بدون مودم", PROXY_STALE: "پروکسی جامانده", VPN_BROKEN: "فیلترشکن خراب", DNS_ISSUE: "DNS", UNKNOWN: "خاموش/بدون داده" };
  const fa = (n) => Number(n).toLocaleString("fa-IR");
  const hh = (ts) => new Date(ts * 1000).toLocaleTimeString("fa-IR", { hour: "2-digit", minute: "2-digit" });
  const wd = (ts) => new Date(ts * 1000).toLocaleDateString("fa-IR", { weekday: "short" });
  const dur = (m) => (m >= 60 ? `${fa(Math.floor(m / 60))}س ${fa(m % 60)}د` : `${fa(m)} دقیقه`);
  const TZ = -new Date().getTimezoneOffset() * 60;

  let period = "day", data = null;
  const tip = $("ana-tip");
  const showTip = (x, y, html) => { tip.innerHTML = html; tip.classList.remove("hidden"); const r = tip.getBoundingClientRect(); tip.style.left = `${Math.min(x + 12, innerWidth - r.width - 8)}px`; tip.style.top = `${Math.max(8, y - r.height - 10)}px`; };
  const hideTip = () => tip.classList.add("hidden");

  function ctx2d(c, h) {
    const dpr = devicePixelRatio || 1, w = c.clientWidth || c.parentElement?.clientWidth || 300;
    c.width = Math.round(w * dpr); c.height = Math.round(h * dpr); c.style.height = `${h}px`;
    const ctx = c.getContext("2d"); ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    return [ctx, w, h];
  }
  function rr(ctx, x, y, w, h, r) { ctx.beginPath(); ctx.roundRect(x, y, Math.max(w, 0), Math.max(h, 0), r); }
  function empty(c, h, msg = "داده‌ای نیست") { const [ctx, W, H] = ctx2d(c, h); ctx.fillStyle = "#71717a"; ctx.font = "12px sans-serif"; ctx.textAlign = "center"; ctx.fillText(msg, W / 2, H / 2); }
  function safe(name, fn) { try { fn(); } catch (e) { console.error(`[analytics] ${name}:`, e); const c = $(name); if (c && c.tagName === "CANVAS") empty(c, 60, `خطا در ${name}`); } }

  function kpis(k, p) {
    const delta = (a, b, inv = false, unit = "") => { if (b == null || a == null) return ""; const d = Math.round((a - b) * 10) / 10; if (!d) return `<span class="d">بدون تغییر</span>`; const good = inv ? d < 0 : d > 0; return `<span class="d ${good ? "up" : "down"}">${d > 0 ? "▲" : "▼"} ${fa(Math.abs(d))}${unit}</span>`; };
    const ring = (pct, c) => { const r = 17, P = 2 * Math.PI * r; return `<svg class="ring" viewBox="0 0 44 44"><circle cx="22" cy="22" r="${r}" stroke="rgba(255,255,255,.08)" stroke-width="5" fill="none"/><circle cx="22" cy="22" r="${r}" stroke="${c}" stroke-width="5" fill="none" stroke-linecap="round" stroke-dasharray="${P}" stroke-dashoffset="${P * (1 - pct / 100)}" transform="rotate(-90 22 22)"/></svg>`; };
    const items = [["اینترنت آزاد", `${fa(k.free_pct.toFixed(1))}<small>%</small>`, C.ONLINE, delta(k.free_pct, p.free_pct, false, "%"), ring(k.free_pct, C.ONLINE)], ["قابل استفاده", `${fa(k.usable_pct.toFixed(1))}<small>%</small>`, C.DEGRADED, delta(k.usable_pct, p.usable_pct, false, "%"), ring(k.usable_pct, C.DEGRADED)], ["اینترنت ملی", dur(k.national_min), C.NATIONAL, delta(k.national_min, p.national_min, true, "د"), ""], ["قطعی کامل", dur(k.outage_min), C.OFFLINE, delta(k.outage_min, p.outage_min, true, "د"), ""], ["مشکل فیلترشکن/DNS", dur(k.vpn_issue_min), C.VPN_BROKEN, delta(k.vpn_issue_min, p.vpn_issue_min, true, "د"), ""], ["تعداد اختلال", `${fa(k.incidents)}<small>بار</small>`, C.UNSTABLE, delta(k.incidents, p.incidents, true), `<span class="d">طولانی‌ترین: ${dur(k.longest_outage_min)}</span>`], ["پینگ خارجی", k.avg_intl_ms == null ? "—" : `${fa(k.avg_intl_ms)}<small>ms</small>`, "#3b82f6", delta(k.avg_intl_ms, p.avg_intl_ms, true, "ms"), k.p95_intl_ms == null ? "" : `<span class="d">p95: ${fa(k.p95_intl_ms)}ms</span>`], ["پکت‌لاس میانگین", `${fa(k.avg_loss)}<small>%</small>`, "#ef4444", delta(k.avg_loss, p.avg_loss, true, "%"), ""]];
    $("ana-kpis").innerHTML = items.map(([t, v, c, d, extra]) => `<div class="glass kpi" style="--c:${c}"><div class="k">${t}</div><div class="v">${v}</div><div>${d} ${extra}</div></div>`).join("");
  }

  function timeline(segs, until) {
    const c = $("ana-timeline"), since = until - 12 * 3600; if (!segs.length) return empty(c, 52);
    const [ctx, W, H] = ctx2d(c, 52), PL = 22, top = 10, bh = 20, x = (t) => W - ((t - since) / (until - since)) * (W - PL);
    rr(ctx, PL, top, W - PL, bh, 7); ctx.fillStyle = "rgba(255,255,255,.05)"; ctx.fill(); ctx.save(); rr(ctx, PL, top, W - PL, bh, 7); ctx.clip();
    for (const s of segs) { if (s.end < since) continue; const a = x(Math.min(s.end, until)), b = x(Math.max(s.start, since)); ctx.fillStyle = C[s.state] || C.UNKNOWN; ctx.fillRect(a, top, Math.max(b - a, 0.8), bh); }
    ctx.restore(); ctx.font = "10px sans-serif"; ctx.textAlign = "center";
    for (let t = Math.ceil((since + TZ) / 7200) * 7200 - TZ; t <= until; t += 7200) { ctx.fillStyle = "rgba(255,255,255,.14)"; ctx.fillRect(x(t), top + bh, 1, 4); ctx.fillStyle = "#71717a"; ctx.fillText(hh(t), x(t), H - 4); }
    const nx = x(until); ctx.strokeStyle = "#fff"; ctx.lineWidth = 1.5; ctx.setLineDash([3, 2]); ctx.beginPath(); ctx.moveTo(nx, 2); ctx.lineTo(nx, top + bh + 4); ctx.stroke(); ctx.setLineDash([]); ctx.beginPath(); ctx.arc(nx, 5, 3.2, 0, Math.PI * 2); ctx.fillStyle = "#fff"; ctx.shadowColor = "#fff"; ctx.shadowBlur = 8; ctx.fill(); ctx.shadowBlur = 0; ctx.fillStyle = "#e4e4e7"; ctx.font = "bold 9.5px sans-serif"; ctx.textAlign = "left"; ctx.fillText("اکنون", nx + 5, H - 4);
    $("ana-legend").innerHTML = [...new Set(segs.map((s) => s.state))].map((s) => `<span><i style="background:${C[s]}"></i>${L[s] || s}</span>`).join("");
    c.onmousemove = (e) => { const r = c.getBoundingClientRect(), t = since + ((W - (e.clientX - r.left)) / (W - PL)) * (until - since), s = segs.find((g) => t >= g.start && t <= g.end); if (!s || t < since) return hideTip(); showTip(e.clientX, e.clientY, `<b style="color:${C[s.state]}">${L[s.state]}</b><br>${hh(Math.max(s.start, since))} → ${hh(s.end)} · ${dur(Math.round((s.end - Math.max(s.start, since)) / 60))}`); }; c.onmouseleave = hideTip;
  }

  function heat(buckets) {
    const rows = period === "week" ? 7 : 1, c = $("ana-heat"); if (!buckets.length) return empty(c, 60);
    const w0 = c.clientWidth || c.parentElement?.clientWidth || 300, gap = 3, cell = Math.max(8, Math.floor((w0 - 34) / 24) - gap), TOP = 14, H = TOP + rows * (cell + gap) + 2;
    const [ctx, W] = ctx2d(c, H), x0 = W - 24 * (cell + gap); ctx.font = "10px sans-serif"; ctx.textAlign = "center";
    for (let h = 0; h < 24; h += 4) { const cx = W - h * (cell + gap) - gap / 2; ctx.fillStyle = "#9ca3af"; ctx.fillText(fa(h), cx, 10); ctx.fillStyle = "rgba(255,255,255,.12)"; ctx.fillRect(cx, TOP - 3, 1, 3); }
    ctx.fillStyle = "#52525b"; ctx.textAlign = "left"; ctx.fillText(fa(24), x0 - gap / 2, 10);
    buckets.forEach((b, i) => { const r = Math.floor(i / 24), h = i % 24, x = W - (h + 1) * (cell + gap), y = TOP + r * (cell + gap), col = b.known ? C[b.dominant] || C.UNKNOWN : C.UNKNOWN; ctx.globalAlpha = !b.known ? 0.25 : b.dominant === "ONLINE" ? 0.35 + 0.5 * (b.free_pct / 100) : 0.55 + 0.45 * (1 - b.free_pct / 100); rr(ctx, x, y, cell, cell, 3); ctx.fillStyle = col; ctx.fill(); ctx.globalAlpha = 1; if (rows > 1 && h === 0) { ctx.fillStyle = "#71717a"; ctx.font = "10px sans-serif"; ctx.textAlign = "right"; ctx.fillText(wd(b.ts), x0 - 6, y + cell - 2); } });
    c.onmousemove = (e) => { const r = c.getBoundingClientRect(), h = Math.floor((W - (e.clientX - r.left)) / (cell + gap)), row = Math.floor((e.clientY - r.top - TOP) / (cell + gap)), b = buckets[row * 24 + h]; if (!b || h < 0 || h > 23 || row < 0) return hideTip(); showTip(e.clientX, e.clientY, `${wd(b.ts)} ${hh(b.ts)} – ${hh(b.ts + 3600)}<br>${b.known ? `<b style="color:${C[b.dominant]}">${L[b.dominant]}</b> · آزاد ${fa(b.free_pct)}%` : L.UNKNOWN}${b.intl_ms != null ? `<br>پینگ ${fa(b.intl_ms)}ms · لاس ${fa(b.loss)}%` : ""}`); }; c.onmouseleave = hideTip;
  }

  function ping(series) {
    const c = $("ana-ping"); if (!series.length) return empty(c, 170);
    const [ctx, W, H] = ctx2d(c, 170), PL = 6, PR = 40, AX = H - 26, max = Math.max(150, ...series.map((s) => Math.max(s[1] || 0, s[2] || 0))) * 1.1, t0 = series[0][0], t1 = Math.max(last(series)[0], t0 + 60), span = t1 - t0;
    const x = (t) => W - PR - ((t - t0) / span) * (W - PR - PL), y = (v) => AX - (Math.min(v, max) / max) * (AX - 10); ctx.font = "9.5px sans-serif"; ctx.textAlign = "left";
    [0.25, 0.5, 0.75, 1].forEach((f) => { ctx.strokeStyle = "rgba(255,255,255,.06)"; ctx.beginPath(); ctx.moveTo(PL, y(max * f)); ctx.lineTo(W - PR, y(max * f)); ctx.stroke(); ctx.fillStyle = "#71717a"; ctx.fillText(fa(Math.round(max * f)), W - PR + 5, y(max * f) + 3); }); ctx.fillStyle = "#52525b"; ctx.fillText("ms", W - PR + 5, 10);
    let step = period === "week" ? 86400 : span > 12 * 3600 ? 7200 : span > 4 * 3600 ? 3600 : span > 3600 ? 1800 : 600; const minPx = period === "week" ? 60 : 46; for (let i = 0; i < 12 && (W - PR - PL) / (span / step) < minPx; i++) step *= 2;
    ctx.strokeStyle = "rgba(255,255,255,.18)"; ctx.beginPath(); ctx.moveTo(PL, AX + 0.5); ctx.lineTo(W - PR, AX + 0.5); ctx.stroke(); ctx.textAlign = "center"; ctx.font = "10px sans-serif";
    for (let t = Math.ceil((t0 + TZ) / step) * step - TZ; t <= t1; t += step) { const px = x(t); ctx.strokeStyle = "rgba(255,255,255,.05)"; ctx.beginPath(); ctx.moveTo(px, 10); ctx.lineTo(px, AX); ctx.stroke(); ctx.fillStyle = "rgba(255,255,255,.25)"; ctx.fillRect(px, AX, 1, 4); ctx.fillStyle = "#9ca3af"; ctx.fillText(period === "week" ? wd(t) : hh(t), px, H - 8); }
    ctx.fillStyle = "#52525b"; ctx.textAlign = "right"; ctx.font = "9.5px sans-serif"; ctx.fillText(period === "week" ? "روز" : "ساعت", W - PR, H - 8); series.forEach((s) => { if (s[3] > 0) { ctx.fillStyle = `rgba(239,68,68,${0.15 + 0.6 * (s[3] / 100)})`; ctx.fillRect(x(s[0]) - 1, AX - Math.max(2, (s[3] / 100) * 40), 2, Math.max(2, (s[3] / 100) * 40)); } });
    const line = (idx, col, area) => { const pts = series.filter((s) => s[idx] != null); if (!pts.length) return; if (area) { const g = ctx.createLinearGradient(0, 0, 0, AX); g.addColorStop(0, col + "55"); g.addColorStop(1, col + "00"); ctx.beginPath(); ctx.moveTo(x(pts[0][0]), AX); pts.forEach((s) => ctx.lineTo(x(s[0]), y(s[idx]))); ctx.lineTo(x(last(pts)[0]), AX); ctx.closePath(); ctx.fillStyle = g; ctx.fill(); } ctx.beginPath(); ctx.strokeStyle = col; ctx.lineWidth = area ? 1.8 : 1.2; ctx.lineJoin = "round"; pts.forEach((s, i) => (i ? ctx.lineTo(x(s[0]), y(s[idx])) : ctx.moveTo(x(s[0]), y(s[idx])))); ctx.stroke(); };
    line(2, "#3b82f6", true); line(1, "#22c55e", false); const nx = x(t1); ctx.strokeStyle = "rgba(255,255,255,.5)"; ctx.setLineDash([3, 2]); ctx.beginPath(); ctx.moveTo(nx, 10); ctx.lineTo(nx, AX); ctx.stroke(); ctx.setLineDash([]);
    c.onmousemove = (e) => { const r = c.getBoundingClientRect(), t = t0 + ((W - PR - (e.clientX - r.left)) / (W - PR - PL)) * span; let best = series[0]; for (const s of series) if (Math.abs(s[0] - t) < Math.abs(best[0] - t)) best = s; showTip(e.clientX, e.clientY, `${period === "week" ? wd(best[0]) + " " : ""}${hh(best[0])}<br><span style="color:#3b82f6">خارجی ${best[2] == null ? "—" : fa(best[2]) + "ms"}</span> · <span style="color:#22c55e">داخلی ${best[1] == null ? "—" : fa(best[1]) + "ms"}</span><br>پکت‌لاس ${fa(best[3])}%`); }; c.onmouseleave = hideTip;
  }

  function days(rows) {
    const c = $("ana-days"); if (!rows.length) return empty(c, 160); const [ctx, W, H] = ctx2d(c, 160), bw = Math.min(46, (W - 20) / rows.length - 10), base = H - 24;
    rows.forEach((d, i) => { const k = d.kpis, x = W - (i + 1) * (bw + 10), parts = [[k.free_pct, C.ONLINE], [k.usable_pct - k.free_pct, C.DEGRADED], [Math.max(0, 100 - k.usable_pct), C.NATIONAL]]; let y = base; rr(ctx, x, 8, bw, base - 8, 8); ctx.fillStyle = "rgba(255,255,255,.05)"; ctx.fill(); ctx.save(); rr(ctx, x, 8, bw, base - 8, 8); ctx.clip(); parts.forEach(([p, col]) => { const h = ((base - 8) * p) / 100; ctx.fillStyle = col; ctx.fillRect(x, y - h, bw, h); y -= h; }); ctx.restore(); ctx.fillStyle = "#e4e4e7"; ctx.font = "bold 10px sans-serif"; ctx.textAlign = "center"; ctx.fillText(`${fa(Math.round(k.free_pct))}%`, x + bw / 2, Math.max(18, base - (base - 8) * (k.free_pct / 100) - 4)); ctx.fillStyle = "#71717a"; ctx.font = "10px sans-serif"; ctx.fillText(wd(d.ts), x + bw / 2, H - 6); });
    c.onmousemove = (e) => { const r = c.getBoundingClientRect(), i = Math.floor((W - (e.clientX - r.left)) / (bw + 10)), d = rows[i]; if (!d) return hideTip(); const k = d.kpis; showTip(e.clientX, e.clientY, `<b>${new Date(d.ts * 1000).toLocaleDateString("fa-IR")}</b><br>آزاد ${fa(k.free_pct.toFixed(1))}% · ملی ${dur(k.national_min)}<br>قطعی ${dur(k.outage_min)} · اختلال ${fa(k.incidents)} بار${k.avg_intl_ms != null ? `<br>پینگ ${fa(k.avg_intl_ms)}ms` : ""}`); }; c.onmouseleave = hideTip;
  }
  function worst(list) { $("ana-worst").innerHTML = list.length ? list.map(([h, m]) => `<div class="w"><b>${fa(h)}:۰۰ – ${fa((h + 1) % 24)}:۰۰</b><span>${dur(m)} اختلال</span></div>`).join("") : `<div class="empty">این هفته اختلال قابل‌توجهی ثبت نشده 🎉</div>`; }

  function render(d) {
    data = d; const f = (ts) => new Date(ts * 1000).toLocaleString("fa-IR", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" }); $("ana-range").textContent = `${f(d.since)} تا ${f(d.until)}${d.kpis.unknown_min ? ` · ${dur(d.kpis.unknown_min)} بدون داده` : ""}`;
    const s12 = Array.isArray(d.segments_12h) && d.segments_12h.length ? d.segments_12h : (d.segments || []).filter((s) => s.end > d.until - 43200);
    safe("ana-kpis", () => kpis(d.kpis, d.prev || {})); safe("ana-timeline", () => timeline(s12, d.until)); safe("ana-heat", () => heat(d.buckets || [])); safe("ana-ping", () => ping(d.series || [])); $("ana-week").classList.toggle("hidden", period !== "week"); if (period === "week") { safe("ana-days", () => days(d.days || [])); safe("ana-worst", () => worst(d.worst_hours || [])); }
  }
  let lastLoad = 0;
  async function load(force) { if (!force && Date.now() - lastLoad < 60_000) return; lastLoad = Date.now(); try { render(await invoke("get_analytics", { period })); } catch (e) { console.error("[analytics] load:", e); } }
  document.querySelectorAll(".seg button[data-p]").forEach((b) => (b.onclick = () => { document.querySelectorAll(".seg button[data-p]").forEach((x) => x.classList.toggle("on", x === b)); period = b.dataset.p; load(true); }));
  $("ana-csv").onclick = () => { if (!data) return; const rows = [["start", "end", "state", "minutes"], ...data.segments.map((s) => [new Date(s.start * 1000).toISOString(), new Date(s.end * 1000).toISOString(), s.state, ((s.end - s.start) / 60).toFixed(1)])]; const blob = new Blob(["\ufeff" + rows.map((r) => r.join(",")).join("\n")], { type: "text/csv" }); const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = `netwatch-${period}-${Date.now()}.csv`; a.click(); };
  listen("snapshot", () => load(false)); let rt; addEventListener("resize", () => { clearTimeout(rt); rt = setTimeout(() => data && render(data), 150); }); load(true);
})();