use super::net;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Exit {
    pub ok: bool,
    pub ip: Option<String>,
    pub country: Option<String>,
    pub ms: Option<u32>,
}

impl Exit {
    pub fn is_iran(&self) -> bool { self.country.as_deref() == Some("IR") }
    pub fn is_foreign(&self) -> bool { self.ok && self.country.is_some() && !self.is_iran() }
}

const TRACE: &[&str] = &[
    "https://1.1.1.1/cdn-cgi/trace",
    "https://www.cloudflare.com/cdn-cgi/trace",
    "https://cloudflare-dns.com/cdn-cgi/trace",
];
const IPAPI: &str = "http://ip-api.com/json/?fields=status,countryCode,query";

async fn trace(c: &reqwest::Client, url: &str) -> Option<Exit> {
    let t = Instant::now();
    let body = tokio::time::timeout(Duration::from_secs(4), async {
        c.get(url).send().await.ok()?.text().await.ok()
    }).await.ok()??;
    let get = |key: &str| body.lines().find_map(|line| {
        line.strip_prefix(&format!("{key}=")).map(|value| value.trim().to_string())
    });
    let country = get("loc")?;
    Some(Exit { ok: true, ip: get("ip"), country: Some(country), ms: Some(t.elapsed().as_millis() as u32) })
}

async fn ipapi(c: &reqwest::Client) -> Option<Exit> {
    let t = Instant::now();
    let value: serde_json::Value = tokio::time::timeout(Duration::from_secs(4), async {
        c.get(IPAPI).send().await.ok()?.json().await.ok()
    }).await.ok()??;
    Some(Exit {
        ok: true,
        ip: value["query"].as_str().map(String::from),
        country: value["countryCode"].as_str().map(String::from),
        ms: Some(t.elapsed().as_millis() as u32),
    })
}

pub async fn check(c: &reqwest::Client) -> Exit {
    let mut set = tokio::task::JoinSet::new();
    for url in TRACE {
        let client = c.clone();
        set.spawn(async move { trace(&client, url).await });
    }
    let client = c.clone();
    set.spawn(async move { ipapi(&client).await });
    while let Some(result) = set.join_next().await {
        if let Ok(Some(exit)) = result {
            set.abort_all();
            return exit;
        }
    }
    Exit::default()
}

pub async fn canary(c: &reqwest::Client) -> Option<u32> {
    net::http(c, "https://www.youtube.com/generate_204", Duration::from_secs(4)).await
}