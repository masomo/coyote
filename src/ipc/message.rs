
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

pub type Pid = u32;

#[repr(u8)]
#[derive(Debug, FromPrimitive)]
enum MessageType {
    Identity,
    Request,
    Response,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq)]
pub struct Response(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Identity(Pid),
    Request(Request),
    Response(Response),
}

impl Message {
    pub async fn write_to(
        self,
        dst: impl AsyncWrite,
    ) -> Result<()> {
        tokio::pin!(dst);

        match self {
            Message::Identity(id) => {
                let mut buf = Vec::with_capacity(
                    mem::size_of::<MessageType>() + mem::size_of::<Pid>(),
                );
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
            dst: impl AsyncWrite,
            ty: MessageType,
            buf: Vec<u8>,
        ) -> Result<()> {
            tokio::pin!(dst);

            let mut header = Vec::with_capacity(
                mem::size_of::<MessageType>() + mem::size_of::<usize>(),
            );
            header.push(ty as u8);
            header.extend(&buf.len().to_be_bytes());
            dst.write_all(&header).await?;

            dst.write_all(&buf).await?;

            Ok(())
        }
    }

    pub async fn read_from(src: impl AsyncRead) -> Result<Message> {
        tokio::pin!(src);

        // TODO: read full header, that contains type+length for req&res.
        let ty = MessageType::from_u8(src.read_u8().await?)
            .ok_or_else(|| anyhow!("unexpected message type"))?;

        return match ty {
            MessageType::Identity => {
                let mut buf = [0; mem::size_of::<Pid>()];
                src.read_exact(&mut buf).await?;
                Ok(Message::Identity(Pid::from_be_bytes(buf)))
            }
            MessageType::Request => {
                read_u8_vec(src).await.map(Request).map(Message::Request)
            }
            MessageType::Response => {
                read_u8_vec(src).await.map(Response).map(Message::Response)
            }
        };

        async fn read_u8_vec(src: impl AsyncRead) -> Result<Vec<u8>> {
            tokio::pin!(src);

            let mut size_buf = [0; mem::size_of::<usize>()];
            src.read_exact(&mut size_buf).await?;
            let size = usize::from_be_bytes(size_buf);

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
        request: Message::Request(Request("hello world req".as_bytes().to_vec())),
        response: Message::Response(Response("hello world res".as_bytes().to_vec())),
    }
}
