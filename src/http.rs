use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Read},
    net::TcpStream,
};

use strum_macros::EnumString;

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
            let mut body = vec![0u8; len];

            reader.read_exact(&mut body)?;

            request.body = body;
        }

        Ok(request)
    }
}

struct Response {}
