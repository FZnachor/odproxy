mod conf;
mod data;

use std::{str::FromStr, process::Command, path::Path, time::Duration};
use conf::{ProxyConf, SpawnConf};
use data::{HOST_MAP, SERVICES, ServiceData};
use hyperlocal::{UnixClientExt};
use tokio::{fs, time::sleep};
use tower::make::Shared;

use hyper::{service::service_fn, Body, Client, Request, Response, Server};

use crate::conf::CONFIG;

async fn run(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {

	let host = req.headers().get("host");
	let host_index = data::get_proxy_index(host);
	let proxy = data::get_proxy(host_index);
	match proxy {
		Some(p) => {

			check_service(host_index.unwrap().clone(),p).await;

			// Create new Request
			let mut request_builder = Request::builder().method(req.method());
			let path = req.uri().path_and_query().unwrap().as_str();

			let is_socket = p.socket.unwrap_or(false);

			if is_socket {
				request_builder = request_builder.uri(hyperlocal::Uri::new(&p.target, path));
			} else {
				let url = p.target.clone() + path;
				request_builder = request_builder.uri(hyper::Uri::from_str(url.as_str()).expect("[!] Wrong url address!"));
			}

			// Copy all the headers
			for (name, value) in req.headers().iter() {
				request_builder = request_builder.header(name, value);
			}

			// Copy body
			let body = req.into_body();
			let nreq = request_builder.body(body).unwrap();

			if is_socket {
				Client::unix().request(nreq).await
			} else {
				Client::new().request(nreq).await
			}

		},
		None => {
			println!("Unknown host accessed: {:?}", host.unwrap());
			return Ok(Response::new(Body::empty()));
		}
	}

}

fn set_service_running(index: usize) {
	SERVICES.lock().unwrap().get_mut(index).unwrap().set_running(true);
}

fn is_service_running(index: usize) -> bool {
	SERVICES.lock().unwrap().get(index).unwrap().running
}

async fn check_service(index: usize, proxy: &ProxyConf) {

	match &proxy.spawn {
		Some(spawn) => {
			spawn_service(index, spawn);
			if !is_service_running(index) {
				wait_for_service(proxy).await;
				set_service_running(index);
			}
		},
		None => {}
	}

}

fn spawn_service(index: usize, spawn: &SpawnConf) -> bool {
	match SERVICES.lock() {
		Ok(mut array) => {
			if array.get(index).is_none() {
				let command = spawn.command.clone();
				let args = spawn.args.clone().unwrap_or(vec![]);
				let envs = spawn.envs.clone().unwrap_or(vec![]);
				let spawned_child = Command::new(command).args(args).envs(envs).spawn();
				match spawned_child {
					Ok(child) => {
						array.insert(index, ServiceData::new(Some(child)));
						return true;
					},
					Err(_) => println!("Error while spawning process!")
				}
			}
		},
		Err(_) => {}
	}
	return false;
}

async fn wait_for_service(proxy: &ProxyConf) {
	let path = Path::new(&proxy.target);
	while !path.exists() {
		sleep(Duration::from_millis(100)).await;
	}
}

#[tokio::main]
async fn main() {

	for proxy in CONFIG.proxy.iter() {
		if proxy.socket.unwrap_or(false) {
			let path = Path::new(&proxy.target);
			if path.exists() {
				fs::remove_file(path).await.unwrap();
			}
		}
	}

    let make_service = Shared::new(service_fn(run));

    let server = Server::bind(&CONFIG.listen).serve(make_service);

	let host_count = HOST_MAP.len();
	let service_count = CONFIG.proxy.len();
	println!("odproxy is listening on {} with {} hosts and {} services", CONFIG.listen, host_count, service_count);

    if let Err(e) = server.await {
        println!("error: {}", e);
    }
}
