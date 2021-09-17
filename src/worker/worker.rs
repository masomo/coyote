use std::sync::Arc;
use std::time::Duration;

use anyhow::{
    anyhow,
    Result,
};
use tokio::process::{
    Child,
    Command,
};
use tokio::time::timeout;

use super::ipc::{
    Connection,
    Request,
    Response,
};
use crate::worker::Linker;

pub struct Worker {
    _child: Child,
    conn:   Connection,
}

impl Worker {
    pub async fn new(
        script: &str,
        socket: &str,
        linker: Arc<Linker>,
    ) -> Result<Self> {
        let child = Command::new("php")
            .arg(script)
            .arg(socket)
            .kill_on_drop(true)
            .spawn()?;

        let pid = child
            .id()
            .ok_or_else(|| anyhow!("could not get pid of worker"))?;

        let conn =
            timeout(Duration::from_millis(2000), linker.get(pid as usize))
                .await??;

        Ok(Self {
            _child: child,
            conn,
        })
    }

    pub async fn exec(
        &mut self,
        req: Request,
    ) -> Result<Response> {
        self.conn.round_trip(req).await
    }
}

#[cfg(test)]
mod tests {
    use test::Bencher;
    use tokio::runtime::Runtime;

    use super::*;
    use crate::worker::{
        ipc::listen,
        Linker,
    };

    #[tokio::test]
    async fn communicating_with_worker() -> Result<()> {
        let socket = "/tmp/coyote.test.sock.4";
        let script = "./src/worker/test_data/echo_worker.php";
        let connections = listen(socket)?;
        let linker = Linker::new(connections);

        let mut worker = Worker::new(script, socket, linker).await?;

        assert_eq!(
            worker.exec(r#"{"message":"hello world"}"#.into()).await?,
            r#"{"message":"hello world"}"#.into(),
        );

        Ok(())
    }

    #[bench]
    fn bench_communicating_with_worker(b: &mut Bencher) -> Result<()> {
        let rt = Runtime::new().unwrap();
        let _guard = rt.enter();

        let socket = "/tmp/coyote.test.sock.5";
        let script = "./src/worker/test_data/echo_worker.php";
        let connections = listen(socket)?;
        let linker = Linker::new(connections);

        let mut worker = rt.block_on(Worker::new(script, socket, linker))?;

        b.iter(|| {
            assert_eq!(
                rt.block_on(worker.exec(r#"{"message":"hello world"}"#.into()))
                    .unwrap(),
                r#"{"message":"hello world"}"#.into(),
            );
        });

        Ok(())
    }
}
