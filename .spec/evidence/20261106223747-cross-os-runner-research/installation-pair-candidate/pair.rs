//! Six directed native query edges under the ACTUAL caller role token.
#![forbid(unsafe_code)]

use super::{Refusal, installation::InstallationObservation, windows_process::{ProcessObservation, SecurityObservation}, windows_service::ServiceObservation};
use windows_permissions::{LocalBox, SecurityDescriptor, Sid, constants::{AceType, SecurityInformation}, wrappers};
use ::windows_service::service::ServiceConfig;

/// Fixed index into installer-selected P/S/O locators, never a principal proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role { Publisher = 0, Service = 1, Observer = 2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edge { PublisherToService, ServiceToPublisher, PublisherToObserver, ObserverToPublisher, ServiceToObserver, ObserverToService }
impl Edge {
    fn roles(self) -> (Role, Role) {
        use Role::*;
        match self {
            Self::PublisherToService => (Publisher, Service), Self::ServiceToPublisher => (Service, Publisher),
            Self::PublisherToObserver => (Publisher, Observer), Self::ObserverToPublisher => (Observer, Publisher),
            Self::ServiceToObserver => (Service, Observer), Self::ObserverToService => (Observer, Service),
        }
    }
}

/// No Clone/Debug/raw getters. Lifetimes keep installation handles AND clock.
pub(super) struct EdgeObservation<'a> {
    installation: &'a InstallationObservation,
    edge: Edge,
    caller: Option<ProcessObservation>,
    caller_service: Option<ServiceObservation>,
    peer_service: Option<ServiceObservation>,
    peer: Option<ProcessObservation>,
    security: Option<SecurityObservation>,
    config: Option<ServiceConfig>,
}
pub(super) struct EdgeFailure<'a> { reason: Refusal, retained: EdgeObservation<'a> }
impl EdgeFailure<'_> {
    pub(super) fn reason(&self) -> Refusal { self.reason }
    /// Diagnostic count, never a native proof or cleanup receipt.
    pub(super) fn retained_pin_count(&self) -> usize { usize::from(self.retained.caller.is_some()) + usize::from(self.retained.peer.is_some()) }
}

fn protected(sd: &SecurityDescriptor) -> Result<(), Refusal> {
    let text = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(sd, SecurityInformation::Dacl)
        .map_err(|_| Refusal::Query)?;
    let flags = text.to_str().and_then(|value| value.strip_prefix("D:"))
        .and_then(|value| value.split_once('(')).map(|parts| parts.0).ok_or(Refusal::Descriptor)?;
    if !flags.contains('P') { return Err(Refusal::Descriptor); }
    Ok(())
}

/// Strict query-only role profile. No mutation/repair or caller-selected allowlist.
fn query_descriptor(sd: &SecurityDescriptor, owner: &str, peers: [&str; 2], process: bool) -> Result<(), Refusal> {
    let owner: LocalBox<Sid> = owner.parse().map_err(|_| Refusal::Descriptor)?;
    let peer0: LocalBox<Sid> = peers[0].parse().map_err(|_| Refusal::Descriptor)?;
    let peer1: LocalBox<Sid> = peers[1].parse().map_err(|_| Refusal::Descriptor)?;
    let system: LocalBox<Sid> = "S-1-5-18".parse().map_err(|_| Refusal::Descriptor)?;
    let admin: LocalBox<Sid> = "S-1-5-32-544".parse().map_err(|_| Refusal::Descriptor)?;
    let principals = [&*owner, &*system, &*admin, &*peer0, &*peer1];
    for i in 0..principals.len() {
        for j in i + 1..principals.len() { if principals[i] == principals[j] { return Err(Refusal::Descriptor); } }
    }
    if sd.owner() != Some(&*owner) { return Err(Refusal::Descriptor); }
    protected(sd)?;
    let acl = sd.dacl().ok_or(Refusal::Descriptor)?;
    let mut counts = [0_u8; 5];
    for index in 0..acl.len() {
        let ace = acl.get_ace(index).ok_or(Refusal::Descriptor)?;
        let role = principals.iter().position(|principal| ace.sid() == Some(*principal)).ok_or(Refusal::Descriptor)?;
        let expected = if role >= 3 { if process { 0x0012_1000 } else { 0x0002_0008 } }
            else if process { 0x001f_ffff } else { 0x000f_01ff };
        if ace.ace_type() != AceType::ACCESS_ALLOWED_ACE_TYPE || ace.flags().bits() != 0 || ace.mask().bits() != expected {
            return Err(Refusal::QueryRightsUnavailable);
        }
        counts[role] = counts[role].checked_add(1).ok_or(Refusal::Descriptor)?;
    }
    // Exact owner/two peers mandatory, SYSTEM/admin baseline may each be absent.
    if counts[0] != 1 || counts[3] != 1 || counts[4] != 1 || counts[1] > 1 || counts[2] > 1 {
        return Err(Refusal::QueryRightsUnavailable);
    }
    Ok(())
}

