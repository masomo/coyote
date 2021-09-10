use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{
    anyhow,
    Result,
};
use tokio::sync::{
    oneshot,
    Mutex,
};
use tokio_stream::{
    Stream,
    StreamExt,
};

use super::ipc::{
    Connection,
    Pid,
};

pub struct Linker {
    queue:   Mutex<Vec<Connection>>,
    waiters: Mutex<HashMap<Pid, oneshot::Sender<Connection>>>,
}

impl Linker {
    pub fn new(
        connections: impl Stream<Item = Connection> + Send + Unpin + 'static
    ) -> Arc<Self> {
        let linker = Arc::new(Self {
            queue:   Mutex::new(vec![]),
            waiters: Mutex::new(HashMap::new()),
        });
        linker.clone().listen(connections);
        linker
    }

    fn listen(
        self: Arc<Self>,
        mut connections: impl Stream<Item = Connection> + Send + Unpin + 'static,
    ) {
        let linker = self;
        tokio::spawn(async move {
            'accept_connections: while let Some(conn) = connections.next().await
            {
                {
                    let mut waiters = linker.waiters.lock().await;
                    if let Some(waiter) = waiters.remove(&conn.pid()) {
                        match waiter.send(conn) {
                            Ok(()) => {}
                            Err(_) => {
                                log::error!("could not send conn to waiter");
                            }
                        }
                        continue 'accept_connections;
                    }
                }

                {
                    let mut queue = linker.queue.lock().await;
                    queue.push(conn);
                }
            }
        });
    }

    pub async fn get(
        self: Arc<Self>,
        pid: Pid,
    ) -> Result<Connection> {
        {
            let mut queue = self.queue.lock().await;
            if let Some(id) = queue.iter().position(|x| x.pid() == pid) {
                let conn = queue.remove(id);
                return Ok(conn);
            }
        }

        let (tx, rx) = oneshot::channel::<Connection>();
        {
            let mut waiters = self.waiters.lock().await;
            waiters.insert(pid, tx);
        }
        rx.await
            .map_err(|err| anyhow!("could not receive from waiter ch: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        net::UnixStream,
        time::{
            sleep,
            timeout,
        },
    };

    use super::*;
    use crate::worker::ipc::{
        listen,
        Message,
    };

    #[tokio::test]
    async fn waiting_connection_with_pid() -> Result<()> {
        let socket = "/tmp/coyote.test.sock.3";
        let connections = listen(socket)?;
        let linker = Linker::new(connections);

        let mut client_one = UnixStream::connect(socket).await?;
        Message::Identity(42).write_to(&mut client_one).await?;
        let conn_one =
            timeout(Duration::from_millis(1), linker.clone().get(42)).await??;
        assert_eq!(conn_one.pid(), 42);

        tokio::spawn(async move {
            sleep(Duration::from_millis(2)).await;

            let mut client_two = UnixStream::connect(socket).await.unwrap();
            Message::Identity(43)
                .write_to(&mut client_two)
                .await
                .unwrap();
        });
        let conn_two =
            timeout(Duration::from_millis(10), linker.clone().get(43))
                .await??;
        assert_eq!(conn_two.pid(), 43);

        Ok(())
    }
}
