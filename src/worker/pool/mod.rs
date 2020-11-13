use anyhow::Result;
use async_trait::async_trait;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

/// A trait which provides connection-specific functionality.
#[async_trait]
pub trait ManageConnection: Send + Sync + 'static {
    /// Connection type
    type Connection: Send + 'static;

    /// Attempts to create a new connection
    async fn connect(&self) -> Result<Self::Connection>;

    /// Checks connection if the connection is still valid
    async fn check(
        &self,
        conn: &mut Self::Connection,
    ) -> Result<()>;
}

/// A builder for a connection pool.
pub struct Builder<M>
where
    M: ManageConnection,
{
    pub max_lifetime:       Option<Duration>,
    pub idle_timeout:       Option<Duration>,
    pub connection_timeout: Duration,
    pub min_idle:           Option<u32>,
    pub max_size:           u32,
    pub check_interval:     Duration,
    // NOTE: When we have to modify connection, it is going to be  type
    // M::Connection. Thus the unused field.
    _connection_customizer: PhantomData<M>,
}

impl<M> fmt::Debug for Builder<M>
where
    M: ManageConnection,
{
    fn fmt(
        &self,
        fmt: &mut fmt::Formatter,
    ) -> fmt::Result {
        fmt.debug_struct("Builder")
            .field("max_lifetime", &self.max_lifetime)
            .field("idle_timeout", &self.idle_timeout)
            .field("connection_timeout", &self.connection_timeout)
            .field("min_idle", &self.min_idle)
            .field("max_size", &self.max_size)
            .field("min_idle", &self.min_idle)
            .field("check_interval", &self.check_interval)
            .finish()
    }
}

impl<M> Default for Builder<M>
where
    M: ManageConnection,
{
    fn default() -> Builder<M> {
        Builder {
            max_lifetime:           Some(Duration::from_secs(30 * 60)),
            idle_timeout:           Some(Duration::from_secs(10 * 60)),
            connection_timeout:     Duration::from_secs(30),
            min_idle:               None,
            max_size:               10,
            check_interval:         Duration::from_secs(5),
            _connection_customizer: PhantomData,
        }
    }
}

impl<M> Builder<M>
where
    M: ManageConnection,
{
    /// Constructs a new `Builder`.
    ///
    /// Parameters are initialized with their default values.
    pub fn new() -> Builder<M> {
        Builder::default()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::{
        Context,
        Result,
    };
    use async_trait::async_trait;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::net::TcpStream;

    struct MockPool {
        addr: SocketAddr,
    }
    #[async_trait]
    impl ManageConnection for MockPool {
        type Connection = TcpStream;

        async fn connect(&self) -> Result<Self::Connection> {
            TcpStream::connect(self.addr).await.context("failed")
        }

        async fn check(
            &self,
            _conn: &mut Self::Connection,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn create_default_builder() {
        let b = Builder::<MockPool>::default();
        assert_eq!(b.max_lifetime, Some(Duration::from_secs(1800)));
        assert_eq!(b.idle_timeout, Some(Duration::from_secs(600)));
        assert_eq!(b.connection_timeout, Duration::from_secs(30));
        assert_eq!(b.min_idle, None);
        assert_eq!(b.max_size, 10);
        assert_eq!(b.check_interval, Duration::from_secs(5));
    }
}
