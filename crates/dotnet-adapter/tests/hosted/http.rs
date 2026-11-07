//! HTTP fixture IO borrows an original host deadline; no per-operation renewal.
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, SocketAddrV4, TcpStream},
    time::{Duration, Instant},
};

pub(super) struct Socket {
    inner: TcpStream,
    until: Instant,
}

impl Socket {
    pub(super) fn connect(port: u16, original_run_until: Instant) -> io::Result<Self> {
        // One request ceiling can shorten the original budget, never extend it.
        let now = Instant::now();
        let until = original_run_until.min(now + Duration::from_secs(8));
        let timeout = remaining(until)?;
        let address = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
        let inner = TcpStream::connect_timeout(&address, timeout)?;
        remaining(until)?;
        Ok(Self { inner, until })
    }

    pub(super) fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }
}

fn remaining(until: Instant) -> io::Result<Duration> {
    until
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "original HTTP budget exhausted"))
}

impl Read for Socket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.inner.set_read_timeout(Some(remaining(self.until)?))?;
        let count = self.inner.read(bytes)?;
        remaining(self.until)?;
        Ok(count)
    }
}

impl Write for Socket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner.set_write_timeout(Some(remaining(self.until)?))?;
        let count = self.inner.write(bytes)?;
        remaining(self.until)?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.set_write_timeout(Some(remaining(self.until)?))?;
        self.inner.flush()?;
        remaining(self.until)?;
        Ok(())
    }
}
