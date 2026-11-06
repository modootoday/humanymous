# Local lab only. This image must never be published as a product runtime.
FROM rust:1.88-bookworm AS build
WORKDIR /lab
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
RUN cargo build --locked --release -p humanymous-attacker
FROM debian:bookworm-slim
COPY --from=build /lab/target/release/humanymous-attacker /usr/local/bin/humanymous-attacker
USER 65534:65534
ENTRYPOINT ["/usr/local/bin/humanymous-attacker"]
CMD ["--seed", "42"]
