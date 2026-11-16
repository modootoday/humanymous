# Roadmap unit 35, first increment: real Rust product e2e (Gate + origin + attacker).
# Builds the actual humanymous-runtime gateway and the Rust lab actors from source.
# Lab-only image; no shell entrypoint, no published ports, detection freeze.
# A partial topology: authority, independent audit service and browser are absent.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd AS compiler
ENV CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
WORKDIR /src
COPY . .
# Optional build-time egress trust anchor for TLS-intercepting venues. When the
# secret is absent the image's own certificate store is used unchanged.
RUN --mount=type=secret,id=egress_ca,target=/etc/ssl/certs/ca-certificates.crt,required=false \
    cargo fetch --locked --target x86_64-unknown-linux-gnu
RUN cargo build --release --locked --offline \
    -p humanymous-runtime -p humanymous-runtime-pilot \
    --bin humanymous-runtime --bin humanymous-runtime-pilot

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
COPY --from=compiler /src/target/release/humanymous-runtime /usr/local/bin/humanymous-runtime
COPY --from=compiler /src/target/release/humanymous-runtime-pilot /usr/local/bin/humanymous-runtime-pilot
LABEL org.humanymous.owner=product-e2e org.humanymous.scope=partial-topology
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot"]
