use std::time::Duration;

use anyhow::{
    bail,
    Result,
};
use tokio::net::{
    UnixListener,
    UnixStream,
};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::{
    wrappers::{
        UnboundedReceiverStream,
        UnixListenerStream,
    },
    Stream,
    StreamExt,
};

use super::message::{
    Message,
    Pid,
    Request,
    Response,
};

#[derive(Debug)]
pub struct Connection {
    pid:    Pid,
    stream: UnixStream,
}

impl Connection {
    async fn new(mut stream: UnixStream) -> Result<Self> {
        let message = timeout(
            Duration::from_millis(100),
            Message::read_from(&mut stream),
        )
        .await??;

        let pid = match message {
            Message::Identity(pid) => pid,
            _ => bail!("expected identity message got {:?}", message),
        };

        Ok(Self { pid, stream })
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    pub async fn round_trip(
        &mut self,
        req: Request,
    ) -> Result<Response> {
        Message::Request(req).write_to(&mut self.stream).await?;

        match Message::read_from(&mut self.stream).await? {
            Message::Response(response) => Ok(response),
            message => bail!("unexpected message: {:?}", message),
        }
    }
}

pub fn listen(path: &str) -> Result<impl Stream<Item = Connection> + Unpin> {
    let _ = std::fs::remove_file(path);
    let mut listener = UnixListenerStream::new(UnixListener::bind(path)?);
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Some(stream) = listener.next().await {
            match stream {
                Ok(stream) => {
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        match Connection::new(stream).await {
                            Ok(conn) => {
                                match tx.send(conn) {
                                    Ok(()) => {}
                                    Err(err) => log::error!(
                                        "could not send created connection: {}",
                                        err
                                    ),
                                };
                            }
                            Err(err) => {
                                log::error!(
                                    "could not create connection: {}",
                                    err
                                );
                            }
                        }
                    });
                }
                Err(err) => {
                    log::error!("could not accept new connection: {}", err);
                }
            }
        }
    });

    Ok(UnboundedReceiverStream::new(rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn listening_connections() -> Result<()> {
        let socket = "/tmp/coyote.test.sock.1";
        let mut connections = listen(socket)?;

        let client = UnixStream::connect(socket).await?;
        Message::Identity(42).write_to(client).await?;

        let conn = connections.next().await.unwrap();
        assert_eq!(conn.pid(), 42);

        Ok(())
    }

    #[tokio::test]
    async fn sending_and_receiving_messages() -> Result<()> {
        let socket = "/tmp/coyote.test.sock.2";
        let mut connections = listen(socket)?;

        let mut client = UnixStream::connect(socket).await?;
        Message::Identity(42).write_to(&mut client).await?;

        let mut conn = connections.next().await.unwrap();
        assert_eq!(conn.pid(), 42);

        tokio::spawn(async move {
            let req = timeout(
                Duration::from_millis(5),
                Message::read_from(&mut client),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(req, Message::Request("hello world req".into()));

            Message::Response("hello world res".into())
                .write_to(&mut client)
                .await
                .unwrap();
        });

        let response =
            conn.round_trip(Request("hello world req".into())).await?;
        assert_eq!(response, Response("hello world res".into()));

        Ok(())
    }
}
