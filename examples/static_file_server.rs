use std::sync::Arc;

use httpr::{
    http::Server,
    static_server::{
        NoBodyOnHeadResInterceptor, NotFoundRenderResInterceptor, OnlyGetReqInterceptor,
        StaticFileHandler,
    },
};

#[tokio::main]
async fn main() {
    let bind = "127.0.0.1:4444".to_string();
    let handler = StaticFileHandler::new(".", true).expect("Failed creating handler");

    let log_env = env_logger::Env::default().default_filter_or("debug");
    env_logger::init_from_env(log_env);

    Server::new(bind, handler)
        .push_req_inter(Arc::new(OnlyGetReqInterceptor))
        .push_res_inter(Arc::new(NoBodyOnHeadResInterceptor))
        .push_res_inter(Arc::new(NotFoundRenderResInterceptor))
        .run()
        .await
        .unwrap()
}
