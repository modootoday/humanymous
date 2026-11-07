//! Serialize native resource mutations between application requests.
use super::*;
use crate::{
    management::{SettingsCommand, SharedProfile},
    settings::SettingsStore,
    startup::SealedAudit,
};
use std::{
    io,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

pub(super) struct ManagedSettings {
    store: SettingsStore,
    audit: Arc<Mutex<SealedAudit>>,
    profile: SharedProfile,
    commands: Option<mpsc::Receiver<SettingsCommand>>,
}
pub(super) async fn next_command(
    receiver: &mut Option<mpsc::Receiver<SettingsCommand>>,
) -> SettingsCommand {
    if let Some(queue) = receiver {
        if let Some(command) = queue.recv().await {
            return command;
        }
        *receiver = None;
    }
    std::future::pending().await
}
impl Application {
    pub(crate) fn install_settings(
        &mut self,
        store: SettingsStore,
        audit: Arc<Mutex<SealedAudit>>,
        profile: SharedProfile,
        commands: Option<mpsc::Receiver<SettingsCommand>>,
    ) {
        self.managed_settings = Some(ManagedSettings {
            store,
            audit,
            profile,
            commands,
        });
    }
    pub(crate) fn operational_revision(&self) -> Option<u64> {
        self.managed_settings
            .as_ref()
            .map(|settings| settings.store.snapshot().revision())
    }
    pub(super) fn take_settings_commands(&mut self) -> Option<mpsc::Receiver<SettingsCommand>> {
        self.managed_settings
            .as_mut()
            .and_then(|settings| settings.commands.take())
    }
    pub(super) fn apply_settings_command(
        &mut self,
        command: SettingsCommand,
        shutdown: &tokio::sync::watch::Receiver<Option<crate::service::ShutdownRequest>>,
    ) -> io::Result<()> {
        let SettingsCommand {
            change,
            context,
            response,
        } = command;
        let last_checked = std::cell::Cell::new(None);
        let check = || {
            if shutdown.borrow().is_some() {
                return Err(Rejection::new(503, "service_stopping"));
            }
            let now = context.check()?;
            if last_checked.get().is_some_and(|previous| now < previous) {
                return Err(Rejection::new(503, "settings_clock_rollback"));
            }
            last_checked.set(Some(now));
            Ok(now)
        };
        let prepared = (|| {
            let now = check()?;
            self.sessions.observe(now)?;
            if change.scope != crate::management::ManagementScope::from_scope(&self.config.scope) {
                return Err(Rejection::new(403, "management_scope"));
            }
            self.managed_settings
                .as_ref()
                .ok_or(Rejection::new(403, "management_readonly"))?
                .store
                .snapshot()
                .prepare(&self.config, change.expected_revision, change.settings)
        })();
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                let _ = response.send(Err(error));
                return Ok(());
            }
        };
        let result = (|| {
            let settings = self
                .managed_settings
                .as_mut()
                .ok_or_else(|| io::Error::other("settings owner unavailable"))?;
            let mut audit = settings
                .audit
                .lock()
                .map_err(|_| io::Error::other("audit poisoned"))?;
            settings
                .store
                .commit(&self.config, prepared, &mut audit, || {
                    check()
                        .map(|_| ())
                        .map_err(|_| io::Error::other("settings request cancelled or expired"))
                })
        })();
        let applied = (|| {
            let committed = result?;
            let now = check().map_err(|_| io::Error::other("settings outcome unknown"))?;
            self.sessions
                .observe(now)
                .map_err(|_| io::Error::other("settings epoch unavailable"))?;
            let snapshot = serde_json::to_value(committed.snapshot())?;
            self.apply_committed_settings(committed)
                .map_err(|_| io::Error::other("settings epoch application failed"))?;
            let now = check().map_err(|_| io::Error::other("settings outcome unknown"))?;
            self.sessions
                .observe(now)
                .map_err(|_| io::Error::other("settings epoch unavailable"))?;
            let settings = self
                .managed_settings
                .as_ref()
                .ok_or_else(|| io::Error::other("settings owner unavailable"))?;
            let mut profile = settings
                .profile
                .write()
                .map_err(|_| io::Error::other("settings profile poisoned"))?;
            profile["operational"] = serde_json::to_value(self.config.operational())?;
            profile["operational_revision"] =
                serde_json::json!(settings.store.snapshot().revision());
            Ok::<_, io::Error>(snapshot)
        })();
        match applied {
            Ok(snapshot) => {
                let _ = response.send(Ok(snapshot));
                Ok(())
            }
            Err(error) => {
                self.sessions.poison();
                let _ = response.send(Err(Rejection::new(503, "settings_outcome_unknown")));
                Err(error)
            }
        }
    }
}
