//! Features to build a static server

use core::str;
use std::{
    borrow::Cow,
    collections::HashSet,
    ops::ControlFlow,
    path::{Component, Path, PathBuf},
};

use async_trait::async_trait;
use handlebars::{Assets, DIRECTORY_TEMPLATE, HBS, NOT_FOUND_TEMPLATE};
use log::{debug, warn};
use serde::Serialize;
use tokio::{
    fs::{read_dir, File},
    io::AsyncReadExt,
};
use utils::{mime_by_ext, mime_by_path};

use crate::http::{
    HttpHandler, HttpStatus, InterceptorReq, InterceptorRes, Method, Named, Request, Response,
};

mod handlebars;
mod utils;

enum FileMatch {
    File(File),
    Redirect(PathBuf),
    NotFound,
}

const ALLOWED_METHODS: [Method; 3] = [Method::Get, Method::Head, Method::Options];

const INDEX_FILE_NAME: &str = "index.html";
pub(in crate::static_server) const INTERNAL_ROOT: &str = "/__internal/";

#[derive(Serialize)]
struct TemplateDirCtx<'a> {
    is_root: bool,
    dir: Cow<'a, str>,
    bread_crums: Vec<TemplateBreadCrumCtx>,
    files: Vec<TemplateEntryCtx<'a>>,
}

#[derive(Eq, PartialEq, Serialize)]
struct TemplateEntryCtx<'a> {
    is_dir: bool,
    file_name: Cow<'a, str>,
    mime: Option<String>,
}

#[derive(Serialize)]
struct TemplateBreadCrumCtx {
    name: String,
    path: PathBuf,
}

impl<'a> Ord for TemplateEntryCtx<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .is_dir
            .cmp(&self.is_dir)
            .then(self.file_name.cmp(&other.file_name))
    }
}

impl<'a> PartialOrd for TemplateEntryCtx<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct StaticFileHandler {
    root: PathBuf,
    is_browsable: bool,
}

impl StaticFileHandler {
    pub fn new<P: Into<PathBuf>>(root: P, browsable: bool) -> Result<Self, &'static str> {
        let root: PathBuf = root.into();

        if !root.exists() {
            return Err("Path doesn't exists in the system!");
        }

        if !root.is_dir() {
            return Err("Path is not a directory!");
        }

        Ok(StaticFileHandler {
            root,
            is_browsable: browsable,
        })
    }

    async fn match_file(&self, mut path: &Path) -> FileMatch {
        let request_path = path;

        if let Ok(p) = path.strip_prefix("/") {
            path = p;
        }

        let file_path = self.root.join(path);
        if !file_path.exists() {
            return FileMatch::NotFound;
        }

        if file_path.is_dir() {
            let mut request_path = request_path.to_path_buf();
            request_path.push(INDEX_FILE_NAME);

            return FileMatch::Redirect(request_path);
        }

        FileMatch::File(File::open(&file_path).await.expect("File access"))
    }

    fn generate_bread_crum(path: &Path) -> Vec<TemplateBreadCrumCtx> {
        let mut bread_crums = Vec::new();
        let mut current_path = PathBuf::new();
        for component in path.components() {
            current_path.push(component);

            match component {
                Component::RootDir => {
                    bread_crums.push(TemplateBreadCrumCtx {
                        name: String::from("/"),
                        path: current_path.clone(),
                    });
                }
                Component::Normal(n) => {
                    bread_crums.push(TemplateBreadCrumCtx {
                        name: n.to_string_lossy().to_string(),
                        path: current_path.clone(),
                    });
                }
                _ => unreachable!("Never here!"),
            }
        }

        bread_crums
    }

    async fn solve_file_request(&self, request: &Request) -> Result<Response, &'static str> {
        let url = request.url();
        let path = Path::new(url.path());

        debug!("Reading {:?}", path);

        let mut file = match self.match_file(path).await {
            FileMatch::File(f) => f,
            FileMatch::Redirect(p) => return Ok(Response::redirect(p)),
            FileMatch::NotFound => return Ok(Response::not_found()),
        };

        let mut body = Vec::new();

        if let Err(e) = file.read_to_end(&mut body).await {
            warn!("{e:?}");
            return Ok(Response::new(HttpStatus::InternalServerError));
        }

        let mut response = Response::new(HttpStatus::Ok);

        response.add_header(("Content-Type", &mime_by_path(path)));
        response.add_body(&body);

        Ok(response)
    }

    async fn solve_browsable_request(&self, request: &Request) -> Result<Response, &'static str> {
        let url = request.url();
        let request_path = Path::new(url.path());

        // Internal access
        if request_path.starts_with(INTERNAL_ROOT) {
            let internal_path = request_path.strip_prefix(INTERNAL_ROOT).unwrap();

            if let Some(asset) = Assets::get(internal_path.to_str().unwrap()) {
                let ext = internal_path
                    .extension()
                    .map(|ext| ext.to_str().unwrap())
                    .unwrap();

                let mut response = Response::new(HttpStatus::Ok);
                response.add_header(("Content-Type", &mime_by_ext(ext)));
                response.add_body(&asset.data);

                return Ok(response);
            }

            return Ok(Response::not_found());
        }

        let path = if let Ok(p) = request_path.strip_prefix("/") {
            p
        } else {
            request_path
        };

        let absolute_path = self.root.join(path);
        if !absolute_path.exists() {
            return Ok(Response::not_found());
        }

        if absolute_path.is_file() {
            return self.solve_file_request(request).await;
        }

        if !request_path.to_string_lossy().ends_with("/") {
            let location = format!("{}/", request_path.display());
            return Ok(Response::redirect(location));
        }

        let mut dir_reading = read_dir(absolute_path).await.unwrap();
        let mut files = Vec::new();
        while let Some(entry) = dir_reading.next_entry().await.unwrap() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().await.unwrap().is_dir();

            let mime = entry
                .path()
                .extension()
                .and_then(|v| v.to_str())
                .map(mime_by_ext);

            let file = TemplateEntryCtx {
                is_dir,
                file_name: Cow::Owned(file_name),
                mime,
            };

            files.push(file);
        }

        files.sort();

        let context = TemplateDirCtx {
            is_root: request_path.to_str().unwrap().trim() == "/",
            bread_crums: StaticFileHandler::generate_bread_crum(request_path),
            dir: Cow::Borrowed(request_path.to_str().unwrap()),
            files,
        };

        let body = HBS.render(DIRECTORY_TEMPLATE, &context).unwrap();

        let mut response = Response::new(HttpStatus::Ok);
        response.add_header(("Content-Type", "text/html; charset=utf-8"));
        response.add_body(body.as_bytes());

        Ok(response)
    }
}

impl Named for StaticFileHandler {}

#[async_trait]
impl HttpHandler for StaticFileHandler {
    async fn solve_request(&self, request: &Request) -> Result<Response, &'static str> {
        if self.is_browsable {
            self.solve_browsable_request(request).await
        } else {
            self.solve_file_request(request).await
        }
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

pub struct NotFoundRenderResInterceptor;

impl Named for NotFoundRenderResInterceptor {}

#[async_trait]
impl InterceptorRes for NotFoundRenderResInterceptor {
    async fn chain_res(&self, _: &Request, mut response: Response) -> Response {
        if response.status() == HttpStatus::NotFound {
            response.add_body(HBS.render(NOT_FOUND_TEMPLATE, &()).unwrap().as_bytes());
        }

        response
    }
}
