# Lab-only Rust executables; fixture reports never impersonate browser evidence.
FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS compiler
WORKDIR /source
ENV CARGO_BUILD_JOBS=1
COPY . .
RUN cargo build --release --locked -p humanymous-lab-runner

FROM compiler AS runtime-compiler
RUN cargo build --release --locked -p humanymous-runtime -p humanymous-cli
RUN cargo build --release --locked --manifest-path test/lab/runtime/Cargo.toml
RUN cargo test --locked -p humanymous-runtime-pilot
RUN cargo clippy --locked -p humanymous-runtime-pilot --all-targets -- -D warnings
RUN cargo build --release --locked -p humanymous-runtime --no-default-features --features audit --bin audit-witness

FROM compiler AS custody-directories
RUN ["mkdir","-p","/empty/witness/keys","/empty/witness/head","/empty/writer/keys","/empty/writer/wal"]
RUN ["chmod","0700","/empty/witness/keys","/empty/witness/head","/empty/writer/keys","/empty/writer/wal"]

FROM scratch AS controls
COPY --from=compiler /source/target/release/humanymous-lab-runner /usr/local/bin/
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-lab-runner"]

FROM controls AS custody-layout
COPY --from=custody-directories --chown=65533:65533 /empty/witness/ /witness/
COPY --from=custody-directories --chown=65532:65532 /empty/writer/ /writer/

FROM custody-layout AS runtime
COPY --from=runtime-compiler /source/target/release/humanymous-runtime /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/humanymous /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/humanymous-runtime-pilot /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/audit-witness /usr/local/bin/
# Read-only legacy Pass page for the pass-pilot attest_floor recovery case.
COPY --from=compiler /source/web/pass.html /usr/local/share/humanymous/web/

# Roadmap 4 items 2h/2i: the image shape compose.release.yaml names
# (build/rust/runtime-candidate.Dockerfile): scratch, the runtime binary as
# entrypoint, UID 65532, no argv. Same binary bytes as the runtime stage.
# deployment-state is the empty-volume layout for the manifest's hmn-data
# volume: Docker copies its owner and mode into the volume on first mount, and
# the runner creates the runtime container before any lab actor mounts it.
FROM compiler AS deployment-state
RUN ["mkdir","-p","/empty/state/humanymous"]
RUN ["chmod","0700","/empty/state/humanymous"]

FROM scratch AS deployment-runtime
COPY --from=runtime-compiler /source/target/release/humanymous-runtime /usr/local/bin/humanymous-runtime
COPY --from=deployment-state --chown=65532:65532 /empty/state/ /var/lib/
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime"]
