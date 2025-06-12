use std::{env, path::PathBuf, sync::Arc};

use clap::Parser;
use httpr::{
    http::Server,
    static_server::{
        NoBodyOnHeadResInterceptor, NotFoundRenderResInterceptor, OnlyGetReqInterceptor,
        StaticFileHandler,
    },
};

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, default_value_t = 4444)]
    port: u16,
    #[arg(short, default_value = "127.0.0.1")]
    bind: String,
    #[arg(short('w'), help("Allow browse in directories"))]
    browsable: bool,
    working_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let Args {
        port,
        mut bind,
        browsable,
        working_dir,
    } = Args::parse();

    let working_dir = match working_dir {
        Some(p) => p,
        None => env::current_dir().expect("Failed to get current directory"),
    };

    bind.push_str(&format!(":{port}"));

    let log_env = env_logger::Env::default().default_filter_or("info");
    env_logger::init_from_env(log_env);

    let handler = StaticFileHandler::new(working_dir, browsable).expect("Failed creating handler");
    Server::new(bind, handler)
        .push_req_inter(Arc::new(OnlyGetReqInterceptor))
        .push_res_inter(Arc::new(NoBodyOnHeadResInterceptor))
        .push_res_inter(Arc::new(NotFoundRenderResInterceptor))
        .run()
        .await
        .unwrap()
}
