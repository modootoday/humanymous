# Unlock B Cap topology — W2 H2 product-binding (original_h2 serve path).
# Distinct from capture observe_h2. Component Cap green != pass_product_clean_room.
# Lab-only integration image. Detection freeze. CARGO_BUILD_JOBS=1.
# Cap pilot oneshot: prove gateway runtime binary is Cap-bound; do not invent
# readiness original_h2:true or pass_product_clean_room from composition alone.
# network_mode=none composition Cap never measures hosted original_h2 (N06 join
# still requires Root-hosted TLS listen+client pilot — see gateway contract).
FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS compiler
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p humanymous-runtime --no-default-features --features gateway --bin humanymous-runtime

FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383
COPY --from=compiler /src/target/release/humanymous-runtime /usr/local/bin/humanymous-runtime
COPY deployments/lab/gateway/h2-product-binding-contract.json /opt/lab/h2-product-binding-contract.json
RUN printf '%s\n' \
  '#!/bin/sh' \
  'set -eu' \
  'test -x /usr/local/bin/humanymous-runtime' \
  'test -f /opt/lab/h2-product-binding-contract.json' \
  'printf "%s\n" "{\"schema\":\"humanymous.lab-h2-product-binding-cap-oneshot/1\",\"kind\":\"w2-h2-product-binding-component\",\"product\":0,\"pass_product_clean_room\":0,\"original_h2_measured\":false,\"note\":\"composition Cap oneshot only; never invent readiness original_h2 or pass_product_clean_room\"}"' \
  'exit 0' \
  > /opt/lab/cap-pilot.sh \
  && chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
