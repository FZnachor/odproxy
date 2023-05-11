use std::{sync::{Arc, Mutex}};
use lazy_static::lazy_static;
use tokio::process::Child;

use std::collections::HashMap;
use hyper::http::HeaderValue;

use crate::conf::{CONFIG, ProxyConf};

lazy_static! {
    pub static ref HOST_MAP: HashMap<String, usize> = generate_host_map();
	pub static ref SERVICES: Arc<Mutex<Vec<ServiceData>>> = Arc::new(Mutex::new(vec![]));
}

pub struct ServiceData {
	pub child: Option<Child>,
	pub running: bool,
	pub last_active: u64
}

impl ServiceData {
	pub fn new() -> ServiceData {
		ServiceData {
			child: None,
			running: false,
			last_active: 0
		}
	}
}

pub fn get_proxy(host_index: Option<&usize>) -> Option<&ProxyConf> {
	match host_index {
		Some(i) => CONFIG.proxy.get(i.clone()),
		None => None
	}
}

pub fn get_proxy_index(host: Option<&HeaderValue>) -> Option<&usize> {
	match host {
		Some(host) => {
			let host_parts: Vec<&str> = host.to_str().unwrap().split(":").collect();
			let domain = host_parts.get(0);
			match domain {
				Some(domain) => HOST_MAP.get(&domain.to_string()),
				None => None
			}
		},
		None => None
	}
}

pub fn generate_host_map() -> HashMap<String, usize> {
	let mut hosts: Vec<(String, usize)> = vec![];
	for (i, proxy) in CONFIG.proxy.iter().enumerate() {
		for host in proxy.hosts.iter() {
			hosts.push((host.to_string(), i));
		};
	}
	HashMap::from_iter(hosts)
}