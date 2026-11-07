//! Fixed-peer byte relay; conservative contacts are not application mutations.
use serde_json::{Value, json};
use std::{
    io,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, watch},
    task::JoinSet,
};

#[derive(Clone, Default)]
pub struct Contacts {
    accepted: Arc<AtomicU32>,
    finished: Arc<AtomicU32>,
    failed: Arc<AtomicU32>,
}
impl Contacts {
    pub fn count(&self) -> u32 {
        self.accepted.load(Ordering::Acquire)
    }
    pub fn snapshot(&self) -> Value {
        json!({"contacts":self.count(),"completed":self.finished.load(Ordering::Acquire),"failed":self.failed.load(Ordering::Acquire),"counter_kind":"conservative_tcp_contacts","application_mutations_observed":false})
    }
}
async fn pipe(mut client: TcpStream, peer: SocketAddr) -> io::Result<()> {
    let mut remote = TcpStream::connect(peer).await?;
    let (client_read, mut client_write) = client.split();
    let (remote_read, mut remote_write) = remote.split();
    let send = async {
        let bytes = tokio::io::copy(&mut client_read.take(81921), &mut remote_write).await?;
        remote_write.shutdown().await?;
        if bytes > 81920 {
            return Err(io::Error::other("bounded relay request exceeded"));
        }
        Ok(())
    };
    let receive = async {
        let bytes = tokio::io::copy(&mut remote_read.take(81921), &mut client_write).await?;
        client_write.shutdown().await?;
        if bytes > 81920 {
            return Err(io::Error::other("bounded relay response exceeded"));
        }
        Ok(())
    };
    tokio::try_join!(send, receive)?;
    Ok(())
}
pub async fn serve(
    listener: TcpListener,
    peer: SocketAddr,
    contacts: Contacts,
    mut stop: watch::Receiver<bool>,
) -> io::Result<()> {
    if !(peer.ip().is_loopback() || matches!(peer.ip(),std::net::IpAddr::V4(ip)if ip.is_private()))
    {
        return Err(io::Error::other("owned relay peer required"));
    }
    let mut tasks = JoinSet::new();
    let limit = Arc::new(Semaphore::new(16));
    loop {
        tokio::select! {
            changed=stop.changed()=>{if changed.is_err()||*stop.borrow(){break;}}
            joined=tasks.join_next(),if !tasks.is_empty()=>{if joined.is_some_and(|r|r.is_err()){return Err(io::Error::other("relay worker failed"));}}
            accepted=listener.accept()=>{
                let (socket,_)=accepted?;
                contacts.accepted.try_update(Ordering::AcqRel,Ordering::Acquire,|n|n.checked_add(1)).map_err(|_|io::Error::other("contact count exhausted"))?;
                let Ok(permit)=limit.clone().try_acquire_owned()else{contacts.failed.fetch_add(1,Ordering::AcqRel);continue;};
                let counts=contacts.clone();
                tasks.spawn(async move {
                    let _permit=permit;
                    if matches!(tokio::time::timeout(Duration::from_secs(8),pipe(socket,peer)).await,Ok(Ok(()))){counts.finished.fetch_add(1,Ordering::AcqRel);}else{counts.failed.fetch_add(1,Ordering::AcqRel);}
                });
            }
        }
    }
    let drained = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(result) = tasks.join_next().await {
            result.map_err(io::Error::other)?;
        }
        Ok::<_, io::Error>(())
    })
    .await;
    match drained {
        Ok(result) => result,
        Err(_) => {
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
            Err(io::Error::other("relay drain deadline"))
        }
    }
}