impl<'a> EdgeObservation<'a> {
    pub(super) fn observe(installation: &'a InstallationObservation, edge: Edge) -> Result<Self, EdgeFailure<'a>> {
        let mut owner = Self { installation, edge, caller: None, caller_service: None, peer_service: None, peer: None, security: None, config: None };
        match owner.query() { Ok(()) => Ok(owner), Err(reason) => Err(EdgeFailure { reason, retained: owner }) }
    }

    fn query(&mut self) -> Result<(), Refusal> {
        self.installation.clock.check()?;
        self.installation.recheck_files()?;
        let (caller_role, peer_role) = self.edge.roles();
        let mut sids = Vec::new();
        for index in 0..3 {
            let name = self.installation.service_name(index).ok_or(Refusal::InvalidSelection)?;
            sids.push(crate::native_security::storage::resolve_service_sid(name).map_err(|_| Refusal::Query)?);
            self.installation.clock.check()?;
        }
        if sids[0] == sids[1] || sids[0] == sids[2] || sids[1] == sids[2] { return Err(Refusal::Descriptor); }
        let identity = crate::native_security::process::current_identity().map_err(|_| Refusal::Query)?;
        if identity.sid() != sids[caller_role as usize] || !identity.installer_allowed() { return Err(Refusal::WrongCaller); }
        let caller_name = self.installation.service_name(caller_role as usize).ok_or(Refusal::InvalidSelection)?;
        self.caller_service = Some(ServiceObservation::open_locator(caller_name, self.installation.clock.deadline).map_err(|_| Refusal::Query)?);
        if self.caller_service.as_ref().unwrap().running_pid_locator().map_err(|_| Refusal::Query)? != std::process::id() {
            return Err(Refusal::WrongCaller);
        }
        self.caller = Some(ProcessObservation::open_locator(std::process::id(), self.installation.clock.deadline).map_err(|_| Refusal::Query)?);
        let peer_name = self.installation.service_name(peer_role as usize).ok_or(Refusal::InvalidSelection)?;
        self.peer_service = Some(ServiceObservation::open_locator(peer_name, self.installation.clock.deadline).map_err(|_| Refusal::Query)?);
        let service = self.peer_service.as_ref().unwrap();
        let pid = service.running_pid_locator().map_err(|_| Refusal::Query)?;
        self.config = Some(service.configuration_data().map_err(|_| Refusal::Query)?);
        self.peer = Some(ProcessObservation::open_locator(pid, self.installation.clock.deadline).map_err(|_| Refusal::Query)?);
        let peer = self.peer.as_ref().unwrap();
        if peer.primary_sid_data().map_err(|_| Refusal::Query)? != sids[peer_role as usize] { return Err(Refusal::WrongCaller); }
        self.security = Some(peer.security_data().map_err(|_| Refusal::Query)?);
        let other: Vec<_> = (0..3).filter(|index| *index != peer_role as usize).map(|index| sids[index].as_str()).collect();
        let security = self.security.as_ref().unwrap();
        query_descriptor(&security.process, &sids[peer_role as usize], [other[0], other[1]], true)?;
        query_descriptor(&security.token, &sids[peer_role as usize], [other[0], other[1]], false)?;
        if service.running_pid_locator().map_err(|_| Refusal::Query)? != pid
            || service.configuration_data().map_err(|_| Refusal::Query)? != *self.config.as_ref().unwrap()
        { return Err(Refusal::Changed); }
        peer.recheck().map_err(|_| Refusal::Query)?;
        self.caller.as_ref().unwrap().recheck().map_err(|_| Refusal::Query)?;
        if self.caller_service.as_ref().unwrap().running_pid_locator().map_err(|_| Refusal::Query)? != std::process::id() {
            return Err(Refusal::Changed);
        }
        self.installation.clock.check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_six_edges_have_distinct_fixed_directions() {
        let edges = [Edge::PublisherToService, Edge::ServiceToPublisher, Edge::PublisherToObserver,
            Edge::ObserverToPublisher, Edge::ServiceToObserver, Edge::ObserverToService];
        let mut pairs = std::collections::BTreeSet::new();
        for edge in edges { let (from, to) = edge.roles(); assert_ne!(from, to); assert!(pairs.insert((from as usize, to as usize))); }
        assert_eq!(pairs.len(), 6);
    }
    #[test]
    fn exact_query_descriptors_refuse_termination_rights_and_missing_peer() {
        const OWNER: &str = "S-1-5-21-1-2-3-1000";
        const PEER0: &str = "S-1-5-21-1-2-3-1001";
        const PEER1: &str = "S-1-5-21-1-2-3-1002";
        for (process, full, query) in [(true, 0x001f_ffff_u32, 0x0012_1000_u32), (false, 0x000f_01ff, 0x0002_0008)] {
            let good = format!("O:{OWNER}D:P(A;;0x{full:x};;;{OWNER})(A;;0x{query:x};;;{PEER0})(A;;0x{query:x};;;{PEER1})");
            let sd: LocalBox<SecurityDescriptor> = good.parse().unwrap();
            query_descriptor(&sd, OWNER, [PEER0, PEER1], process).unwrap();
            let overbroad = good.replace(&format!("0x{query:x};;;{PEER0}"), &format!("0x{:x};;;{PEER0}", query | 1));
            let sd: LocalBox<SecurityDescriptor> = overbroad.parse().unwrap();
            assert!(query_descriptor(&sd, OWNER, [PEER0, PEER1], process).is_err());
            let missing = good.replace(&format!("(A;;0x{query:x};;;{PEER1})"), "");
            let sd: LocalBox<SecurityDescriptor> = missing.parse().unwrap();
            assert!(query_descriptor(&sd, OWNER, [PEER0, PEER1], process).is_err());
        }
    }
}
