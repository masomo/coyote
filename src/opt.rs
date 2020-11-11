use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Coyote")]
pub struct Opt {
    /// Http handler's serving address.
    #[structopt(short = "s", long, default_value = "127.0.0.1:3000")]
    pub http_listen: String,

    /// Unix socket to use.
    #[structopt(long, default_value = "/tmp/coyote.sock")]
    pub unix_socket: String,

    /// PHP Worker script to use.
    #[structopt(long, default_value = "worker.php")]
    pub worker_script: String,
}

impl Opt {
    pub fn args() -> Self
    where
        Self: Sized,
    {
        Opt::from_args()
    }
}
