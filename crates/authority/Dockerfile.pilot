# Isolated authority contract/security pilot, not a full Rust product runtime.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd
WORKDIR /lab
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY docs/tooling/ ./docs/tooling/
COPY scripts/ci/delivery/ ./scripts/ci/delivery/
COPY skills/agent-maintenance/ ./skills/agent-maintenance/
COPY test/lab/runner/ ./test/lab/runner/
COPY test/lab/runtime/ ./test/lab/runtime/
RUN cargo test --locked -p humanymous-authority --no-run
ENV CARGO_NET_OFFLINE=true
CMD ["cargo", "test", "--locked", "--offline", "-p", "humanymous-authority", "--", "--nocapture"]
