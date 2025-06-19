//! Essential tools to build a http server

use std::any::type_name;
use std::collections::HashSet;
use std::fmt;
use std::ops::ControlFlow;
use std::path::Path;
use std::string::FromUtf8Error;
use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use log::{debug, error, info, log_enabled};
use strum_macros::{Display, EnumString};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Error},
    net::{tcp::OwnedReadHalf, TcpListener},
};
use url::Url;

macro_rules! define_status {
    ($($name:ident = ($code:expr, $desc:expr)),*) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum HttpStatus {
            $(
                $name,
            )*
        }

        impl HttpStatus {
            fn code(&self) -> u16 {
                match *self {
                    $(
                        HttpStatus::$name => $code,
                    )*
                }
            }

            fn description(&self) -> &'static str {
                match *self {
                    $(
                        HttpStatus::$name => $desc,
                    )*
                }
            }
        }
    }
}

define_status! {
    // 1xx Informational
    Continue = (100, "Continue"),
    SwitchingProtocols = (101, "Switching Protocols"),
    Processing = (102, "Processing"),
    EarlyHints = (103, "Early Hints"),

    // 2xx Success
    Ok = (200, "OK"),
    Created = (201, "Created"),
    Accepted = (202, "Accepted"),
    NonAuthoritativeInformation = (203, "Non-Authoritative Information"),
    NoContent = (204, "No Content"),
    ResetContent = (205, "Reset Content"),
    PartialContent = (206, "Partial Content"),
    MultiStatus = (207, "Multi-Status"),
    AlreadyReported = (208, "Already Reported"),
    ImUsed = (226, "IM Used"),

    // 3xx Redirection
    MultipleChoices = (300, "Multiple Choices"),
    MovedPermanently = (301, "Moved Permanently"),
    Found = (302, "Found"),
    SeeOther = (303, "See Other"),
    NotModified = (304, "Not Modified"),
    UseProxy = (305, "Use Proxy"),
    TemporaryRedirect = (307, "Temporary Redirect"),
    PermanentRedirect = (308, "Permanent Redirect"),

    // 4xx Client Errors
    BadRequest = (400, "Bad Request"),
    Unauthorized = (401, "Unauthorized"),
    PaymentRequired = (402, "Payment Required"),
    Forbidden = (403, "Forbidden"),
    NotFound = (404, "Not Found"),
    MethodNotAllowed = (405, "Method Not Allowed"),
    NotAcceptable = (406, "Not Acceptable"),
    ProxyAuthenticationRequired = (407, "Proxy Authentication Required"),
    RequestTimeout = (408, "Request Timeout"),
    Conflict = (409, "Conflict"),
    Gone = (410, "Gone"),
    LengthRequired = (411, "Length Required"),
    PreconditionFailed = (412, "Precondition Failed"),
    PayloadTooLarge = (413, "Payload Too Large"),
    UriTooLong = (414, "URI Too Long"),
    UnsupportedMediaType = (415, "Unsupported Media Type"),
    RangeNotSatisfiable = (416, "Range Not Satisfiable"),
    ExpectationFailed = (417, "Expectation Failed"),
    ImATeapot = (418, "I'm a teapot"),
    MisdirectedRequest = (421, "Misdirected Request"),
    UnprocessableEntity = (422, "Unprocessable Entity"),
    Locked = (423, "Locked"),
    FailedDependency = (424, "Failed Dependency"),
    TooEarly = (425, "Too Early"),
    UpgradeRequired = (426, "Upgrade Required"),
    PreconditionRequired = (428, "Precondition Required"),
    TooManyRequests = (429, "Too Many Requests"),
    RequestHeaderFieldsTooLarge = (431, "Request Header Fields Too Large"),
    UnavailableForLegalReasons = (451, "Unavailable For Legal Reasons"),

    // 5xx Server Errors
    InternalServerError = (500, "Internal Server Error"),
    NotImplemented = (501, "Not Implemented"),
    BadGateway = (502, "Bad Gateway"),
    ServiceUnavailable = (503, "Service Unavailable"),
    GatewayTimeout = (504, "Gateway Timeout"),
    HttpVersionNotSupported = (505, "HTTP Version Not Supported"),
    VariantAlsoNegotiates = (506, "Variant Also Negotiates"),
    InsufficientStorage = (507, "Insufficient Storage"),
    LoopDetected = (508, "Loop Detected"),
    NotExtended = (510, "Not Extended"),
    NetworkAuthenticationRequired = (511, "Network Authentication Required")
}

