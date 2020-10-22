use std::{
    convert::Infallible,
    sync::Arc,
};

use anyhow::Result;
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
use tokio::sync::Mutex;

mod clap;
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
    let opts = clap::Opt::args();
    let worker = Arc::new(Mutex::new(worker::Worker::new()?));

    let addr = opts.http_listen.parse()?;

    let make_svc = make_service_fn(move |_| {
        let worker = worker.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle(worker.clone(), req)
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    server.await?;
    Ok(())
}
