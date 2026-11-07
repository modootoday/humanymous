//! Native address/home/reload intents. Target documents never own these controls.
use crate::{child::Shared, navigation};
use humanymous_browser::{
    internal_urls::{self, Route},
    ui_navigation::{self, ImeGuard, Intent},
};
use meari_native::infrastructure::native_host::NativeDocumentHandle;
use std::io;
use winit::{
    event::{ElementState, Ime, WindowEvent},
    keyboard::{Key, NamedKey},
};
#[path = "internal_probe.rs"]
mod probe;

pub struct Controls {
    document: NativeDocumentHandle,
    state: Shared,
    cursor: usize,
    ime: ImeGuard,
    committed: u64,
    notice: Option<&'static str>,
    controls: Option<humanymous_browser::history::Controls>,
    probe: Option<probe::Probe>,
}
impl Controls {
    pub fn new(document: NativeDocumentHandle, state: Shared) -> Self {
        let probe_enabled = std::env::args().any(|a| a == "--native-internal-probe");
        Self {
            document,
            state,
            cursor: 0,
            ime: ImeGuard::default(),
            committed: 0,
            notice: None,
            controls: None,
            probe: probe_enabled.then(probe::Probe::default),
        }
    }
    pub fn event(&mut self, event: &WindowEvent) -> io::Result<()> {
        match event {
            WindowEvent::Ime(Ime::Preedit(text, _)) => self.ime.preedit(!text.is_empty()),
            WindowEvent::Ime(Ime::Commit(_)) => self.ime.commit(),
            WindowEvent::Ime(Ime::Disabled) | WindowEvent::Focused(false) => self.ime.cancel(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if event.logical_key == Key::Named(NamedKey::Enter) {
                    let focused = self
                        .document
                        .try_with(ui_navigation::address_focused)
                        .map_err(io::Error::other)?;
                    if self.ime.enter(focused, event.repeat) {
                        let address = self
                            .document
                            .try_with(ui_navigation::address)
                            .map_err(io::Error::other)?;
                        if let Some(address) = address {
                            self.dispatch(Intent::Open(address))?;
                        }
                    }
                } else {
                    self.ime.ordinary_key();
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn dispatch(&mut self, intent: Intent) -> io::Result<()> {
        let (browser, current) = {
            let state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?;
            if state.revoked || state.closed {
                return Ok(());
            }
            (
                state.browser.clone(),
                state.navigation.committed().map(|(_, r)| r),
            )
        };
        if matches!(intent, Intent::Back | Intent::Forward | Intent::Stop) {
            if let Some(browser) = browser {
                let action = match intent {
                    Intent::Back => crate::history::Action::Back,
                    Intent::Forward => crate::history::Action::Forward,
                    _ => crate::history::Action::Stop,
                };
                crate::history::queue(browser, self.state.clone(), action)?;
            }
            return Ok(());
        }
        let (route, reload) = match intent {
            Intent::Home => (Ok(Route::NewTab), false),
            Intent::Open(address) => (internal_urls::classify(&address), false),
            Intent::Reload => (current.ok_or("No committed local page"), true),
            Intent::Back | Intent::Forward | Intent::Stop => {
                unreachable!("history dispatched above")
            }
        };
        match (route, browser) {
            (Ok(route), Some(browser)) => {
                navigation::queue(browser, route, self.state.clone(), reload)?;
            }
            (Err(_), _) => {
                self.state
                    .lock()
                    .map_err(|_| io::Error::other("state poisoned"))?
                    .navigation
                    .notice = Some("Only local newtab and permitted diagnostics are available");
            }
            _ => {}
        }
        Ok(())
    }
    pub fn poll(&mut self) -> io::Result<()> {
        let (cursor, intents) = self
            .document
            .try_with(|doc| ui_navigation::intents_since(doc, self.cursor))
            .map_err(io::Error::other)?;
        if cursor > 4096 {
            return Err(io::Error::other("navigation intent budget exceeded"));
        }
        self.cursor = cursor;
        for intent in intents {
            self.dispatch(intent)?;
        }
        if let Some(probe) = &mut self.probe {
            probe.tick(&self.state)?;
        }
        let (committed, notice, controls) = {
            let state = self
                .state
                .lock()
                .map_err(|_| io::Error::other("state poisoned"))?;
            let mut controls = state.navigation.controls;
            controls.loading |= state.navigation.transaction.pending().is_some();
            controls.back &= !controls.loading;
            controls.forward &= !controls.loading;
            (
                state.navigation.committed(),
                state
                    .navigation
                    .notice
                    .or(controls.loading.then_some("Loading local page")),
                controls,
            )
        };
        if self.controls != Some(controls) {
            let stamp = self.document.stamp().map_err(io::Error::other)?;
            self.document
                .try_reproject(&stamp, |doc| {
                    ui_navigation::controls(doc, controls);
                    Ok::<_, io::Error>(())
                })
                .map_err(io::Error::other)?;
            self.controls = Some(controls);
        }
        if let Some((generation, route)) = committed
            && generation != self.committed
        {
            let stamp = self.document.stamp().map_err(io::Error::other)?;
            self.document
                .try_reproject(&stamp, |doc| {
                    ui_navigation::committed(doc, route).map_err(io::Error::other)
                })
                .map_err(io::Error::other)?;
            self.committed = generation;
        }
        if notice != self.notice {
            let stamp = self.document.stamp().map_err(io::Error::other)?;
            self.document
                .try_reproject(&stamp, |doc| {
                    ui_navigation::notice(doc, notice.unwrap_or("LOCAL ONLY"));
                    Ok::<_, io::Error>(())
                })
                .map_err(io::Error::other)?;
            self.notice = notice;
        }
        Ok(())
    }
}
