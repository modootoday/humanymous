# Lab only: real process pilot, no public ports or external requests.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd
WORKDIR /lab
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo build --locked -p humanymous-runtime
COPY test/lab/runtime/ ./test/lab/runtime/
RUN rustup component add clippy && cargo build --locked --manifest-path test/lab/runtime/Cargo.toml && cargo clippy --locked --manifest-path test/lab/runtime/Cargo.toml -- -D warnings
RUN cp target/debug/humanymous-runtime /usr/local/bin/humanymous-runtime && cp test/lab/runtime/target/debug/humanymous-runtime-pilot /usr/local/bin/humanymous-runtime-pilot
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot"]
