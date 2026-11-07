use super::*;

pub(super) const CAPS: [usize; 3] = [467, 784, 275];

/// Parse three exact bounded public artifacts. No signature/native claim is made.
pub fn parse_bundle(
    delegation: &[u8],
    prepared: &[u8],
    commit: &[u8],
) -> Result<ParsedAuthorization> {
    Ok(ParsedAuthorization {
        delegation: signed(delegation, b"HMBGD002", CAPS[0], delegation_body)?,
        prepared: signed(prepared, b"HMBGP002", CAPS[1], prepared_body)?,
        commit: signed(commit, b"HMBGC002", CAPS[2], commit_body)?,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(AuthorizationError::Size)?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or(AuthorizationError::Encoding)?;
        self.offset = end;
        Ok(result)
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| AuthorizationError::Encoding)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.fixed::<1>()?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.fixed()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.fixed()?))
    }
    fn name(&mut self) -> Result<String> {
        let length = usize::from(self.byte()?);
        if !(16..=63).contains(&length) {
            return Err(AuthorizationError::Encoding);
        }
        let bytes = self.take(length)?;
        if !binding::valid_name(bytes) {
            return Err(AuthorizationError::Encoding);
        }
        String::from_utf8(bytes.to_vec()).map_err(|_| AuthorizationError::Encoding)
    }
    fn sid(&mut self) -> Result<Vec<u8>> {
        let length = usize::from(self.byte()?);
        if !(8..=68).contains(&length) {
            return Err(AuthorizationError::Encoding);
        }
        let bytes = self.take(length)?;
        if !binding::valid_sid(bytes) {
            return Err(AuthorizationError::Encoding);
        }
        Ok(bytes.to_vec())
    }
    fn flags(&mut self) -> Result<()> {
        if self.byte()? != 1 || self.byte()? != 0 {
            return Err(AuthorizationError::Encoding);
        }
        Ok(())
    }
}
fn signed<T>(
    bytes: &[u8],
    magic: &[u8; 8],
    cap: usize,
    parse: fn(&mut Reader<'_>) -> Result<T>,
) -> Result<Signed<T>> {
    if bytes.len() > cap {
        return Err(AuthorizationError::Size);
    }
    if bytes.len() < 74 || &bytes[..8] != magic {
        return Err(AuthorizationError::Encoding);
    }
    let length = usize::from(u16::from_be_bytes([bytes[8], bytes[9]]));
    if length.checked_add(74) != Some(bytes.len()) {
        return Err(AuthorizationError::Encoding);
    }
    let body = &bytes[10..10 + length];
    let mut reader = Reader {
        bytes: body,
        offset: 0,
    };
    if reader.fixed::<2>()? != [0, 2] {
        return Err(AuthorizationError::Encoding);
    }
    let claims = parse(&mut reader)?;
    if reader.offset != body.len() {
        return Err(AuthorizationError::Encoding);
    }
    Ok(Signed {
        claims,
        body: body.to_vec(),
        signature: bytes[10 + length..]
            .try_into()
            .map_err(|_| AuthorizationError::Encoding)?,
        artifact: bytes.to_vec(),
    })
}
fn delegation_body(r: &mut Reader<'_>) -> Result<Delegation> {
    let claims = Delegation {
        installation_epoch: r.fixed()?,
        scope: r.fixed()?,
        installer: r.fixed()?,
        installer_key: r.fixed()?,
        publisher_sid: r.sid()?,
        publisher_name: r.name()?,
        publisher_key: r.fixed()?,
        publisher_artifact: r.fixed()?,
        pair: r.fixed()?,
        id: r.fixed()?,
        not_before: r.u64()?,
        not_after: r.u64()?,
    };
    r.flags()?;
    if !binding::nonzero(&claims.installation_epoch) || !binding::nonzero(&claims.id) {
        return Err(AuthorizationError::Encoding);
    }
    Ok(claims)
}
fn prepared_body(r: &mut Reader<'_>) -> Result<Prepared> {
    let delegation = r.fixed()?;
    let scope = r.fixed()?;
    let installation = r.fixed()?;
    let publisher_key = r.fixed()?;
    let grant = r.fixed()?;
    let publisher_instance = r.fixed()?;
    let publisher_pid = r.u32()?;
    let publisher_creation = r.u64()?;
    let publisher_epoch = r.fixed()?;
    let service_epoch = r.fixed()?;
    let observer_epoch = r.fixed()?;
    let credential_epoch = r.fixed()?;
    let service_name = r.name()?;
    let observer_name = r.name()?;
    let service_sid = r.sid()?;
    let observer_sid = r.sid()?;
    let service_pid = r.u32()?;
    let service_creation = r.u64()?;
    let observer_pid = r.u32()?;
    let observer_creation = r.u64()?;
    let service_plan = r.fixed()?;
    let observer_plan = r.fixed()?;
    let service_artifact = r.fixed()?;
    let observer_artifact = r.fixed()?;
    let claims = Prepared {
        delegation,
        scope,
        installation,
        publisher_key,
        grant,
        publisher_instance,
        publisher_pid,
        publisher_creation,
        publisher_epoch,
        credential_epoch,
        service: RoleData {
            role: AudienceRole::Service,
            name: service_name,
            sid: service_sid,
            pid: service_pid,
            creation: service_creation,
            epoch: service_epoch,
            base_plan: service_plan,
            artifact: service_artifact,
        },
        observer: RoleData {
            role: AudienceRole::Observer,
            name: observer_name,
            sid: observer_sid,
            pid: observer_pid,
            creation: observer_creation,
            epoch: observer_epoch,
            base_plan: observer_plan,
            artifact: observer_artifact,
        },
        issued: r.u64()?,
        not_after: r.u64()?,
        cap: r.u32()?,
    };
    r.flags()?;
    if publisher_pid == 0
        || publisher_creation == 0
        || service_pid == 0
        || observer_pid == 0
        || service_creation == 0
        || observer_creation == 0
        || !binding::nonzero(&grant)
        || [
            publisher_epoch,
            service_epoch,
            observer_epoch,
            credential_epoch,
        ]
        .iter()
        .any(|e| !binding::nonzero(e))
    {
        return Err(AuthorizationError::Encoding);
    }
    Ok(claims)
}
fn commit_body(r: &mut Reader<'_>) -> Result<Commit> {
    let claims = Commit {
        delegation: r.fixed()?,
        grant: r.fixed()?,
        prepared: r.fixed()?,
        publisher_key: r.fixed()?,
        service_epoch: r.fixed()?,
        observer_epoch: r.fixed()?,
        credential_epoch: r.fixed()?,
        issued: r.u64()?,
        not_after: r.u64()?,
        cap: r.u32()?,
    };
    r.flags()?;
    if r.byte()? != 3
        || !binding::nonzero(&claims.grant)
        || [
            claims.service_epoch,
            claims.observer_epoch,
            claims.credential_epoch,
        ]
        .iter()
        .any(|e| !binding::nonzero(e))
    {
        return Err(AuthorizationError::Encoding);
    }
    Ok(claims)
}
