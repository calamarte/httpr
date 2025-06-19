use async_trait::async_trait;
use httpr::http::{HttpHandler, Named, Response, Server};
use log::info;

struct DummyHandler {}
impl Named for DummyHandler {}

#[async_trait]
impl HttpHandler for DummyHandler {
    async fn solve_request(
        &self,
        request: &httpr::http::Request,
    ) -> Result<httpr::http::Response, &'static str> {
        info!("request: {request:?}");

        let mut response = Response::new(httpr::http::HttpStatus::Ok);
        response.add_body(b"Hello, world :)");

        Ok(response)
    }
}

#[tokio::main]
async fn main() {
    let bind = "127.0.0.1:4444";

    let log_env = env_logger::Env::default().default_filter_or("debug");
    env_logger::init_from_env(log_env);

    Server::new(bind.to_string(), DummyHandler {})
        .run()
        .await
        .unwrap();
}
