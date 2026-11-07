# Owned browser component tests only; Firefox is not the product UI engine.
FROM rust:1.99.1-slim-bookworm@sha256:f2c004e3d2d8b547e8cf7ffbc29205385f4e71d23e53e4e383065db5e9f822cd
RUN apt-get update && apt-get install -y --no-install-recommends firefox-esr curl ca-certificates && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://github.com/mozilla/geckodriver/releases/download/v0.37.1/geckodriver-v0.37.1-linux64.tar.gz -o /tmp/geckodriver.tar.gz && echo 'e815130ea95983e162ae91843b48d3a3ce991735635fce83a647afde21e09f7e  /tmp/geckodriver.tar.gz' | sha256sum -c - && tar -xzf /tmp/geckodriver.tar.gz -C /usr/local/bin && rm /tmp/geckodriver.tar.gz
RUN rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version 0.2.130 --locked
WORKDIR /lab
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo test -p humanymous-collector --target wasm32-unknown-unknown --locked --no-run && find target/wasm32-unknown-unknown/debug/deps -name 'humanymous_collector-*.wasm' -exec cp '{}' /usr/local/lib/collector-tests.wasm ';'
ENV HOME=/tmp/collector-home GECKODRIVER=/usr/local/bin/geckodriver WASM_BINDGEN_USE_BROWSER=1 WASM_BINDGEN_TEST_ONLY_WEB=1 WASM_BINDGEN_TEST_TIMEOUT=90
USER 65532:65532
ENTRYPOINT ["wasm-bindgen-test-runner", "/usr/local/lib/collector-tests.wasm"]
