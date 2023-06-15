use std::{process::Stdio, time::{Duration, SystemTime, UNIX_EPOCH}, path::Path, io::Error, thread};
use tokio::{process::{Command, Child}, time::sleep, fs};

use crate::{data::{SERVICES, ServiceData}, conf::{ProxyConf, self}};

fn modify_service_data<F>(name: &String, modify_fn: F)
    where F: FnOnce(&mut ServiceData)
{
    let mut hashmap = SERVICES.lock().unwrap();
    if let Some(service_data) = hashmap.get_mut(name) {
        modify_fn(service_data);
    }
}

fn set_service_running(name: &String) {
    modify_service_data(name, |s| {
        s.running = true;
    });
}

fn set_service_last_active(name: &String) {
    modify_service_data(name, |s| {
		s.last_active = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    });
}

fn is_service_running(name: &String) -> bool {
    if let Some(service_data) = SERVICES.lock().unwrap().get(name) {
        service_data.running
    } else {
        false
    }
}

pub async fn check_service(name: &String, proxy: &ProxyConf) {

	if proxy.spawn.is_some() {
		if proxy.socket.unwrap_or(false) && SERVICES.lock().unwrap().get(name).unwrap().child.is_none() {
			let path = Path::new(&proxy.target);
			if path.exists() {
				fs::remove_file(path).await.unwrap();
			}
		}
		start_service(name, &proxy);
		if !is_service_running(name) {
			wait_for_service(&proxy).await;
			set_service_running(name);
		}
		set_service_last_active(name);
	}

}

fn start_service(name: &String, proxy: &ProxyConf) -> bool {
	let mut status = false;
	let spawn = proxy.spawn.as_ref().unwrap();
	modify_service_data(name, |s| {
		if s.child.is_some() {
			return;
		}
		let command = spawn.command.clone();
		let args = spawn.args.clone().unwrap_or(vec![]);
		let envs = spawn.envs.clone().unwrap_or(vec![]);
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
	println!("Stopped");
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
	let path = Path::new(&proxy.target);
	while !path.exists() {
		sleep(Duration::from_millis(100)).await;
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