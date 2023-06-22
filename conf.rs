use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{fs::File, process::exit};
use std::io::prelude::*;
use serde::Deserialize;
use lazy_static::lazy_static;
use serde_yaml::from_str;

use crate::data::{HOST_MAP, generate_host_map};

lazy_static! {
    pub static ref CONFIG: Arc<Mutex<RootConf>> = Arc::new(Mutex::new(load()));
}

#[derive(Debug, Deserialize, Clone)]
pub struct RootConf {
	pub listen: SocketAddr,
	pub proxy: HashMap<String, ProxyConf>
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConf {
	pub hosts: Vec<String>,
	pub target: String,
    #[serde(default)]
	pub socket: bool,
	pub spawn: Option<SpawnConf>,
	pub timeout: Option<u64>
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpawnConf {
	pub command: String,
    #[serde(default)]
	pub args: Vec<String>,
    #[serde(default)]
	pub envs: Vec<(String, String)>
}

fn load() -> RootConf {
    let file = File::open("config.yml");
	if file.is_err() {
		println!("[!] Config file was not found!"); exit(-1);
	}
	let mut contents = String::new();
	if file.unwrap().read_to_string(&mut contents).is_err() {
		println!("[!] Unable to read config file!"); exit(-1);
	}
	match from_str(&contents) {
		Ok(conf) => conf,
		Err(_) => {println!("[!] Unable to parse config!"); exit(0);}
	}
}

pub fn reload() {
	let conf: RootConf = load();
	*CONFIG.lock().unwrap() = conf;
	*HOST_MAP.lock().unwrap() = generate_host_map();
}

pub fn get() -> RootConf {
    return CONFIG.lock().unwrap().clone();
}