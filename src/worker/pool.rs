use anyhow::Result;
use async_trait::async_trait;

mod static_;

pub use static_::Static;

#[async_trait]
pub trait Pool {
    async fn exec(
        &self,
        req: String,
    ) -> Result<String>;
}
