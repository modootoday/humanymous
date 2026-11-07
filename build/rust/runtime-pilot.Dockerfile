# Lab only: real process pilot, no public ports or external requests.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd
WORKDIR /lab
COPY . .
RUN cargo build --locked -p humanymous-runtime -p humanymous-cli
RUN rustup component add clippy && cargo build --locked -p humanymous-runtime-pilot && cargo clippy --locked -p humanymous-runtime-pilot -- -D warnings
RUN cp target/debug/humanymous-runtime /usr/local/bin/humanymous-runtime && cp target/debug/humanymous /usr/local/bin/humanymous && cp target/debug/humanymous-runtime-pilot /usr/local/bin/humanymous-runtime-pilot
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot"]
