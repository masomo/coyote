use std::convert::TryInto;
use std::mem;

use anyhow::{
    anyhow,
    Result,
};
use num_traits::FromPrimitive;
use tokio::io::{
    AsyncRead,
    AsyncReadExt,
    AsyncWrite,
    AsyncWriteExt,
};

pub type Pid = usize;

#[repr(u8)]
#[derive(Debug, FromPrimitive)]
enum MessageType {
    Identity,
    Request,
    Response,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request(pub Vec<u8>);

impl From<&str> for Request {
    fn from(req: &str) -> Self {
        Self(req.as_bytes().to_vec())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response(pub Vec<u8>);

impl From<&str> for Response {
    fn from(req: &str) -> Self {
        Self(req.as_bytes().to_vec())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Identity(Pid),
    Request(Request),
    Response(Response),
}

impl Message {
    const HEADER_SIZE: usize =
        mem::size_of::<MessageType>() + mem::size_of::<usize>();

    pub async fn write_to(
        self,
        mut dst: impl AsyncWrite + Unpin,
    ) -> Result<()> {
        match self {
            Message::Identity(id) => {
                let mut buf = Vec::with_capacity(Message::HEADER_SIZE);
                buf.push(MessageType::Identity as u8);
                buf.extend(&id.to_be_bytes());
                dst.write_all(&buf).await?;
            }
            Message::Request(buf) => {
                write_u8_vec(&mut dst, MessageType::Request, buf.0).await?;
            }
            Message::Response(buf) => {
                write_u8_vec(&mut dst, MessageType::Response, buf.0).await?;
            }
        };

        dst.flush().await?;
        return Ok(());

        // NOTE: we are calling `write_all` multiple times because writes
        // are buffered and will flushed at the end.
        async fn write_u8_vec(
            mut dst: impl AsyncWrite + Unpin,
            ty: MessageType,
            buf: Vec<u8>,
        ) -> Result<()> {
            let mut header = Vec::with_capacity(Message::HEADER_SIZE);
            header.push(ty as u8);
            header.extend(&buf.len().to_be_bytes());
            dst.write_all(&header).await?;

            dst.write_all(&buf).await?;

            Ok(())
        }
    }

    pub async fn read_from(mut src: impl AsyncRead + Unpin) -> Result<Message> {
        let mut header = vec![0u8; Message::HEADER_SIZE];
        src.read_exact(&mut header).await?;

        let ty = MessageType::from_u8(
            *header
                .first()
                .ok_or_else(|| anyhow!("missing message type"))?,
        )
        .ok_or_else(|| anyhow!("unexpected message type"))?;
        let size = usize::from_be_bytes(
            header[mem::size_of::<MessageType>()..].try_into()?,
        );

        return match ty {
            MessageType::Identity => Ok(Message::Identity(size as Pid)),
            MessageType::Request => read_u8_vec(size, src)
                .await
                .map(Request)
                .map(Message::Request),
            MessageType::Response => read_u8_vec(size, src)
                .await
                .map(Response)
                .map(Message::Response),
        };

        async fn read_u8_vec(
            size: usize,
            mut src: impl AsyncRead + Unpin,
        ) -> Result<Vec<u8>> {
            let mut buf = vec![0; size];
            src.read_exact(&mut buf).await?;

            Ok(buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tokio::io::duplex;

    use super::*;

    macro_rules! message_send_receive_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[tokio::test]
            async fn $name() -> Result<()> {
                let (client, server) = duplex(64);
                $value.clone().write_to(client).await?;
                assert_eq!($value, Message::read_from(server).await?);
                Ok(())
            }
        )*
        }
    }

    message_send_receive_tests! {
        identity: Message::Identity(42),
        request: Message::Request("hello world req".into()),
        response: Message::Response("hello world res".into()),
    }
}
