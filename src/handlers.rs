use async_trait::async_trait;

use crate::http::{HttpHandler, HttpStatus, Request, Response};

pub struct DummyHandler;
#[async_trait]
impl HttpHandler for DummyHandler {
    async fn solve_request(&self, _: Request) -> Result<Response, &'static str> {
        let mut response = Response::new(HttpStatus::Ok);
        response.add_header(("Content-Type", "text/plain"));
        response.add_body("Everything is okay :)".as_bytes());

        Ok(response)
    }
}
