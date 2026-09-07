const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
const fmt = (value) => value == null ? "-" : `${value}ms`;
const colors = {
  NO_LINK: ["#7f1d1d", "اتصال به مودم قطع است"], OFFLINE: ["#dc2626", "قطعی کامل"],
  NATIONAL: ["#eab308", "اینترنت ملی"], PROXY_STALE: ["#2563eb", "پروکسی جا مانده"],
  VPN_BROKEN: ["#9333ea", "فیلترشکن خراب"], DNS_ISSUE: ["#f97316", "مشکل DNS"],
  DEGRADED: ["#fb923c", "اختلال"], UNSTABLE: ["#fbbf24", "ناپایدار"], ONLINE: ["#22c55e", "سالم"]
};
const triggers = { start: "شروع", iface_change: "تغییر اینترفیس", route_change: "تغییر مسیر", proxy_change: "تغییر پروکسی", heartbeat_down: "ضربان: قطعی", heartbeat_up: "ضربان: وصل", heartbeat_slow: "ضربان: کندی", followup: "تأیید", manual: "دستی", timer: "زمان‌بندی" };
let targets = [];
let lastSnap = null;
const html = (value) => String(value).replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]);
document.querySelectorAll(".tg-form .chk input").forEach((checkbox) => {
  const sync = () => checkbox.parentElement.classList.toggle("on", checkbox.checked);
  checkbox.addEventListener("change", sync);
  sync();
});
async function saveTargets() {
  const settings = await invoke("get_settings");
  settings.custom_targets = targets;
  await invoke("set_settings", { settings });
  renderTargets();
}
function renderTargets() {
  const list = $("tg-list");
  if (!targets.length) { list.innerHTML = `<div class="tg-empty">تارگتی تعریف نشده — سرور شرکت یا سایت دلخواهت را اضافه کن</div>`; return; }
  const live = Object.fromEntries((lastSnap?.samples || []).filter((sample) => sample.group.startsWith("custom")).map((sample) => [sample.name, sample]));
  list.innerHTML = targets.map((target, index) => {
    const result = live[target.name];
    const led = target.enabled === false ? "" : result ? (result.ok ? "ok" : "bad") : "";
    return `<div class="tg ${target.enabled === false ? "off" : ""}" data-i="${index}"><span class="led ${led}"></span><div class="nm"><b>${html(target.name)}</b><small>${html(target.host)}${target.kind !== "http" ? `:${target.port}` : ""}</small></div><div class="tags"><span>${html(target.kind)}</span><span>${target.domestic ? "داخلی" : "خارجی"}</span>${target.affects_state ? "<span>مؤثر</span>" : ""}</div><span class="ms">${target.enabled === false ? "غیرفعال" : result ? (result.ok ? `${result.ms}ms` : "✗") : "…"}</span><div class="ops"><button class="ib" data-op="toggle" title="${target.enabled === false ? "فعال کن" : "موقتاً غیرفعال"}">${target.enabled === false ? "▶" : "⏸"}</button><button class="ib" data-op="edit" title="ویرایش">✎</button><button class="ib del" data-op="del" title="حذف">🗑</button></div></div>`;
  }).join("");
}
function render(snapshot) {
  const [color, title] = colors[snapshot.state] || ["#666", snapshot.state];
  $("dot").style.background = color;
  $("dot").style.color = color;
  $("stitle").textContent = title;
  $("smsg").textContent = `آخرین بررسی: ${new Date(snapshot.ts * 1000).toLocaleTimeString("fa-IR")} · علت: ${triggers[snapshot.trigger] || snapshot.trigger || "—"}`;
  $("pd").textContent = fmt(snapshot.domestic_ping);
  $("pi").textContent = fmt(snapshot.international_ping);
  $("loss").textContent = `${snapshot.loss_pct}%`;
  $("dns").textContent = snapshot.dns.poisoned ? "مسموم" : snapshot.dns.system_ok ? `سالم ${fmt(snapshot.dns.system_ms)}` : "خراب";
  $("prx").textContent = snapshot.tun ? `TUN: ${snapshot.tun}` : snapshot.proxy.configured ? `${snapshot.proxy.host}:${snapshot.proxy.port}` : "غیرفعال";
  $("gw").textContent = snapshot.gateway_ok ? `وصل ${fmt(snapshot.gateway_ms)}` : "قطع";
  $("targets").innerHTML = snapshot.samples.map((sample) => `<div class="t ${sample.ok ? "ok" : "bad"}"><span>${sample.name} (${sample.proto})</span><span>${sample.ok ? fmt(sample.ms) : "✗"}</span></div>`).join("");
}
async function loadSettings() {
  const settings = await invoke("get_settings");
  $("iok").value = settings.interval_ok;
  $("ibad").value = settings.interval_bad;
  $("psec").value = settings.popup_seconds;
  $("pen").checked = settings.popup_enabled;
  $("pout").checked = settings.popup_only_outages;
  $("tonline").checked = settings.toast_online;
  $("quiet").checked = settings.quiet;
  $("anchor").checked = settings.anchor_tray;
  $("trlayout").value = settings.tray_layout ?? "separate";
  const isWindows = navigator.userAgent.includes("Windows");
  if (!isWindows) {
    $("trlayout").querySelector("[data-win]")?.remove();
    $("wpill-row").hidden = true;
    if ($("trlayout").value === "widget") $("trlayout").value = "separate";
  }
  $("wpill").checked = !!settings.widget_pill;
  $("wfs").checked = !!settings.widget_hide_fullscreen;
  $("trdown").checked = settings.tray_down !== false;
  $("trup").checked = settings.tray_up !== false;
  $("trsep").classList.toggle("hidden", $("trlayout").value !== "separate");
  $("wpill-row").hidden = !isWindows || $("trlayout").value !== "widget";
  if (!navigator.platform.startsWith("Mac")) $("trlayout").querySelector("[data-mac]")?.remove();
  targets = (settings.custom_targets || []).map((target) => ({ ...target, enabled: target.enabled !== false }));
  renderTargets();
}
$("recheck").onclick = () => invoke("run_now");
$("tg-list").onclick = async (event) => {
  const button = event.target.closest("button[data-op]"); if (!button) return;
  const index = +button.closest(".tg").dataset.i, target = targets[index];
  if (button.dataset.op === "del") {
    if (!confirm(`«${target.name}» حذف شود؟`)) return;
    targets.splice(index, 1);
  } else if (button.dataset.op === "toggle") {
    target.enabled = target.enabled === false;
  } else if (button.dataset.op === "edit") {
    $("tg-name").value = target.name; $("tg-host").value = target.host; $("tg-port").value = target.port;
    $("tg-kind").value = target.kind; $("tg-dom").checked = target.domestic; $("tg-aff").checked = target.affects_state;
    targets.splice(index, 1); renderTargets(); $("tg-name").focus(); return;
  }
  await saveTargets();
};
$("preset").onchange = () => {
  const value = $("preset").value; if (!value) return;
  const [name, host, port, kind, domestic, affects] = value.split("|");
  $("tg-name").value = name; $("tg-host").value = host; $("tg-port").value = port; $("tg-kind").value = kind;
  $("tg-dom").checked = domestic === "1"; $("tg-aff").checked = affects === "1"; $("preset").value = "";
};
$("tg-form").onsubmit = async (event) => {
  event.preventDefault();
  const target = { name: $("tg-name").value.trim(), host: $("tg-host").value.trim(), port: +$("tg-port").value || 443, kind: $("tg-kind").value, domestic: $("tg-dom").checked, affects_state: $("tg-aff").checked, enabled: true };
  const error = $("tg-err"); error.textContent = "";
  if (target.kind === "http" && !/^https?:\/\//.test(target.host)) { error.textContent = "برای http هاست باید با http:// یا https:// شروع شود"; return; }
  if (target.kind !== "http" && !(target.port > 0 && target.port < 65536)) { error.textContent = "پورت نامعتبر"; return; }
  if (targets.some((item) => item.name === target.name)) { error.textContent = "نامی تکراری است"; return; }
  targets.push(target); await saveTargets(); event.target.reset(); $("tg-port").value = 443;
};
$("save").onclick = async () => {
  const settings = await invoke("get_settings");
  settings.interval_ok = +$("iok").value || 15;
  settings.interval_bad = +$("ibad").value || 5;
  settings.popup_seconds = +$("psec").value || 4;
  settings.popup_enabled = $("pen").checked;
  settings.popup_only_outages = $("pout").checked;
  settings.toast_online = $("tonline").checked;
  settings.quiet = $("quiet").checked;
  settings.anchor_tray = $("anchor").checked;
  settings.tray_layout = $("trlayout").value;
  settings.tray_down = $("trdown").checked;
  settings.tray_up = $("trup").checked;
  settings.widget_pill = $("wpill").checked;
  settings.widget_hide_fullscreen = $("wfs").checked;
  await invoke("set_settings", { settings });
  $("trsep").classList.toggle("hidden", $("trlayout").value !== "separate");
  alert("ذخیره شد");
};

$("trlayout").onchange = () => {
  $("trsep").classList.toggle("hidden", $("trlayout").value !== "separate");
  $("wpill-row").hidden = !navigator.userAgent.includes("Windows") || $("trlayout").value !== "widget";
};
listen("snapshot", (event) => { lastSnap = event.payload; render(event.payload); renderTargets(); });
invoke("get_snapshot").then(render);
loadSettings();
