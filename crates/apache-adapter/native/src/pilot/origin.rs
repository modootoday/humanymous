//! Owned loopback effect witness; no request bytes are retained in the report.
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
pub struct Origin {
    pub port: u16,
    pub count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Origin {
    pub fn start() -> Result<Self, super::Error> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let observed = count.clone();
        let cancelled = stop.clone();
        let thread = std::thread::spawn(move || {
            while !cancelled.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = serve(&mut stream, &observed);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            port,
            count,
            stop,
            thread: Some(thread),
        })
    }
}
impl Drop for Origin {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn serve(stream: &mut TcpStream, count: &AtomicUsize) -> Result<(), super::Error> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut header = Vec::new();
    let mut byte = [0];
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 65536 {
            return Err("origin header bound".into());
        }
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let text = std::str::from_utf8(&header)?;
    let mut length = None;
    let mut spoof = false;
    for line in text.split("\r\n").skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length")
                && length.replace(value.trim().parse::<usize>()?).is_some()
            {
                return Err("origin duplicate framing".into());
            }
            spoof |= humanymous_framework_adapter::internal_header(name);
        }
    }
    let length = length.ok_or("origin framing")?;
    if length > 1_048_576 {
        return Err("origin body bound".into());
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    count.fetch_add(1, Ordering::AcqRel);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nX-Pilot-Spoof-Present: {}\r\n\r\n",
        body.len(),
        u8::from(spoof)
    )?;
    stream.write_all(&body)?;
    Ok(())
}
