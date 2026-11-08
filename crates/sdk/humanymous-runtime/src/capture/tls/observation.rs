//! Same-owner projection only. No DATA constructor can mint the borrowed view.
use super::Connection;
use crate::{
    capture::http2::{InputState, StreamMetadata},
    network::{
        http::HeaderInfo,
        http2::Fingerprint,
        ingress::{H2Claim, Refusal, project_h2},
        tls::ClientHello,
    },
};

/// Historical DATA snapshot. It is not a native proof, live lease, complete
/// application request or original-hop attestation; operator intelligence is absent.
pub struct IngressSnapshot {
    /// Observations from the actual same TLS owner.
    pub hello: ClientHello,
    /// Actual negotiated ALPN; never the first offered protocol.
    pub negotiated_alpn: Vec<u8>,
    /// Actual initial-header fingerprint for the selected stream.
    pub h2: Fingerprint,
    /// Raw field order from that owner's validated decoded header section.
    pub header: HeaderInfo,
    /// Selected client stream on the same connection.
    pub stream: u32,
    /// Unpadded captured DATA count, not an application body digest.
    pub data_bytes: u64,
    /// Frozen monitor output only, without new predicates or scoring.
    pub abuse: Option<&'static str>,
}

/// Borrowed observation view, constructed only from the actual live connection.
/// No Clone/Deserialize/From<DATA> or public field constructor exists. Cancellation
/// remains possible through its supervisor handle; each snapshot rechecks liveness.
///
/// Historical DATA cannot be promoted into this live view:
/// ```compile_fail
/// use humanymous_runtime::capture::tls::{H2HeadersObservation, IngressSnapshot};
/// fn promote(data: IngressSnapshot) -> H2HeadersObservation<'static> { data.into() }
/// ```
/// Sibling/external callers cannot bypass validation through field construction:
/// ```compile_fail
/// use humanymous_runtime::capture::tls::{Connection, H2HeadersObservation};
/// fn bypass(owner: &Connection) -> H2HeadersObservation<'_> {
///     H2HeadersObservation { owner, stream: 1, body_bytes: 0, header: Default::default() }
/// }
/// ```
pub struct H2HeadersObservation<'a> {
    owner: &'a Connection,
    stream: u32,
    body_bytes: u64,
    header: HeaderInfo,
}

impl Connection {
    /// Match application claims against this owner's completed captured stream.
    /// This does not validate application body contents, flow control, policy,
    /// session authority or origin effects. H2 serving remains a separate gate.
    pub fn observe_h2(&self, claim: &H2Claim<'_>) -> Result<H2HeadersObservation<'_>, Refusal> {
        self.remaining().map_err(|_| Refusal::OwnerUnavailable)?;
        let metadata = self.h2_stream_metadata(claim.stream);
        validate_state(
            self.negotiated_alpn(),
            metadata,
            claim.stream,
            claim.body_bytes,
        )?;
        let request = self
            .h2_request(claim.stream)
            .ok_or(Refusal::MissingStream)?;
        let mut header = project_h2(request.fields(), claim)?;
        // These flags are established HERE from the same actual capture owner,
        // never by the pure DATA projector or supplied caller booleans.
        header.order_reliable = true;
        header.casing_reliable = true;
        self.remaining().map_err(|_| Refusal::OwnerUnavailable)?;
        Ok(H2HeadersObservation {
            owner: self,
            stream: claim.stream,
            body_bytes: claim.body_bytes,
            header,
        })
    }
}

impl H2HeadersObservation<'_> {
    /// Release a point-in-time DATA snapshot; it must never be accepted back as
    /// a live proof. No inferred healthy operator/TCP/collector planes are filled.
    /// This checks known failure/revocation/expiry without I/O; unread peer FIN,
    /// close-notify or GOAWAY is not a remote-liveness/idle qualification.
    pub fn snapshot(&self) -> Result<IngressSnapshot, Refusal> {
        self.owner
            .remaining()
            .map_err(|_| Refusal::OwnerUnavailable)?;
        validate_state(
            self.owner.negotiated_alpn(),
            self.owner.h2_stream_metadata(self.stream),
            self.stream,
            self.body_bytes,
        )?;
        let hello = self
            .owner
            .client_hello()
            .ok_or(Refusal::OwnerUnavailable)?
            .clone();
        let request = self
            .owner
            .h2_request(self.stream)
            .ok_or(Refusal::MissingStream)?;
        let snapshot = IngressSnapshot {
            hello,
            negotiated_alpn: b"h2".to_vec(),
            h2: request.fingerprint().clone(),
            header: self.header.clone(),
            stream: self.stream,
            data_bytes: self.body_bytes,
            abuse: self.owner.h2_abuse(),
        };
        self.owner
            .remaining()
            .map_err(|_| Refusal::OwnerUnavailable)?;
        Ok(snapshot)
    }
}

fn validate_state(
    alpn: Option<&[u8]>,
    metadata: Option<StreamMetadata>,
    stream: u32,
    body_bytes: u64,
) -> Result<(), Refusal> {
    if alpn != Some(b"h2".as_slice()) {
        return Err(Refusal::Protocol);
    }
    if stream == 0 || stream.is_multiple_of(2) {
        return Err(Refusal::MissingStream);
    }
    let metadata = metadata.ok_or(Refusal::MissingStream)?;
    if metadata.input != InputState::Ended {
        return Err(Refusal::IncompleteStream);
    }
    if metadata.trailer_fields.is_some() {
        return Err(Refusal::TrailersUnsupported);
    }
    if metadata.data_bytes != body_bytes {
        return Err(Refusal::BodyLength);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn metadata(input: InputState) -> StreamMetadata {
        StreamMetadata {
            input,
            data_bytes: 3,
            trailer_fields: None,
        }
    }
    #[test]
    fn absent_or_other_actual_alpn_never_substitutes_an_offer() {
        for alpn in [
            None,
            Some(b"http/1.1".as_slice()),
            Some(b"h2,http/1.1".as_slice()),
        ] {
            assert_eq!(
                validate_state(alpn, Some(metadata(InputState::Ended)), 1, 3),
                Err(Refusal::Protocol)
            );
        }
    }
    #[test]
    fn missing_unknown_or_incomplete_stream_is_not_complete() {
        assert_eq!(
            validate_state(Some(b"h2"), None, 1, 3),
            Err(Refusal::MissingStream)
        );
        for input in [
            InputState::HeadersPending,
            InputState::Receiving,
            InputState::Reset,
        ] {
            assert_eq!(
                validate_state(Some(b"h2"), Some(metadata(input)), 1, 3),
                Err(Refusal::IncompleteStream)
            );
        }
        for stream in [0, 2] {
            assert_eq!(
                validate_state(Some(b"h2"), Some(metadata(InputState::Ended)), stream, 3),
                Err(Refusal::MissingStream)
            );
        }
    }
    #[test]
    fn trailer_or_body_count_ambiguity_refuses() {
        let mut meta = metadata(InputState::Ended);
        assert_eq!(
            validate_state(Some(b"h2"), Some(meta), 1, 4),
            Err(Refusal::BodyLength)
        );
        meta.trailer_fields = Some(0);
        assert_eq!(
            validate_state(Some(b"h2"), Some(meta), 1, 3),
            Err(Refusal::TrailersUnsupported)
        );
    }
    #[test]
    fn complete_state_only_passes_the_pure_predicate_not_owner_construction() {
        assert_eq!(
            validate_state(Some(b"h2"), Some(metadata(InputState::Ended)), 1, 3),
            Ok(())
        );
        // No Connection or H2HeadersObservation is constructed by this fixture.
    }
}
