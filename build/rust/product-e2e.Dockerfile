# Roadmap unit 35: real Rust product e2e (Gate + origin + attacker, a
# witness-required Gate with an independent audit witness container, and a
# real browser running the Rust WASM collector against a witnessed Gate).
# Builds the actual humanymous-runtime gateway and the Rust lab actors from source.
# Lab-only images; no shell entrypoint, no published ports, detection freeze.
# A partial topology: authority is absent.
#
# Targets: the default (last) stage is the Gate/origin/driver/witness image.
# `--target browser` is the separate browser actor image: Firefox ESR and
# geckodriver driven by the Rust wasm-bindgen-test-runner, which loads the
# feature-gated humanymous-collector WASM test build.
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

# The real Rust WASM collector, compiled as its feature-gated runtime pilot test.
FROM compiler AS wasm-compiler
RUN --mount=type=secret,id=egress_ca,target=/etc/ssl/certs/ca-certificates.crt,required=false \
    rustup target add wasm32-unknown-unknown && cargo fetch --locked --target wasm32-unknown-unknown
RUN cargo test --locked --offline -p humanymous-collector --features runtime-pilot \
    --target wasm32-unknown-unknown --no-run

# Pinned browser tooling: the Rust wasm-bindgen test runner and geckodriver.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd AS browser-tools
ENV CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
RUN --mount=type=secret,id=egress_ca,target=/etc/ssl/certs/ca-certificates.crt,required=false,mode=0444 \
    sed -i s,http://deb.debian.org,https://deb.debian.org,g /etc/apt/sources.list.d/debian.sources \
    && apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*
RUN --mount=type=secret,id=egress_ca,target=/etc/ssl/certs/ca-certificates.crt,required=false \
    curl -fsSL https://github.com/mozilla/geckodriver/releases/download/v0.37.1/geckodriver-v0.37.1-linux64.tar.gz -o /tmp/geckodriver.tar.gz \
    && echo 'e815130ea95983e162ae91843b48d3a3ce991735635fce83a647afde21e09f7e  /tmp/geckodriver.tar.gz' | sha256sum -c - \
    && tar -xzf /tmp/geckodriver.tar.gz -C /usr/local/bin && rm /tmp/geckodriver.tar.gz
RUN --mount=type=secret,id=egress_ca,target=/etc/ssl/certs/ca-certificates.crt,required=false \
    cargo install wasm-bindgen-cli --version 0.2.130 --locked --root /opt/wasm-bindgen

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc AS browser
RUN --mount=type=secret,id=egress_ca,target=/etc/ssl/certs/ca-certificates.crt,required=false,mode=0444 \
    sed -i s,http://deb.debian.org,https://deb.debian.org,g /etc/apt/sources.list.d/debian.sources \
    && apt-get update && apt-get install -y --no-install-recommends firefox-esr && rm -rf /var/lib/apt/lists/*
COPY --from=browser-tools /usr/local/bin/geckodriver /usr/local/bin/geckodriver
COPY --from=browser-tools /opt/wasm-bindgen/bin/wasm-bindgen-test-runner /usr/local/bin/wasm-bindgen-test-runner
# A missing or ambiguous wildcard fails the build instead of choosing an artifact.
COPY --from=wasm-compiler /src/target/wasm32-unknown-unknown/debug/deps/humanymous_collector-*.wasm /usr/local/lib/collector-product-tests.wasm
COPY --from=wasm-compiler /src/crates/collector/pilot/webdriver.json /usr/local/lib/browser-pilot-webdriver.json
COPY --from=compiler /src/target/release/humanymous-runtime-pilot /usr/local/bin/humanymous-runtime-pilot
LABEL org.humanymous.owner=product-e2e org.humanymous.scope=partial-topology org.humanymous.role=browser
ENV HOME=/tmp/collector-home GECKODRIVER=/usr/local/bin/geckodriver WASM_BINDGEN_USE_BROWSER=1 WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_TEST_TIMEOUT=90
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot"]

FROM debian:bookworm-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc
COPY --from=compiler /src/target/release/humanymous-runtime /usr/local/bin/humanymous-runtime
COPY --from=compiler /src/target/release/humanymous-runtime-pilot /usr/local/bin/humanymous-runtime-pilot
LABEL org.humanymous.owner=product-e2e org.humanymous.scope=partial-topology
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot"]
