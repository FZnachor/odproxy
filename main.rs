mod conf;
mod data;
mod services;

use std::{error::Error, thread, str::FromStr};
use data::HOST_MAP;
use hyperlocal::UnixClientExt;
use services::check_service;
use tower::make::Shared;
use signal_hook::{iterator::Signals, consts::SIGHUP};

use hyper::{service::service_fn, Body, Client, Request, Response, Server};

use crate::services::prepare_services;

async fn run(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {

	let host = req.headers().get("host");
	let name = data::get_proxy_name(host);
	let proxy = data::get_proxy(name.clone());

	match (name, proxy) {
		(Some(name), Some(p)) => {

			check_service(&name, &p).await;

			// Create new Request
			let mut request_builder = Request::builder().method(req.method());
			let path = req.uri().path_and_query().unwrap().as_str();

			if p.socket {
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

			if p.socket {
				Client::unix().request(nreq).await
			} else {
				Client::new().request(nreq).await
			}

		},
		_ => {
			println!("Unknown host accessed: {:?}", host.unwrap());
			return Ok(Response::new(Body::empty()));
		}
	}

}

#[tokio::main]
async fn main() {

	prepare_services().await;

	_ = register_signals();

    let make_service = Shared::new(service_fn(run));

    let server = Server::bind(&conf::get().listen).serve(make_service);

	let host_count = HOST_MAP.lock().ok().map(|m| m.len()).unwrap_or(0);
	let service_count = conf::get().proxy.len();
	println!("odproxy is listening on {} with {} hosts and {} services", conf::get().listen, host_count, service_count);

    if let Err(e) = server.await {
        println!("error: {}", e);
    }
}

fn register_signals() -> Result<(), Box<dyn Error>> {
    let mut signals = Signals::new(&[SIGHUP])?;
    thread::spawn(move || {
        signals.forever().for_each(|_| {
			conf::reload()
        });
    });
    Ok(())
}
