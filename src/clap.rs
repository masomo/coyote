use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Coyote")]
pub struct Opt {
    /// Http handler's serving address.
    #[structopt(short = "s", long, default_value = "127.0.0.1:3000")]
    pub http_listen: String,
}

impl Opt {
    pub fn args() -> Self
    where
        Self: Sized,
    {
        Opt::from_args()
    }
}