pub trait Named {
    fn name(&self) -> &str {
        type_name::<Self>().split("::").last().unwrap()
    }
}

#[async_trait]
pub trait HttpHandler: Send + Sync + Named + 'static {
    async fn solve_request(&self, request: &Request) -> Result<Response, &'static str>;
}

#[async_trait]
pub trait InterceptorReq: Send + Sync + Named {
    async fn chain_req(&self, request: Request) -> ControlFlow<Response, Request>;
}

#[async_trait]
pub trait InterceptorRes: Send + Sync + Named {
    async fn chain_res(&self, request: &Request, response: Response) -> Response;
}

#[async_trait]
pub trait AsyncTryFrom<T>: Sized {
    type Error;

    async fn try_from(value: T) -> Result<Self, Self::Error>;
}

const HTTP_VERSION: &str = "HTTP/1.1";

#[derive(Default, Debug, Clone, Copy, EnumString, Display, Eq, PartialEq, Hash)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

#[derive(Default, Debug)]
#[allow(dead_code)]
pub struct Request {
    method: Method,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, uri: String, version: String) -> Self {
        Self {
            method,
            uri,
            version,
            ..Default::default()
        }
    }

    /// Return request method
    pub fn method(&self) -> Method {
        self.method
    }

    pub fn body_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.body.to_vec())
    }

    /// ```
    /// let mut request = Request::new(Method::Get, String::from("/"), HTTP_VERSION);
    /// request.
    ///
    /// assert_eq!(request.method.url(), request.method());
    /// ```
    pub fn url(&self) -> Url {
        let host = self.headers.get("host").unwrap();
        Url::parse(&format!("http://{host}{}", self.uri)).unwrap()
    }
}

#[async_trait]
impl AsyncTryFrom<BufReader<OwnedReadHalf>> for Request {
    type Error = Error;

    async fn try_from(value: BufReader<OwnedReadHalf>) -> Result<Self, Self::Error> {
        let reader = BufReader::new(value);
        let mut lines = reader.lines();

        let first_line = lines.next_line().await.unwrap().unwrap();
        let mut parts = first_line.split_whitespace();

        let (verb, uri, protocol) = (
            parts
                .next()
                .expect("verb")
                .to_uppercase()
                .parse::<Method>()
                .expect("Not allowed method!"),
            parts.next().expect("path").to_string(),
            parts.next().expect("protocol").to_lowercase(),
        );

        let mut request = Request::new(verb, uri, protocol);

        while let Some(line) = lines.next_line().await? {
            if line.is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(": ") {
                request.headers.insert(key.to_lowercase(), value.into());
            }
        }

        if let Some(len) = request.headers.get("content-length") {
            let len = len.parse().unwrap_or(0usize);
            request.body.resize(len, 0);

            lines.get_mut().read_exact(&mut request.body).await?;
        }

        Ok(request)
    }
}

#[derive(Debug)]
pub struct Response {
    status: HttpStatus,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    pub fn new(status: HttpStatus) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn not_found() -> Self {
        Self::new(HttpStatus::NotFound)
    }

    pub fn redirect<P: AsRef<Path>>(path: P) -> Self {
        let mut response = Self::new(HttpStatus::MovedPermanently);
        response.headers.insert(
            "Location".to_string(),
            path.as_ref().to_str().unwrap().to_string(),
        );

        response
    }

    pub fn allowed(methods: HashSet<Method>) -> Self {
        let methods_string = methods
            .into_iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let mut headers = HashMap::with_capacity(1);
        headers.insert("Allowed".to_string(), methods_string);

        Self {
            status: HttpStatus::NoContent,
            headers,
            body: Vec::new(),
        }
    }

