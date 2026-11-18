//! Plain bidirectional TCP relay. It carries bytes unchanged and adds no
//! protocol: the gateway still parses, authenticates, audits and forwards.
use std::{
    io,
    net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs},
    thread,
    time::Duration,
};

const CONNECT: Duration = Duration::from_secs(5);
const IDLE: Duration = Duration::from_secs(30);

/// Accept on `listener` forever, relaying each connection to `upstream`
/// (resolved per connection). A failed upstream connect closes the client.
pub fn spawn(listener: TcpListener, upstream: String) {
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(client) = stream else { continue };
            let upstream = upstream.clone();
            thread::spawn(move || {
                let _ = pipe(client, &upstream);
            });
        }
    });
}

fn connect(upstream: &str) -> io::Result<TcpStream> {
    let mut last = io::Error::other("upstream did not resolve");
    for address in upstream.to_socket_addrs()? {
        match TcpStream::connect_timeout(&address, CONNECT) {
            Ok(stream) => return Ok(stream),
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn pipe(client: TcpStream, upstream: &str) -> io::Result<()> {
    let server = connect(upstream)?;
    for stream in [&client, &server] {
        stream.set_read_timeout(Some(IDLE))?;
        stream.set_write_timeout(Some(IDLE))?;
    }
    let (mut client_read, mut server_write) = (client.try_clone()?, server.try_clone()?);
    let forward = thread::spawn(move || {
        let _ = io::copy(&mut client_read, &mut server_write);
        let _ = server_write.shutdown(Shutdown::Write);
    });
    let (mut server_read, mut client_write) = (server, client);
    let _ = io::copy(&mut server_read, &mut client_write);
    let _ = client_write.shutdown(Shutdown::Write);
    let _ = forward.join();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn relays_bytes_both_ways_unchanged() {
        let backend = TcpListener::bind("127.0.0.1:0").unwrap();
        let backend_address = backend.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = backend.accept().unwrap();
            let mut request = [0; 5];
            stream.read_exact(&mut request).unwrap();
            stream.write_all(b"pong:").unwrap();
            stream.write_all(&request).unwrap();
        });
        let front = TcpListener::bind("127.0.0.1:0").unwrap();
        let front_address = front.local_addr().unwrap();
        spawn(front, backend_address.to_string());
        let mut client = TcpStream::connect(front_address).unwrap();
        client.write_all(b"ping!").unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut reply = Vec::new();
        client.read_to_end(&mut reply).unwrap();
        assert_eq!(reply, b"pong:ping!");
        server.join().unwrap();
    }

    #[test]
    fn unreachable_upstream_closes_the_client_without_bytes() {
        let unused = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead = unused.local_addr().unwrap();
        drop(unused);
        let front = TcpListener::bind("127.0.0.1:0").unwrap();
        let front_address = front.local_addr().unwrap();
        spawn(front, dead.to_string());
        let mut client = TcpStream::connect(front_address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut reply = Vec::new();
        let _ = client.read_to_end(&mut reply);
        assert!(reply.is_empty());
    }
}
