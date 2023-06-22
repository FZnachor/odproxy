use std::{process::Stdio, time::{Duration, SystemTime, UNIX_EPOCH}, path::Path, io::Error, thread, net::SocketAddr};
use tokio::{process::{Command, Child}, time::sleep, fs};
use url::Url;
use std::net::TcpStream;
use std::net::{ToSocketAddrs};

use crate::{data::{SERVICES, ServiceData}, conf::{ProxyConf, self}};

fn target_to_address(target: &str) -> Option<SocketAddr> {
    Url::parse(target)
        .ok()
        .and_then(|url| {
            let host = url.host()?;
            let port = url.port()?;
            (host.to_string(), port).to_socket_addrs().ok().and_then(|addr| addr.last())
        })
}

fn modify_service_data<F>(name: &str, modify_fn: F)
    where F: FnOnce(&mut ServiceData)
{
    let mut hashmap = SERVICES.lock().unwrap();
    if let Some(service_data) = hashmap.get_mut(name) {
        modify_fn(service_data);
    }
}

pub async fn check_service(name: &String, proxy: &ProxyConf) {

	if proxy.spawn.is_some() {

		let mut ready = false;
		let mut running = false;
		modify_service_data(&name, |s| {
			ready = s.child.is_some();
			running = s.running;
			s.last_active = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
		});

		if !ready && proxy.socket {
			let path = Path::new(&proxy.target);
			if path.exists() {
				fs::remove_file(path).await.unwrap();
			}
		}

		if !running {
			start_service(&name, proxy);
			wait_for_service(proxy).await;
			modify_service_data(&name, |s| s.running = true);
		}

	}

}

fn start_service(name: &str, proxy: &ProxyConf) -> bool {
	let mut status = false;
	let spawn = proxy.spawn.as_ref().unwrap();
	modify_service_data(name, |s| {
		if s.child.is_some() {
			return;
		}
		let command = spawn.command.clone();
		let args = spawn.args.clone();
		let envs = spawn.envs.clone();
		let spawned_child = create_child(command, args, envs);
		match spawned_child {
			Ok(child) => {
				s.child = Some(child);
				status = true;
			},
			Err(_) => println!("Error while spawning process!")
		}
	});
	return status;
}

fn stop_service(name: &String) {
	modify_service_data(name, |s| {
		match s.child.as_mut() {
			Some(c) => {
				c.start_kill().unwrap();
			},
			None => {}
		}
		s.running = false;
		s.child = None;
	});
}

async fn wait_for_service(proxy: &ProxyConf) {
	if proxy.socket {

		let path = Path::new(&proxy.target);
		while !path.exists() {
			sleep(Duration::from_millis(100)).await;
		}

	} else {

		if let Some(address) = target_to_address(&proxy.target) {
			loop {
				sleep(Duration::from_millis(100)).await;
				match TcpStream::connect(address) {
					Ok(_) => break,
					Err(_) => {}
				}
			}
		}

	}
}

pub async fn prepare_services() {
	let mut hashmap = SERVICES.lock().unwrap();
	for proxy in conf::get().proxy.into_iter() {
		hashmap.insert(proxy.0, ServiceData::new());
	}

    let interval_duration = Duration::from_secs(10);
	thread::spawn(move || {
        loop {
			let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			for (name, proxy) in conf::get().proxy.iter() {
				match proxy.timeout {
					Some(t) => {
						{
							let hashmap = SERVICES.lock().unwrap();
							let s = hashmap.get(name).unwrap();
							if !s.running || s.last_active+t > now {continue;}
						}
						stop_service(name);
					},
					None => {}
				}
			}
            thread::sleep(interval_duration);
        }
    });
}

fn create_child(command: String, args: Vec<String>, envs: Vec<(String, String)>) -> Result<Child, Error> {
	let stdio = Stdio::piped();
	return Command::new(command).args(args).envs(envs).stdout(stdio).spawn();
}