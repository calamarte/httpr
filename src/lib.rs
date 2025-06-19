// #![warn(missing_docs)]

//! # Simple http implementation
//!
//! httpr provides an implementation of [http](https://en.wikipedia.org/wiki/HTTP) protocol
//! allowing to manage requests and build a responses.
//!
//! You can use [static_server] features to work as a static file server or build your own handler and
//! interceptors to manage the requests.
//!
//! # Example
//!
//! ## Setup a static file server
//!
//! See full example on [examples/static_file_server.rs](https://github.com/calamarte/httpr/blob/main/examples/static_file_server.rs)
//!
//! ```
//!   use httpr::{
//!       http::Server,
//!       static_server::{
//!           NoBodyOnHeadResInterceptor, NotFoundRenderResInterceptor, OnlyGetReqInterceptor,
//!           StaticFileHandler,
//!       },
//!   };
//!
//!   #[tokio::main]
//!   async fn main() {
//!
//!       let bind = "127.0.0.1:4444";
//!       let handler = StaticFileHandler::new(".", true).expect("Failed creating handler");
//!
//!       Server::new(bind, handler)
//!           .push_req_inter(Arc::new(OnlyGetReqInterceptor))
//!           .push_res_inter(Arc::new(NoBodyOnHeadResInterceptor))
//!           .push_res_inter(Arc::new(NotFoundRenderResInterceptor))
//!           .run()
//!           .await
//!           .unwrap()
//!   }
//!
//! ```
//!
//! ## Setup a dummy server
//!
//! See full example on [examples/dummy.rs](https://github.com/calamarte/httpr/blob/main/examples/dummy.rs)
//!
//! ```
//!   use async_trait::async_trait;
//!   use httpr::http::{HttpHandler, Named, Response, Server};
//!   use log::info;
//!
//!   struct DummyHandler {}
//!   impl Named for DummyHandler {}
//!
//!   #[async_trait]
//!   impl HttpHandler for DummyHandler {
//!       async fn solve_request(
//!           &self,
//!           request: &httpr::http::Request,
//!       ) -> Result<httpr::http::Response, &'static str> {
//!           info!("request: {request:?}");
//!
//!           let mut response = Response::new(httpr::http::HttpStatus::Ok);
//!           response.add_body(b"Hello, world :)");
//!
//!           Ok(response)
//!       }
//!   }
//!
//!   #[tokio::main]
//!   async fn main() {
//!       let bind = "127.0.0.1:4444";
//!
//!       let log_env = env_logger::Env::default().default_filter_or("debug");
//!       env_logger::init_from_env(log_env);
//!
//!       Server::new(bind.to_string(), DummyHandler {})
//!           .run()
//!           .await
//!           .unwrap();
//!   }
//! ```
//!

pub mod http;
pub mod static_server;
