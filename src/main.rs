use std::{env, path::PathBuf, sync::Arc};

use clap::Parser;
use httpr::{
    http::Server,
    static_server::{NoBodyOnHeadResInterceptor, OnlyGetReqInterceptor, StaticFileHandler},
};

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, default_value_t = 4444)]
    port: u16,
    #[arg(short, default_value = "127.0.0.1")]
    bind: String,
    working_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let Args {
        port,
        mut bind,
        working_dir,
    } = Args::parse();

    let working_dir = match working_dir {
        Some(p) => p,
        None => env::current_dir().expect("Failed to get current directory"),
    };

    bind.push_str(&format!(":{port}"));

    let log_env = env_logger::Env::default().default_filter_or("info");
    env_logger::init_from_env(log_env);

    let handler = Arc::new(StaticFileHandler::new(working_dir).expect("Failed creating handler"));
    Server::new(bind, handler)
        .push_req_inter(Arc::new(OnlyGetReqInterceptor))
        .push_res_inter(Arc::new(NoBodyOnHeadResInterceptor))
        .run()
        .await
        .unwrap()
}
