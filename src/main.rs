use httpr::http::{AsyncTryFrom, Request, Response};
use log::info;
use tokio::{
    io::{self, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let log_env = env_logger::Env::default().default_filter_or("info");
    env_logger::init_from_env(log_env);

    server().await.unwrap();
}

async fn server() -> io::Result<()> {
    const BIND: &str = "127.0.0.1:4444";

    info!("bind -> {BIND}");

    let listener = TcpListener::bind(BIND).await?;
    loop {
        let (stream, socket) = listener.accept().await?;

        info!("Connection from: {}:{}", socket.ip(), socket.port());

        tokio::spawn(async move { dummy_http(stream).await });
    }
}

async fn dummy_http(stream: TcpStream) -> Result<(), ()> {
    let (read_half, mut write_half) = stream.into_split();
    let reader = BufReader::new(read_half);

    let request: Result<Request, _> = AsyncTryFrom::try_from(reader).await;

    if request.is_err() {
        return Err(());
    }

    let request = request.unwrap();

    info!("{request:?}");
    info!("Body: {}", request.body_string().unwrap());

    let mut response = Response::new(httpr::http::HttpStatus::Ok);
    response.add_header(("Content-Type", "text/plain"));
    response.add_body("Everything is okay :)".as_bytes());

    write_half.write_all(&response.as_bytes()).await;

    Ok(())
}

// fn dummy_shell(mut s: TcpStream) -> io::Result<()> {
//     const PROMPT: &[u8] = b"> ";
//
//     info!("{:?}", thread::current().id());
//
//     let mut reader = BufReader::new(s.try_clone().unwrap());
//
//     info!("Connected {}", s.peer_addr()?);
//
//     // Hero
//     s.write_all(b"Welcome to rust-server!\n")?;
//     s.write_all(b"Press q to quit\n")?;
//
//     s.write_all(PROMPT)?;
//     s.flush()?;
//
//     loop {
//         let mut key = String::new();
//         reader.read_line(&mut key)?;
//
//         match key.trim() {
//             "q" | "quit" => {
//                 info!("Exiting...");
//                 s.write_all(b"Bye!\n")?;
//                 s.shutdown(Shutdown::Both)?;
//                 break;
//             }
//             "ls" | "list" => {
//                 let dir = Path::new(".");
//
//                 s.write_all(dir.canonicalize()?.to_string_lossy().as_bytes())?;
//                 s.write_all(b"\n")?;
//
//                 read_dir(dir)?
//                     .map(|v| v.unwrap().file_name())
//                     .for_each(|v| {
//                         s.write_fmt(format_args!("  {}\n", v.to_str().unwrap()))
//                             .unwrap()
//                     });
//             }
//             _ => (),
//         }
//
//         s.write_all(PROMPT)?;
//         s.flush()?;
//         info!("{}", key.trim());
//     }
//
//     Ok(())
// }
