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
RUN cargo build --release --locked -p humanymous-audit --bin audit-witness

FROM compiler AS custody-directories
RUN ["mkdir","-p","/empty/witness-keys","/empty/witness-head","/empty/writer-keys","/empty/writer-wal"]
RUN ["chmod","0700","/empty/witness-keys","/empty/witness-head","/empty/writer-keys","/empty/writer-wal"]

FROM scratch AS controls
COPY --from=compiler /source/target/release/humanymous-lab-runner /usr/local/bin/
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-lab-runner"]

FROM controls AS custody-layout
COPY --from=custody-directories --chown=65533:65533 --chmod=0700 /empty/witness-keys /witness/keys
COPY --from=custody-directories --chown=65533:65533 --chmod=0700 /empty/witness-head /witness/head
COPY --from=custody-directories --chown=65532:65532 --chmod=0700 /empty/writer-keys /writer/keys
COPY --from=custody-directories --chown=65532:65532 --chmod=0700 /empty/writer-wal /writer/wal

FROM custody-layout AS runtime
COPY --from=runtime-compiler /source/target/release/humanymous-runtime /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/humanymous /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/humanymous-runtime-pilot /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/audit-witness /usr/local/bin/
