//! CEF-owned history traversal. No mirrored application history or foreign routes.
use crate::child::Shared;
use cef::*;
use humanymous_browser::{
    history::Entries,
    internal_urls,
    navigation::{Outcome, Ticket},
};
use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy)]
pub enum Action {
    Back,
    Forward,
    Stop,
}

pub fn queue(browser: Browser, state: Shared, action: Action) -> io::Result<()> {
    let mut task = HistoryTask::new(browser, state, action);
    if post_task(ThreadId::UI, Some(&mut task)) != 1 {
        return Err(io::Error::other("history UI task rejected"));
    }
    Ok(())
}
wrap_task! {
    struct HistoryTask { browser: Browser, state: Shared, action: Action, }
    impl Task {
        fn execute(&self) {
            let anchor = {
                let Ok(mut state)=self.state.lock() else { return; };
                if !state.is_live() { return; }
                if matches!(self.action, Action::Stop) {
                    if let Some(ticket)=state.navigation.transaction.pending() {
                        state.navigation.finish(ticket,Outcome::Stopped,0,0);
                    }
                    drop(state); self.browser.stop_load(); return;
                }
                if state.navigation.transaction.pending().is_some() || state.navigation.controls.loading { return; }
                let Some(anchor)=state.navigation.transaction.current() else { return; };
                anchor
            };
            let Some(host)=self.browser.host() else { return; };
            let mut visitor=HistoryVisitor::new(self.browser.clone(),self.state.clone(),anchor,
                matches!(self.action,Action::Forward),Arc::new(Mutex::new(Entries::default())));
            host.navigation_entries(Some(&mut visitor),0);
        }
    }
}
wrap_navigation_entry_visitor! {
    struct HistoryVisitor { browser: Browser, state: Shared, anchor: Ticket, forward: bool, entries: Arc<Mutex<Entries>>, }
    impl NavigationEntryVisitor {
        fn visit(&self,entry:Option<&mut NavigationEntry>,current:i32,index:i32,total:i32)->i32 {
            let route=entry.and_then(|e|internal_urls::classify(&CefString::from(&e.url()).to_string()).ok());
            let target={
                let Ok(mut entries)=self.entries.lock() else { return 0; };
                if !matches!(current,0|1) || !entries.push(route,current==1,index,total) { return 0; }
                if index!=total-1 { return 1; }
                entries.target(self.forward,self.anchor.route())
            };
            let Some(target)=target else { return 0; };
            let available=if self.forward { self.browser.can_go_forward()==1 } else { self.browser.can_go_back()==1 };
            let current_route=self.browser.main_frame().and_then(|f|internal_urls::classify(&CefString::from(&f.url()).to_string()).ok());
            if !available || current_route!=Some(self.anchor.route()) { return 0; }
            let Ok(mut state)=self.state.lock() else { return 0; };
            if !state.is_live() || state.navigation.transaction.current()!=Some(self.anchor)
                || state.navigation.transaction.pending().is_some() || state.navigation.controls.loading { return 0; }
            if state.navigation.begin(target,"request_accepted").is_err() { return 0; }
            drop(state);
            if self.forward { self.browser.go_forward(); } else { self.browser.go_back(); }
            0
        }
    }
}
