# Candidate component, not a qualified replacement for legacy core/gate images.
FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS compiler
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
WORKDIR /source
RUN cargo install --locked --version 0.7.7 --root /opt/cargo-auditable cargo-auditable
COPY . .
RUN PATH="/opt/cargo-auditable/bin:${PATH}" cargo auditable build --release --locked -p humanymous-runtime --no-default-features --features gateway --bin humanymous-runtime
# Empty state directory for the manifests' hmn-data volume. Docker copies its
# owner and mode into a new named volume on first mount; the runtime refuses
# group- or world-accessible audit and custody directories.
RUN ["mkdir","-p","/empty/state/humanymous"]
RUN ["chmod","0700","/empty/state/humanymous"]

FROM scratch AS runtime-candidate
ARG HUMANYMOUS_SOURCE_REV
ARG HUMANYMOUS_SOURCE_MANIFEST
LABEL org.opencontainers.image.source="https://github.com/modootoday/humanymous" \
      org.opencontainers.image.revision=$HUMANYMOUS_SOURCE_REV \
      org.humanymous.source-manifest-sha256=$HUMANYMOUS_SOURCE_MANIFEST \
      org.humanymous.qualification="candidate-component" \
      org.humanymous.role="runtime-private-pipes"
COPY --from=compiler /source/target/release/humanymous-runtime /usr/local/bin/humanymous-runtime
COPY --from=compiler /source/LICENSE /usr/share/licenses/humanymous/LICENSE
COPY --from=compiler /source/crates/sdk/humanymous-runtime/src/audit/LICENSE-Go.txt /usr/share/licenses/humanymous/Go.txt
COPY --from=compiler --chown=65532:65532 /empty/state/ /var/lib/
# /var/lib/humanymous is empty: audit/ and keys/ custody and the private
# configuration pipe are supplied by the owning host.
# The binary has no public config argv, secret environment or default listener.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime"]
