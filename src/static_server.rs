use std::{
    collections::HashSet,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use log::{debug, warn};
use tokio::{
    fs::{read_dir, File, ReadDir},
    io::AsyncReadExt,
};

use crate::http::{
    HttpHandler, HttpStatus, InterceptorReq, InterceptorRes, Method, Named, Request, Response,
};

const ALLOWED_METHODS: [Method; 3] = [Method::Get, Method::Head, Method::Options];

pub struct StaticFileHandler {
    root: PathBuf,
}

impl StaticFileHandler {
    pub fn new<P: Into<PathBuf>>(root: P) -> Result<Self, &'static str> {
        let root: PathBuf = root.into();

        if !root.exists() {
            return Err("Path doesn't exists in the system!");
        }

        if !root.is_dir() {
            return Err("Path is not a directory!");
        }

        Ok(StaticFileHandler { root })
    }

    async fn match_file(&self, mut path: &Path) -> Option<File> {
        if let Ok(p) = path.strip_prefix("/") {
            path = p;
        }

        let file_path = self.root.join(path);
        if file_path.is_file() {
            if !file_path.exists() {
                return None;
            }

            return File::open(file_path).await.ok();
        }

        if file_path.is_dir() {
            return StaticFileHandler::find_first_file(read_dir(file_path).await.ok()?).await;
        }

        None
    }

    async fn find_first_file(mut entries: ReadDir) -> Option<File> {
        while let Some(entry) = entries.next_entry().await.ok()? {
            if entry.path().is_file() {
                return File::open(entry.path()).await.ok();
            }
        }

        None
    }
}

impl Named for StaticFileHandler {}

#[async_trait]
impl HttpHandler for StaticFileHandler {
    async fn solve_request(&self, request: &Request) -> Result<Response, &'static str> {
        let url = request.url();
        let path = Path::new(url.path());

        debug!("Reading {:?}", path);

        let mut file = match self.match_file(path).await {
            Some(f) => f,
            None => return Ok(Response::new(HttpStatus::NotFound)),
        };

        let mut body = Vec::new();

        if let Err(e) = file.read_to_end(&mut body).await {
            warn!("{e:?}");
            return Ok(Response::new(HttpStatus::InternalServerError));
        }

        let mut response = Response::new(HttpStatus::Ok);
        let mime = mime_guess::from_path(path)
            .first_or_text_plain()
            .to_string();

        response.add_header(("Content-Type", &mime));
        response.add_body(&body);

        Ok(response)
    }
}

pub struct OnlyGetReqInterceptor;

impl Named for OnlyGetReqInterceptor {}

#[async_trait]
impl InterceptorReq for OnlyGetReqInterceptor {
    async fn chain_req(&self, request: Request) -> ControlFlow<Response, Request> {
        match request.method() {
            Method::Get | Method::Head => ControlFlow::Continue(request),
            Method::Options => {
                ControlFlow::Break(Response::allowed(HashSet::from(ALLOWED_METHODS)))
            }
            _ => ControlFlow::Break(Response::new(HttpStatus::MethodNotAllowed)),
        }
    }
}

pub struct NoBodyOnHeadResInterceptor;

impl Named for NoBodyOnHeadResInterceptor {}

#[async_trait]
impl InterceptorRes for NoBodyOnHeadResInterceptor {
    async fn chain_res(&self, request: &Request, mut response: Response) -> Response {
        if request.method() == Method::Head {
            response.clean_body();
        }

        response
    }
}
