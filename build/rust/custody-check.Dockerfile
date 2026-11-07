# Deterministic ownership regression; never a product image or public service.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd
WORKDIR /lab
COPY . .
RUN rustup component add clippy && cargo clippy --locked -p humanymous-runtime --no-default-features --features audit,authority --all-targets -- -D warnings
RUN cargo test --locked -p humanymous-runtime --no-default-features --features audit,authority --lib --no-run && mkdir -p /usr/local/libexec && find target/debug/deps -maxdepth 1 -type f -executable -name 'humanymous_runtime-*' -exec cp '{}' /usr/local/libexec/runtime-context-tests ';'
# Keep the two existing lab entrypoints scoped to their original owner contexts.
RUN printf '%s\n' '#!/bin/sh' 'exec /usr/local/libexec/runtime-context-tests audit:: "$@"' > /usr/local/bin/audit-tests && printf '%s\n' '#!/bin/sh' 'exec /usr/local/libexec/runtime-context-tests authority:: "$@"' > /usr/local/bin/authority-tests && chmod 755 /usr/local/bin/audit-tests /usr/local/bin/authority-tests
RUN cargo test --locked -p humanymous-runtime --no-default-features --features audit --test audit-key-custody --no-run && find target/debug/deps -maxdepth 1 -type f -executable -name 'audit_key_custody-*' -exec cp '{}' /usr/local/bin/custody-tests ';'
RUN cargo test --locked -p humanymous-runtime --no-default-features --features authority --test authority-native-process --no-run && find target/debug/deps -maxdepth 1 -type f -executable -name 'authority_native_process-*' -exec cp '{}' /usr/local/bin/authority-native-tests ';'
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/audit-tests"]
