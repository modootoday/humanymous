//! Synthetic Rust origin and HTTP1 client; no application code or external target.
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

#[derive(Clone)]
pub struct Effect {
    pub target: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub durable_at_effect: usize,
}
pub struct Origin {
    pub port: u16,
    pub effects: Arc<Mutex<Vec<Effect>>>,
}
impl Origin {
    pub fn start(audits: Arc<AtomicUsize>) -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let effects = Arc::new(Mutex::new(Vec::new()));
        let captured = effects.clone();
        std::thread::spawn(move || {
            for connection in listener.incoming() {
                let Ok(stream) = connection else {
                    break;
                };
                let effects = captured.clone();
                let audits = audits.clone();
                std::thread::spawn(move || {
                    let _ = serve(stream, effects, audits);
                });
            }
        });
        Ok(Self { port, effects })
    }
    pub fn count(&self) -> usize {
        self.effects.lock().unwrap().len()
    }
    pub fn last(&self) -> Effect {
        self.effects.lock().unwrap().last().unwrap().clone()
    }
}
fn serve(
    mut stream: TcpStream,
    effects: Arc<Mutex<Vec<Effect>>>,
    audits: Arc<AtomicUsize>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let head = head(&mut reader)?;
    let mut lines = head.lines();
    let first = lines
        .next()
        .ok_or_else(|| io::Error::other("origin request"))?;
    let words: Vec<_> = first.split(' ').collect();
    if words.len() != 3 {
        return Err(io::Error::other("origin line"));
    }
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| {
            line.split_once(':')
                .map(|(a, b)| (a.to_owned(), b.trim().to_owned()))
        })
        .collect();
    let length = headers
        .iter()
        .find(|h| h.0.eq_ignore_ascii_case("content-length"))
        .map(|h| h.1.parse::<usize>())
        .transpose()
        .map_err(io::Error::other)?
        .unwrap_or(0);
    if length > 1_048_576 {
        return Err(io::Error::other("origin limit"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    effects.lock().unwrap().push(Effect {
        target: words[1].into(),
        method: words[0].into(),
        headers,
        body: body.clone(),
        durable_at_effect: audits.load(Ordering::Acquire),
    });
    if words[1] == "/stream" {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n9\r\ndata: a\n\n\r\n")?;
        stream.flush()?;
        std::thread::sleep(Duration::from_millis(1000));
        stream.write_all(b"9\r\ndata: b\n\n\r\n0\r\n\r\n")?;
        return Ok(());
    }
    let extra = if words[1] == "/redirect" {
        "X-Accel-Redirect: /echo\r\n"
    } else {
        ""
    };
    let status = if words[1] == "/empty" { 204 } else { 200 };
    let response = if status == 204 {
        &[][..]
    } else {
        body.as_slice()
    };
    write!(
        stream,
        "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nSet-Cookie: a=1; HttpOnly\r\nSet-Cookie: b=2; Secure\r\nX-Repeat: one\r\nX-Repeat: two\r\nETag: \"fixture\"\r\n{extra}Connection: close\r\n\r\n",
        response.len()
    )?;
    if words[0] != "HEAD" {
        stream.write_all(response)?;
    }
    Ok(())
}
pub fn head<R: BufRead>(reader: &mut R) -> io::Result<String> {
    let mut bytes = Vec::new();
    loop {
        let mut b = [0];
        reader.read_exact(&mut b)?;
        bytes.push(b[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
        if bytes.len() > 32768 {
            return Err(io::Error::other("header limit"));
        }
    }
    String::from_utf8(bytes).map_err(io::Error::other)
}
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
impl Response {
    pub fn values(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|h| h.0.eq_ignore_ascii_case(name))
            .map(|h| h.1.as_str())
            .collect()
    }
}
pub fn request(
    port: u16,
    method: &str,
    target: &str,
    body: &[u8],
    extra: &str,
) -> io::Result<Response> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "{method} {target} HTTP/1.1\r\nHost: example.test\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    let mut reader = BufReader::new(stream);
    read_response(&mut reader, method)
}

pub fn read_response<R: BufRead>(reader: &mut R, method: &str) -> io::Result<Response> {
    let head = head(reader)?;
    let status = head
        .split_whitespace()
        .nth(1)
        .unwrap_or("0")
        .parse()
        .map_err(io::Error::other)?;
    let headers: Vec<(String, String)> = head
        .lines()
        .skip(1)
        .filter_map(|line| {
            line.split_once(':')
                .map(|(a, b)| (a.into(), b.trim().into()))
        })
        .collect();
    let mut body = Vec::new();
    if method != "HEAD" && status != 204 && status != 304 {
        if headers.iter().any(|h| {
            h.0.eq_ignore_ascii_case("transfer-encoding") && h.1.eq_ignore_ascii_case("chunked")
        }) {
            loop {
                let mut line = String::new();
                reader.read_line(&mut line)?;
                let n = usize::from_str_radix(line.trim(), 16).map_err(io::Error::other)?;
                if n == 0 {
                    let mut end = [0; 2];
                    reader.read_exact(&mut end)?;
                    if end != *b"\r\n" {
                        return Err(io::Error::other("unsupported response trailers"));
                    }
                    break;
                }
                if body.len() + n > 2_000_000 {
                    return Err(io::Error::other("response bound"));
                }
                let at = body.len();
                body.resize(at + n, 0);
                reader.read_exact(&mut body[at..])?;
                let mut end = [0; 2];
                reader.read_exact(&mut end)?;
                if end != *b"\r\n" {
                    return Err(io::Error::other("chunk end"));
                }
            }
        } else if let Some((_, value)) = headers
            .iter()
            .find(|h| h.0.eq_ignore_ascii_case("content-length"))
        {
            let length = value.parse::<usize>().map_err(io::Error::other)?;
            if length > 2_000_000 {
                return Err(io::Error::other("response bound"));
            }
            body.resize(length, 0);
            reader.read_exact(&mut body)?;
        } else {
            reader.take(2_000_001).read_to_end(&mut body)?;
        }
    }
    Ok(Response {
        status,
        headers,
        body,
    })
}
