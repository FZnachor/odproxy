use std::{alloc::System, str::FromStr};

#[global_allocator]
static A: System = System;

mod configuration;

use std::net::SocketAddr;
use hyperlocal::{UnixClientExt};
use tower::make::Shared;

use hyper::{service::service_fn, Body, Client, Request, Response, Server};

async fn log(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {

	let host = req.headers().get("host");
	let p = configuration::get_host(host);
	match p {
		Some(p) => {

			// Create new Request
			let mut request_builder = Request::builder().method(req.method());
			let path = req.uri().path_and_query().unwrap().as_str();

			let is_socket = p.socket.unwrap_or(false);

			if is_socket {
				request_builder = request_builder.uri(hyperlocal::Uri::new("./www.sock", path));
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
			return Ok(Response::new(Body::empty()));
		}
	}

}

#[tokio::main]
async fn main() {
    let make_service = Shared::new(service_fn(log));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        println!("error: {}", e);
    }
}