    pub fn status(&self) -> HttpStatus {
        self.status
    }

    pub fn add_header(&mut self, (k, value): (&str, &str)) {
        self.headers.insert(k.to_lowercase(), value.to_string());
    }

    pub fn add_body(&mut self, body: &[u8]) {
        self.body = body.to_vec();
    }

    pub fn clean_body(&mut self) {
        self.body.clear();
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let status_line = format!(
            "{} {} {}\r\n",
            HTTP_VERSION,
            self.status.code(),
            self.status.description()
        );
        bytes.extend_from_slice(status_line.as_bytes());

        for (k, v) in &self.headers {
            let line = format!("{k}: {v}\r\n");
            bytes.extend_from_slice(line.as_bytes());
        }

        let len_line = format!("Content-Length: {}\r\n\r\n", self.body.len());
        bytes.extend_from_slice(len_line.as_bytes());

        bytes.extend_from_slice(&self.body);

        bytes
    }
}

pub struct Server<H> {
    bind: String,
    handler: Arc<H>,
    interceptors_req: Vec<Arc<dyn InterceptorReq>>,
    interceptors_res: Vec<Arc<dyn InterceptorRes>>,
}

impl<H: HttpHandler> Server<H> {
    pub fn new(bind: String, handler: H) -> Self {
        Self {
            bind,
            handler: Arc::new(handler),
            interceptors_req: Vec::new(),
            interceptors_res: Vec::new(),
        }
    }

    pub fn push_req_inter(&mut self, req_inter: Arc<dyn InterceptorReq>) -> &mut Self {
        self.interceptors_req.push(req_inter);
        self
    }

    pub fn push_res_inter(&mut self, res_inter: Arc<dyn InterceptorRes>) -> &mut Self {
        self.interceptors_res.push(res_inter);
        self
    }

    pub async fn run(&self) -> io::Result<()> {
        debug!("Running in a debug mode...");
        debug!("Server chain: {self:?}");

        info!("bind -> {}", self.bind);

        let listener = TcpListener::bind(&self.bind).await?;
        loop {
            let (stream, socket) = listener.accept().await?;

            debug!("Connection from: {}:{}", socket.ip(), socket.port());

            let handler = self.handler.clone();
            let interceptor_req = self.interceptors_req.clone();
            let interceptor_res = self.interceptors_res.clone();

            tokio::spawn(async move {
                let (read_half, mut write_half) = stream.into_split();
                let reader = BufReader::new(read_half);

                let mut request: Request = match AsyncTryFrom::try_from(reader).await {
                    Ok(req) => req,
                    Err(_) => {
                        error!("Server can't build the request!");
                        return;
                    }
                };

                if !log_enabled!(log::Level::Debug) {
                    info!("Request -> [{}] {}", request.method, request.uri);
                }

                debug!("Request -> {request:?}");

                // Run interceptors_req
                for interceptor in &interceptor_req {
                    match interceptor.chain_req(request).await {
                        ControlFlow::Continue(r) => request = r,
                        ControlFlow::Break(res) => {
                            write_half.write_all(&res.as_bytes()).await.unwrap();
                            return;
                        }
                    }
                }

                // Run handler
                let mut response = match handler.solve_request(&request).await {
                    Ok(res) => res,
                    Err(msg) => {
                        error!("{msg}");
                        Response::new(HttpStatus::InternalServerError)
                    }
                };

                // Run interceptors_req
                for interceptor in &interceptor_res {
                    response = interceptor.chain_res(&request, response).await;
                }

                debug!("Response -> {response:?}");

                write_half.write_all(&response.as_bytes()).await.unwrap();
            });
        }
    }
}

impl<H: Named> fmt::Debug for Server<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_req = self
            .interceptors_req
            .iter()
            .map(|i| i.name())
            .collect::<Vec<_>>()
            .join(" -> ");
        let int_res = self
            .interceptors_res
            .iter()
            .map(|i| i.name())
            .collect::<Vec<_>>()
            .join(" -> ");

        write!(f, "{int_req} -> [{}] -> {int_res}", self.handler.name())
    }
}
