use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Read},
    net::TcpStream,
};

use strum_macros::EnumString;

macro_rules! define_status {
    ($($name:ident = ($code:expr, $desc:expr)),*) => {
        #[derive(Debug)]
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

const HTTP_VERSION: &str = "HTTP/1.1";

#[derive(Default, Debug, Clone, Copy, EnumString)]
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

// TODO: Build a macro to setup Status codes
// pub enum Status {
//     OK = (200, "OK"),
// }

#[derive(Default, Debug)]
pub struct Request {
    method: Method,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    fn new(method: Method, uri: String, version: String) -> Self {
        Self {
            method,
            uri,
            version,
            ..Default::default()
        }
    }
}

impl TryFrom<BufReader<TcpStream>> for Request {
    type Error = io::Error;

    fn try_from(mut reader: BufReader<TcpStream>) -> Result<Self, Self::Error> {
        let mut lines = reader.by_ref().lines();

        let first_line = lines.next().unwrap().unwrap();
        let mut parts = first_line.split_whitespace();

        let (verb, uri, protocol) = (
            parts
                .next()
                .expect("verb")
                .to_uppercase()
                .parse::<Method>()
                .expect("Not allowed method!"),
            parts.next().expect("path").to_lowercase(),
            parts.next().expect("protocol").to_lowercase(),
        );

        let mut request = Request::new(verb, uri, protocol);

        for line in lines {
            let line = line.unwrap();
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

            reader.read_exact(&mut request.body)?;
        }

        Ok(request)
    }
}

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

    pub fn add_header(&mut self, (k, value): (&str, &str)) {
        self.headers.insert(k.to_lowercase(), value.to_string());
    }

    pub fn add_body(&mut self, body: &[u8]) {
        self.body = body.to_vec();
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
