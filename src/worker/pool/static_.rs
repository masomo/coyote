use anyhow::{
    anyhow,
    bail,
    Result,
};
use async_trait::async_trait;
use futures::future::join_all;
use log::error;
use tokio::sync::{
    mpsc,
    Mutex,
};

use super::Pool;
use crate::ipc::listen;
use crate::worker::{
    Linker,
    Worker,
};

pub struct Static {
    worker_tx: mpsc::Sender<Worker>,
    worker_rx: Mutex<mpsc::Receiver<Worker>>,
}

impl Static {
    pub async fn new(
        socket: &str,
        worker_script: &str,
        size: usize,
    ) -> Result<Self> {
        let connections = listen(socket)?;
        let linker = Linker::new(Box::pin(connections));

        let workers = join_all(
            (0..size)
                .map(|_| Worker::new(worker_script, socket, linker.clone())),
        )
        .await;

        let workers = workers
            .into_iter()
            .filter_map(|w| w.ok())
            .collect::<Vec<_>>();
        if workers.len() != size {
            // TODO: return worker errors.
            bail!("could not start all workers");
        }

        let (worker_tx, worker_rx) = mpsc::channel(size);
        for worker in workers {
            worker_tx.send(worker).await.map_err(|err| {
                anyhow!("could not send worker to worker ch: {}", err)
            })?;
        }

        Ok(Self {
            worker_tx,
            worker_rx: Mutex::new(worker_rx),
        })
    }
}

#[async_trait]
impl Pool for Static {
    async fn exec(
        &self,
        req: String,
    ) -> Result<String> {
        // TODO: add timeout.
        // TODO: WorkerGuard?
        let mut worker = {
            let mut rx = self.worker_rx.lock().await;
            rx.recv()
                .await
                .ok_or_else(|| anyhow!("could not get free worker"))
        }?;

        let response = worker.exec(&req).await;

        // TODO: make sure this is super fast or do it in background.
        if let Err(err) = self.worker_tx.send(worker).await {
            error!("could not send worker back to worker ch: {}", err);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use test::Bencher;
    use tokio::runtime::Runtime;

    use super::*;

    #[tokio::test]
    async fn static_pool() -> Result<()> {
        let pool = Static::new(
            "/tmp/coyote.test.sock",
            "./src/worker/test_data/sleepy_pid_worker.php",
            2,
        )
        .await?;

        let (res1, res2) = tokio::join!(
            pool.exec(r#"{"message":"hello world"}"#.to_string()),
            pool.exec(r#"{"message":"hello world"}"#.to_string()),
        );
        let (res1, res2) = (res1.unwrap(), res2.unwrap());

        assert_ne!(res1, res2);

        Ok(())
    }

    #[bench]
    // TODO: parallel benchmark.
    fn bench_static_pool(b: &mut Bencher) -> Result<()> {
        let rt = Runtime::new().unwrap();
        let _guard = rt.enter();

        let worker = rt.block_on(Static::new(
            "/tmp/coyote.test.sock.1",
            "./src/worker/test_data/echo_worker.php",
            2,
        ))?;

        b.iter(|| {
            assert_eq!(
                rt.block_on(
                    worker.exec(r#"{"message":"hello world"}"#.to_string())
                )
                .unwrap(),
                r#"{"message":"hello world"}"#.to_string(),
            );
        });

        Ok(())
    }
}
