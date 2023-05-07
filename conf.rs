use std::net::SocketAddr;
use std::{fs::File, process::exit};
use std::io::prelude::*;
use serde::Deserialize;
use lazy_static::lazy_static;
use serde_yaml::from_str;

lazy_static! {
    pub static ref CONFIG: RootConf = load_config();
}

#[derive(Debug, Deserialize)]
pub struct RootConf {
	pub listen: SocketAddr,
	pub proxy: Vec<ProxyConf>
}

#[derive(Debug, Deserialize)]
pub struct ProxyConf {
	pub hosts: Vec<String>,
	pub target: String,
	pub socket: Option<bool>,
	pub spawn: Option<SpawnConf>,
	pub timeout: Option<u64>
}

#[derive(Debug, Deserialize)]
pub struct SpawnConf {
	pub command: String,
	pub args: Option<Vec<String>>,
	pub envs: Option<Vec<(String, String)>>
}

fn load_config() -> RootConf {
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