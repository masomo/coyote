use anyhow::Result;
use async_trait::async_trait;

mod static_;

use super::ipc::{
    Request,
    Response,
};
pub use static_::Static;

#[async_trait]
pub trait Pool {
    async fn exec(
        &self,
        req: Request,
    ) -> Result<Response>;
}
