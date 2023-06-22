use std::{sync::{Arc, Mutex}, vec};
use lazy_static::lazy_static;
use tokio::process::Child;

use std::collections::HashMap;
use hyper::http::HeaderValue;

use crate::conf::{ProxyConf, self};

lazy_static! {
    pub static ref HOST_MAP: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(generate_host_map()));
	pub static ref SERVICES: Arc<Mutex<HashMap<String, ServiceData>>> = Arc::new(Mutex::new(HashMap::from_iter(vec![])));
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

pub fn get_proxy(name: Option<String>) -> Option<ProxyConf> {
	let c = conf::get();
	match name {
		Some(name) => c.proxy.get(&name).cloned(),
		None => None
	}
}

pub fn get_proxy_name(host: Option<&HeaderValue>) -> Option<String> {
	match host {
		Some(host) => {
			let host_parts: Vec<&str> = host.to_str().unwrap().split(":").collect();
			let domain = host_parts.get(0);
			let host_map = HOST_MAP.lock().ok()?;
			host_map.get(&domain?.to_string()).cloned()
		},
		None => None
	}
}

pub fn generate_host_map() -> HashMap<String, String> {
	let mut hosts: Vec<(String, String)> = vec![];
	for (name, proxy) in conf::get().proxy.iter() {
		for host in proxy.hosts.iter() {
			hosts.push((host.to_string(), name.to_string()));
		}
	}
	HashMap::from_iter(hosts)
}