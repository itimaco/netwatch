use super::net::{self,Route};use std::net::IpAddr;
pub async fn check(route:&Route,gw:Option<IpAddr>)->(bool,Option<u32>){let ip=match gw{Some(ip)=>ip,None=>match netdev::get_default_gateway(){Ok(g)=>match g.ipv4.first(){Some(value)=>IpAddr::V4(*value),None=>return(false,None)},Err(_)=>return(false,None)}};let ms=net::ping_via(route,ip,80).await;(ms.is_some(),ms)}
