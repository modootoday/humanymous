# Owned product browser pilot; separate from the collector component denominator.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd AS browser-tools
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
RUN apt-get update && apt-get install -y --no-install-recommends firefox-esr curl ca-certificates && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://github.com/mozilla/geckodriver/releases/download/v0.37.1/geckodriver-v0.37.1-linux64.tar.gz -o /tmp/geckodriver.tar.gz && echo 'e815130ea95983e162ae91843b48d3a3ce991735635fce83a647afde21e09f7e  /tmp/geckodriver.tar.gz' | sha256sum -c - && tar -xzf /tmp/geckodriver.tar.gz -C /usr/local/bin && rm /tmp/geckodriver.tar.gz
RUN rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version 0.2.130 --locked

FROM browser-tools AS wasm-compiler
WORKDIR /lab
COPY . .
# Read-only original differential oracles remain distinct from new Rust tests.
RUN cargo test --locked -p humanymous-collector --features runtime-pilot --target wasm32-unknown-unknown --no-run

FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS driver-compiler
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
WORKDIR /source
COPY . .
RUN cargo build --release --locked -p humanymous-runtime-pilot

FROM browser-tools AS browser-product
# A missing or ambiguous wildcard fails the build instead of choosing an artifact.
COPY --from=wasm-compiler /lab/target/wasm32-unknown-unknown/debug/deps/humanymous_collector-*.wasm /usr/local/lib/collector-product-tests.wasm
COPY --from=wasm-compiler /lab/crates/collector/pilot/webdriver.json /usr/local/lib/browser-pilot-webdriver.json
COPY --from=driver-compiler /source/target/release/humanymous-runtime-pilot /usr/local/bin/humanymous-runtime-pilot
ENV HOME=/tmp/collector-home GECKODRIVER=/usr/local/bin/geckodriver WASM_BINDGEN_USE_BROWSER=1 WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_TEST_TIMEOUT=90
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot", "--browser-driver"]
