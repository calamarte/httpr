use std::{
    fs::read_dir,
    io::{self, BufRead, BufReader, BufWriter, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::Path,
    thread,
};

use httpr::http::Request;
use log::info;
use uuid::Uuid;

fn main() {
    let log_env = env_logger::Env::default().default_filter_or("info");

    env_logger::Builder::from_env(log_env)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}][{}][{}] {}: {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                thread::current().name().unwrap(), // Include thread ID
                record.target(),
                record.args()
            )
        })
        .init();

    server().unwrap();
}

fn server() -> io::Result<()> {
    const BIND: &str = "127.0.0.1:4444";

    info!("bind -> {BIND}");

    let listener = TcpListener::bind(BIND).unwrap();
    for stream in listener.incoming() {
        thread::Builder::new()
            .name(Uuid::new_v4().to_string())
            .spawn(move || {
                // dummy_shell(stream.unwrap()).unwrap();
                dummy_http(stream.unwrap()).unwrap();
            })?;
    }

    Ok(())
}

fn dummy_http(s: TcpStream) -> io::Result<()> {
    let request: Request = BufReader::new(s.try_clone().unwrap()).try_into()?;

    info!("{request:?}");

    let mut writer = BufWriter::new(s.try_clone().unwrap());
    let response = b"HTTP/1.1 404 Not Found\r\n\
        Content-Type: text/plain\r\n\
        Content-Length: 12\r\n\
        \r\n\
        Ping Pong!\r\n";

    writer.write_all(response)?;
    writer.flush()?;

    s.shutdown(Shutdown::Both)?;

    Ok(())
}

fn dummy_shell(mut s: TcpStream) -> io::Result<()> {
    const PROMPT: &[u8] = b"> ";

    info!("{:?}", thread::current().id());

    let mut reader = BufReader::new(s.try_clone().unwrap());

    info!("Connected {}", s.peer_addr()?);

    // Hero
    s.write_all(b"Welcome to rust-server!\n")?;
    s.write_all(b"Press q to quit\n")?;

    s.write_all(PROMPT)?;
    s.flush()?;

    loop {
        let mut key = String::new();
        reader.read_line(&mut key)?;

        match key.trim() {
            "q" | "quit" => {
                info!("Exiting...");
                s.write_all(b"Bye!\n")?;
                s.shutdown(Shutdown::Both)?;
                break;
            }
            "ls" | "list" => {
                let dir = Path::new(".");

                s.write_all(dir.canonicalize()?.to_string_lossy().as_bytes())?;
                s.write_all(b"\n")?;

                read_dir(dir)?
                    .map(|v| v.unwrap().file_name())
                    .for_each(|v| {
                        s.write_fmt(format_args!("  {}\n", v.to_str().unwrap()))
                            .unwrap()
                    });
            }
            _ => (),
        }

        s.write_all(PROMPT)?;
        s.flush()?;
        info!("{}", key.trim());
    }

    Ok(())
}
