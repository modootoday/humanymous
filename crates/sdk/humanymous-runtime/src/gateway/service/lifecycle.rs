//! Join every owned listener; failure signals the siblings before custody release.
use super::{ShutdownRequest, admin::AdminService};
use crate::gateway::{application::Application, management::Management};
use std::io;
use tokio::{net::TcpListener, sync::watch};

/// Listeners served beside the application listener.
pub(super) struct Siblings {
    /// Native management IPC listener.
    pub(super) management: Option<Management>,
    /// Loopback admin TCP listener (owner decision B01 #1).
    pub(super) admin: Option<AdminService>,
}

pub(super) async fn run(
    application: &mut Application,
    listener: TcpListener,
    siblings: Siblings,
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
    let management = async move {
        let Some(management) = siblings.management else {
            return Ok(());
        };
        let result = management.run(management_shutdown).await;
        if management_control.borrow().is_none() {
            let _ = management_control.send(Some(fallback));
        }
        result
    };
    let admin_control = control.clone();
    let admin_shutdown = internal.clone();
    let admin = async move {
        let Some(admin) = siblings.admin else {
            return Ok(());
        };
        let result = admin.run(admin_shutdown).await;
        if admin_control.borrow().is_none() {
            let _ = admin_control.send(Some(fallback));
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
    let (app, management, admin, forward) = tokio::join!(app, management, admin, forward);
    app?;
    management?;
    admin?;
    forward
}
