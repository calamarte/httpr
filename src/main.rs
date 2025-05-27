use std::{env, sync::Arc};

use httpr::{
    handlers::{DummyHandler, StaticFileHandler},
    http::run_server,
};

#[tokio::main]
async fn main() {
    let log_env = env_logger::Env::default().default_filter_or("debug");
    env_logger::init_from_env(log_env);

    const BIND: &str = "127.0.0.1:4444";
    // run_server(BIND, Arc::new(DummyHandler)).await.unwrap()

    let pwd = env::current_dir().unwrap();
    run_server(BIND, Arc::new(StaticFileHandler::new(pwd).unwrap()))
        .await
        .unwrap()
}
