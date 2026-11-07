# Lab-only integration image. Runtime contains no Go or attack catalog.
FROM rust:1.99.1-alpine@sha256:44819b27be4c97b0e4d7203bd2e02c4820f97700211b6a37fb4c246a2de59383 AS compiler
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p humanymous-runtime --no-default-features --features authority --bin humanymous-authority

FROM node:26-alpine@sha256:8996bb5821d62a8fd9bb27c1cb07462f3fdce6e47d640c6a881ead0b064ce216 AS node-dependencies
WORKDIR /opt/adapter
COPY deployments/lab/authority/package.json deployments/lab/authority/package-lock.json ./
RUN npm ci --omit=dev --ignore-scripts --no-audit --no-fund
COPY adapters/node/src/ ./src/

FROM node:26-alpine@sha256:8996bb5821d62a8fd9bb27c1cb07462f3fdce6e47d640c6a881ead0b064ce216
RUN rm -rf /usr/local/lib/node_modules /opt/yarn* /usr/local/bin/npm /usr/local/bin/npx /usr/local/bin/yarn /usr/local/bin/yarnpkg /usr/local/bin/corepack
COPY --from=compiler /src/target/release/humanymous-authority /usr/local/bin/
COPY --from=node-dependencies /opt/adapter /opt/adapter
COPY deployments/lab/authority/ /opt/lab/
USER 65532:65532
ENTRYPOINT ["node", "/opt/lab/pilot.mjs"]
