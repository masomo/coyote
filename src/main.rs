#![feature(test)]

use std::{
    convert::Infallible,
    sync::Arc,
};

use anyhow::Result;
use env_logger::Env;
use hyper::service::{
    make_service_fn,
    service_fn,
};
use hyper::{
    Body,
    Method,
    Request,
    Response,
    Server,
    StatusCode,
};
use log;
use tokio::sync::Mutex;
#[macro_use]
extern crate num_derive;
extern crate test;

mod ipc;
mod opt;
mod worker;

async fn handle(
    worker: Arc<Mutex<worker::Worker>>,
    req: Request<Body>,
) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, path) if path.starts_with("/hello/") => {
            let name = path.trim_start_matches("/hello/");

            let mut worker = worker.lock().await;
            let response =
                worker.exec(&format!(r#"{{"name":"{}"}}"#, name)).await?;

            Ok(Response::new(Body::from(response)))
        }

        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = opt::Opt::args();
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let connections = ipc::listen(&opts.unix_socket)?;
    let linker = worker::Linker::new(Box::pin(connections));
    let worker =
        worker::Worker::new(&opts.worker_script, &opts.unix_socket, linker)
            .await?;
    let worker = Arc::new(Mutex::new(worker));

    let addr = opts.http_listen.parse()?;

    let make_svc = make_service_fn(move |_| {
        let worker = worker.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle(worker.clone(), req)
            }))
        }
    });

    log::info!("Serving coyote on: {}", &addr);
    let server = Server::bind(&addr).serve(make_svc);
    server.await?;
    Ok(())
}
