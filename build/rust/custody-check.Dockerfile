# Deterministic ownership regression; never a product image or public service.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd
WORKDIR /lab
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN rustup component add clippy && cargo clippy --locked -p humanymous-audit -p humanymous-authority --all-targets -- -D warnings
RUN cargo test --locked -p humanymous-audit -p humanymous-authority --lib --no-run && find target/debug/deps -maxdepth 1 -type f -executable -name 'humanymous_audit-*' -exec cp '{}' /usr/local/bin/audit-tests ';' && find target/debug/deps -maxdepth 1 -type f -executable -name 'humanymous_authority-*' -exec cp '{}' /usr/local/bin/authority-tests ';'
RUN cargo test --locked -p humanymous-audit --test key_custody --no-run && find target/debug/deps -maxdepth 1 -type f -executable -name 'key_custody-*' -exec cp '{}' /usr/local/bin/custody-tests ';'
RUN cargo test --locked -p humanymous-authority --test native_process --no-run && find target/debug/deps -maxdepth 1 -type f -executable -name 'native_process-*' -exec cp '{}' /usr/local/bin/authority-native-tests ';'
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/audit-tests"]
