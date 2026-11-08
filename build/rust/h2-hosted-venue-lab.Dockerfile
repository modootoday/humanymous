# N06 hosted-venue Cap image — Specfold/github-hosted Linux only.
# Reuses Desktop measure binary; venue marker must remain hosted (never desktop_as_hosted).
# Detection freeze. CARGO_BUILD_JOBS=1.
FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS compiler
ENV CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p humanymous-runtime --no-default-features --features gateway,hosted-pilot --bin humanymous-h2-product-binding-measure

FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383
COPY --from=compiler /src/target/release/humanymous-h2-product-binding-measure /usr/local/bin/humanymous-h2-product-binding-measure
COPY deployments/lab/gateway/h2-product-binding-contract.json /opt/lab/h2-product-binding-contract.json
ENV HUMANYMOUS_H2_MEASURE_VENUE=hosted_linux_specfold
RUN printf '%s\n' \
  '#!/bin/sh' \
  'set -eu' \
  'test -x /usr/local/bin/humanymous-h2-product-binding-measure' \
  'test -f /opt/lab/h2-product-binding-contract.json' \
  'test "${HUMANYMOUS_H2_MEASURE_VENUE}" = "hosted_linux_specfold"' \
  'exec /usr/local/bin/humanymous-h2-product-binding-measure' \
  > /opt/lab/cap-pilot.sh \
  && chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
