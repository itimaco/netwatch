use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetState {
    NoLink, Offline, National, ProxyStale, VpnBroken, DnsIssue, Degraded, Unstable,
    #[default] Online,
}

impl NetState {
    pub fn color(self) -> [u8; 4] { match self {
        Self::NoLink => [127,29,29,255], Self::Offline => [220,38,38,255], Self::National => [234,179,8,255],
        Self::ProxyStale => [37,99,235,255], Self::VpnBroken => [147,51,234,255], Self::DnsIssue => [249,115,22,255],
        Self::Degraded => [251,146,60,255], Self::Unstable => [161,98,7,255], Self::Online => [34,197,94,255],
    }}
    pub fn hex(self) -> String { let c=self.color(); format!("#{:02x}{:02x}{:02x}",c[0],c[1],c[2]) }
    pub fn emoji(self) -> &'static str { match self { Self::NoLink=>"📡",Self::Offline=>"🔴",Self::National=>"🇮🇷",Self::ProxyStale=>"🔌",Self::VpnBroken=>"🛡️",Self::DnsIssue=>"🧭",Self::Degraded=>"🐢",Self::Unstable=>"📉",Self::Online=>"🟢" } }
        pub fn title(self) -> &'static str { match self {
        Self::NoLink=>"به اینترنت وصل نیستید",
        Self::Offline=>"اینترنت قطع است",
        Self::National=>"فقط سایت‌های داخلی باز می‌شن",
        Self::ProxyStale=>"یک تنظیم قدیمی جلوی اینترنت رو گرفته",
        Self::VpnBroken=>"فیلترشکن وصله ولی کار نمی‌کنه",
        Self::DnsIssue=>"سایت‌ها باز نمی‌شن",
        Self::Degraded=>"اینترنت کنده",
        Self::Unstable=>"وضعیت شبکه ناپایداره",
        Self::Online=>"همه‌چیز روبه‌راهه",
    }}
    pub fn message(self) -> &'static str { match self {
        Self::NoLink=>"دستگاهتون به وای‌فای یا کابل شبکه وصل نیست. اتصال رو بررسی کنید.",
        Self::Offline=>"هیچ اینترنتی وصل نمی‌شه، احتمالاً مشکل از سرویس‌دهنده‌ست. مودم رو خاموش و روشن کنید.",
        Self::National=>"اینترنت بین‌الملل قطع شده و فقط سایت‌های داخلی باز می‌شوند؛ فیلترشکن هم به همین دلیل کار نمی‌کند.",
        Self::ProxyStale=>"فیلترشکن رو بسته‌اید ولی یه تنظیم قدیمی ازش هنوز فعاله و نمی‌ذاره اینترنت کار کنه.",
        Self::VpnBroken=>"خط اینترنت سالم است اما ترافیک از سرور فیلترشکن رد نمی‌شود. سرور/کانفیگ را عوض کنید.",
        Self::DnsIssue=>"اینترنت وصله ولی سایت‌ها باز نمی‌شن. سعی کنید تنظیم DNS رو عوض کنید.",
        Self::Degraded=>"وصل هستید ولی سرعت پایینه و ممکنه قطع‌ووصل بشه. کمی صبر کنید یا سرور فیلترشکن رو عوض کنید.",
        Self::Unstable=>"وضعیت شبکه مدام در حال تغییره؛ تا آروم شدن اوضاع، اعلان کمتری می‌بینید.",
        Self::Online=>"اینترنت داخلی، خارجی و DNS همه سالم و پایدارن.",
    }}
    pub fn hint(self) -> &'static str { match self {
        Self::NoLink => "وای‌فای/کابل را بررسی کنید",
        Self::Offline => "ISP قطع است؛ مودم را ریست کنید",
        Self::National => "خط بین‌الملل قطع است — مشکل از فیلترشکن نیست",
        Self::ProxyStale => "پروکسی سیستم روشن مانده",
        Self::VpnBroken => "سرور فیلترشکن جواب نمی‌دهد",
        Self::DnsIssue => "DNS را عوض کنید",
        Self::Degraded => "پینگ بالا / پکت‌لاس",
        Self::Unstable => "اتصال مدام قطع و وصل می‌شود",
        Self::Online => "همه‌چیز سالم است",
    }}
    pub fn action(self) -> Option<(&'static str, &'static str)> { match self {
        Self::ProxyStale=>Some(("disable_proxy","رفع خودکار")),
        Self::DnsIssue|Self::VpnBroken=>Some(("open_dashboard","جزئیات بیشتر")),
        _=>None
    }}
    pub fn is_outage(self)->bool { matches!(self,Self::NoLink|Self::Offline|Self::National) }
    pub fn name(self)->String { serde_json::to_value(self).unwrap().as_str().unwrap().to_string() }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sample { pub name:String, pub proto:String, pub group:String, pub ok:bool, pub ms:Option<u32> }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DnsInfo { pub system_ok:bool, pub system_ms:Option<u32>, pub google_ok:bool, pub google_ms:Option<u32>, pub shecan_ok:bool, pub poisoned:bool, pub resolved:Vec<String> }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyInfo { pub configured:bool, pub scheme:String, pub host:String, pub port:u16, pub listening:bool, pub canary_ok:bool, pub canary_ms:Option<u32>, pub exit:crate::probes::exit::Exit, pub works:Option<bool> }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot { pub state:NetState, pub ts:i64, pub trigger:Option<String>, pub gateway_ok:bool, pub gateway_ms:Option<u32>, pub samples:Vec<Sample>, pub dns:DnsInfo, pub proxy:ProxyInfo, pub tun:Option<String>, pub exit:crate::probes::exit::Exit, pub via_vpn:bool, pub canary_direct_ok:bool, pub raw_known:bool, pub vpn_ping:Option<u32>, pub note:Option<String>, pub domestic_ok:bool, pub international_ok:bool, pub international_icmp_only:bool, pub domestic_ping:Option<u32>, pub international_ping:Option<u32>, pub loss_pct:u8 }
