# Candidate component, not a qualified replacement for legacy core/gate images.
FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS compiler
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
WORKDIR /source
RUN cargo install --locked --version 0.7.7 --root /opt/cargo-auditable cargo-auditable
COPY . .
RUN PATH="/opt/cargo-auditable/bin:${PATH}" cargo auditable build --release --locked -p humanymous-runtime --bin humanymous-runtime

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
COPY --from=compiler /source/crates/audit/LICENSE-Go.txt /usr/share/licenses/humanymous/Go.txt
# State/custody and private configuration pipes are supplied by the owning host.
# The binary has no public config argv, secret environment or default listener.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime"]
