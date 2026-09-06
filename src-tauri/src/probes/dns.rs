use super::net::T;
use crate::state::DnsInfo;
use hickory_resolver::config::{NameServerConfigGroup,ResolverConfig,ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use std::net::{IpAddr,Ipv4Addr};
use std::time::Instant;
fn opts()->ResolverOpts{let mut o=ResolverOpts::default();o.timeout=T;o.attempts=1;o.cache_size=0;o.use_hosts_file=false;o}
pub async fn query(server:IpAddr,domain:&str)->Option<(u32,Vec<IpAddr>)>{let cfg=ResolverConfig::from_parts(None,vec![],NameServerConfigGroup::from_ips_clear(&[server],53,true));let r=TokioAsyncResolver::tokio(cfg,opts());let t=Instant::now();let ips=r.lookup_ip(domain).await.ok()?.iter().collect();Some((t.elapsed().as_millis() as u32,ips))}
pub async fn system(domain:&str)->Option<(u32,Vec<IpAddr>)>{let r=TokioAsyncResolver::tokio_from_system_conf().ok()?;let t=Instant::now();let ips=r.lookup_ip(domain).await.ok()?.iter().collect();Some((t.elapsed().as_millis() as u32,ips))}
pub fn is_poisoned(ip:&IpAddr)->bool{match ip{IpAddr::V4(v4)=>{let o=v4.octets();(o[0]==10&&o[1]==10&&o[2]==34)||*v4==Ipv4Addr::new(0,0,0,0)||*v4==Ipv4Addr::new(127,0,0,1)},_=>false}}
pub async fn check()->DnsInfo{let(sys,google,shecan)=tokio::join!(system("google.com."),query("8.8.8.8".parse().unwrap(),"google.com."),query("178.22.122.100".parse().unwrap(),"google.com."));let mut i=DnsInfo::default();if let Some((ms,ips))=&sys{i.system_ms=Some(*ms);i.poisoned=ips.iter().any(is_poisoned);i.system_ok=!ips.is_empty()&&!i.poisoned;i.resolved=ips.iter().map(|x|x.to_string()).collect()}if let Some((ms,ips))=&google{i.google_ms=Some(*ms);i.google_ok=!ips.is_empty()}i.shecan_ok=shecan.map(|(_,ips)|!ips.is_empty()).unwrap_or(false);i}
