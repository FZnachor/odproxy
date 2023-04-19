use std::{fs::File, process::exit};
use std::io::prelude::*;
use toml::{de::from_str};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

use std::collections::HashMap;
use hyper::http::HeaderValue;

lazy_static! {
    pub static ref CONFIG: Root = load_config();
    pub static ref HOST_MAP: HashMap<String, usize> = generate_host_table();
}


#[derive(Debug, Deserialize, Serialize)]
pub struct Root {
	pub proxy: Vec<ProxyItem>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProxyItem {
	pub host: String,
	pub target: String,
	pub socket: Option<bool>
}

fn load_config() -> Root {
    let file = File::open("config.toml");
	if file.is_err() {
		println!("[!] Unable to read config file"); exit(-1);
	}
	let mut contents = String::new();
	if file.unwrap().read_to_string(&mut contents).is_err() {
		println!("[!] Unable to read config file"); exit(-1);
	}
	match from_str(&contents) {
		Ok(conf) => {conf},
		Err(_) => {println!("[!] Unable to parse config"); exit(0);}
	}
}

pub fn get_host(host: Option<&HeaderValue>) -> Option<&ProxyItem> {
	if host.is_some() {
		let host_parts: Vec<&str> = host.unwrap().to_str().unwrap().split(":").collect();
		let domain = host_parts.get(0);
		if domain.is_some() {
			let host_index = HOST_MAP.get(&domain.unwrap().to_string());
			if host_index.is_some() {
				let res = CONFIG.proxy.get(host_index.unwrap().clone());
				return res;
			}
		}
	}
	return None;
}

pub fn generate_host_table() -> HashMap<String, usize> {
	let mut hosts: Vec<(String, usize)> = vec![];
	for (i, proxy) in CONFIG.proxy.iter().enumerate() {
		hosts.push((proxy.host.to_string(), i));
	}
	HashMap::from_iter(hosts)
}