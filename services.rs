use std::{process::{Command, Stdio, Child}, time::{Duration, SystemTime, UNIX_EPOCH}, path::Path, io::Error, thread};
use tokio::{time::sleep, fs};

use crate::{data::{SERVICES, ServiceData}, conf::{ProxyConf, CONFIG}};

fn modify_service_data<F>(index: usize, modify_fn: F)
    where F: FnOnce(&mut ServiceData)
{
    let mut vec = SERVICES.lock().unwrap();
    if let Some(service_data) = vec.get_mut(index) {
        modify_fn(service_data);
    }
}

fn set_service_running(index: usize) {
    modify_service_data(index, |s| {
        s.running = true;
    });
}

fn set_service_last_active(index: usize) {
    modify_service_data(index, |s| {
		s.last_active = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    });
}

fn is_service_running(index: usize) -> bool {
    if let Some(service_data) = SERVICES.lock().unwrap().get(index) {
        service_data.running
    } else {
        false
    }
}

pub async fn check_service(index: usize, proxy: &ProxyConf) {

	if proxy.spawn.is_some() {
		if proxy.socket.unwrap_or(false) && SERVICES.lock().unwrap().get(index).unwrap().child.is_none() {
			let path = Path::new(&proxy.target);
			if path.exists() {
				fs::remove_file(path).await.unwrap();
			}
		}
		start_service(index, proxy);
		if !is_service_running(index) {
			wait_for_service(proxy).await;
			set_service_running(index);
		}
		set_service_last_active(index);
	}

}

fn start_service(index: usize, proxy: &ProxyConf) -> bool {
	let mut status = false;
	let spawn = proxy.spawn.as_ref().unwrap();
	modify_service_data(index, |s| {
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

fn stop_service(index: usize) {
	modify_service_data(index, |s| {
		match s.child.as_mut() {
			Some(c) => {
				c.kill().unwrap();
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
	for i in 0..CONFIG.proxy.len() {
		let mut vec = SERVICES.lock().unwrap();
		vec.insert(i, ServiceData::new());
	}

    let interval_duration = Duration::from_secs(10);
	thread::spawn(move || {
        loop {
			let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
			for (i, proxy) in CONFIG.proxy.iter().enumerate() {
				match proxy.timeout {
					Some(t) => {
						{
							let vec = SERVICES.lock().unwrap();
							let s = vec.get(i).unwrap();
							if !s.running || s.last_active+t > now {continue;}
						}
						stop_service(i);
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