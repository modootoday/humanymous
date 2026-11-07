//! Join both owned listeners; failure signals the sibling before custody release.
use super::ShutdownRequest;
use crate::{application::Application, management::Management};
use std::io;
use tokio::{net::TcpListener, sync::watch};

pub(super) async fn run(
    application: &mut Application,
    listener: TcpListener,
    management: Management,
    mut external: watch::Receiver<Option<ShutdownRequest>>,
    tls: Option<std::sync::Arc<rustls::ServerConfig>>,
) -> io::Result<()> {
    let initial = *external.borrow();
    let (control, internal) = watch::channel(initial);
    let fallback = ShutdownRequest::new(1)?;
    let app_control = control.clone();
    let app_shutdown = internal.clone();
    let app = async move {
        let result = if let Some(tls) = tls {
            application
                .serve_tls_until(listener, tls, app_shutdown)
                .await
        } else {
            application.serve_until(listener, app_shutdown).await
        };
        if app_control.borrow().is_none() {
            let _ = app_control.send(Some(fallback));
        }
        result
    };
    let management_control = control.clone();
    let management_shutdown = internal.clone();
    let admin = async move {
        let result = management.run(management_shutdown).await;
        if management_control.borrow().is_none() {
            let _ = management_control.send(Some(fallback));
        }
        result
    };
    let forward = async move {
        let mut internal = internal;
        loop {
            if let Some(request) = *external.borrow() {
                let _ = control.send(Some(request));
                return Ok(());
            }
            tokio::select! {
                biased;
                changed = external.changed() => {
                    if changed.is_err() {
                        let _ = control.send(Some(fallback));
                        return Err(io::Error::other("shutdown controller closed"));
                    }
                }
                _ = internal.changed() => return Err(io::Error::other("service listener stopped before controller")),
            }
        }
    };
    let (app, admin, forward) = tokio::join!(app, admin, forward);
    app?;
    admin?;
    forward
}
