# Lab-only Rust executables; fixture reports never impersonate browser evidence.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd AS compiler
WORKDIR /source
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
COPY docs/tooling/ ./docs/tooling/
COPY test/lab/runner/ ./test/lab/runner/
RUN cargo build --release --locked -p humanymous-lab-runner

FROM compiler AS runtime-compiler
RUN cargo build --release --locked -p humanymous-runtime -p humanymous-cli
COPY test/lab/runtime/ ./test/lab/runtime/
RUN cargo build --release --locked --manifest-path test/lab/runtime/Cargo.toml

FROM debian:trixie-slim@sha256:a72b4433de1cf7d6bae0c4cee74d0547f58601a643b3375f070c51600d63b6b2 AS controls
COPY --from=compiler /source/target/release/humanymous-lab-runner /usr/local/bin/
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-lab-runner"]

FROM controls AS runtime
COPY --from=runtime-compiler /source/target/release/humanymous-runtime /usr/local/bin/
COPY --from=runtime-compiler /source/target/release/humanymous /usr/local/bin/
COPY --from=runtime-compiler /source/test/lab/runtime/target/release/humanymous-runtime-pilot /usr/local/bin/
