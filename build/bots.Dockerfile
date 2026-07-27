# Bots image — the automation catalog (SoT-04) + the Go protocol attack binaries
# (cmd/redteam uTLS/RIT, cmd/tlsparrot) + curl-impersonate (Chrome TLS/JA4 parrot).
# Also carries the Gate binary so the self-contained Gate conformance e2e can run
# on loopback.
#
# LOCAL TARGET ONLY. In docker-compose this container is attached ONLY to the
# internal `lab` network (no internet route), so it can physically reach nothing
# but the detector — the defensive mandate enforced at the network layer, not just
# by convention.
#
# Build context = repo root; the adjacent bots.Dockerfile.dockerignore re-includes
# cmd/redteam, cmd/tlsparrot, cmd/gate and test/.

# ---- curl-impersonate (Chrome ClientHello / JA4 parrot; musl) ---------------
# Upstream image is Alpine/musl. We copy binaries + musl loader + libz so the
# wrappers run on the Ubuntu Playwright base via the musl dynamic linker.
FROM lwthiker/curl-impersonate:0.6-chrome@sha256:4039d9d38b0182dc2b8550b1320da2fd0e7b9720f21f299fa0e40cbfba95cc64 AS curl-impersonate

# ---- Go attack + gate binaries ----------------------------------------
FROM golang:1.25-alpine@sha256:56961d79ea8129efddcc0b8643fd8a5416b4e6228cfd477e3fd61deb2672c587 AS gobuild
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY cmd ./cmd
COPY internal ./internal
COPY pkg ./pkg
COPY scripts/gen-demo-keys ./scripts/gen-demo-keys
RUN CGO_ENABLED=0 GOOS=linux go build -trimpath -o /out/redteam   ./cmd/redteam/ \
 && CGO_ENABLED=0 GOOS=linux go build -trimpath -o /out/tlsparrot ./cmd/tlsparrot/ \
 && CGO_ENABLED=0 GOOS=linux go build -trimpath -o /out/gate  ./cmd/gate/ \
 && CGO_ENABLED=0 go run ./scripts/gen-demo-keys

# ---- Bots runtime: Node + only the two exercised Playwright browsers --------
# The upstream Playwright image also carries WebKit and its OS dependencies.
# Install from the repository lock instead so this local-only image contains the
# headful Chromium and Firefox builds exercised by the catalog, and nothing else.
FROM node:22-bookworm-slim@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3
WORKDIR /app

ARG DEBIAN_FRONTEND=noninteractive
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright

# Copy the Go outputs before downloading browsers. This dependency makes
# BuildKit finish and release the Go compilation peak before the large
# Playwright extraction starts on 14 GiB standard runners.
COPY --from=gobuild /out/redteam   /app/bin/redteam
COPY --from=gobuild /out/tlsparrot /app/bin/tlsparrot
COPY --from=gobuild /out/gate      /app/bin/gate

# The catalog uses new Chromium headless plus full headful Chromium. It never
# launches the legacy headless-shell binary, so do not download that duplicate.
COPY test/package.json test/package-lock.json /app/test/
RUN dpkg-divert --local --rename --add /usr/bin/update-mime-database \
 && ln -s /bin/true /usr/bin/update-mime-database \
 && apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      fonts-dejavu-core \
      fonts-liberation \
      libasound2 \
      libatk-bridge2.0-0 \
      libatk1.0-0 \
      libcairo2 \
      libcups2 \
      libdbus-1-3 \
      libdbus-glib-1-2 \
      libdrm2 \
      libgbm1 \
      libglib2.0-0 \
      libgtk-3-0 \
      libnspr4 \
      libnss3 \
      libpango-1.0-0 \
      libx11-6 \
      libx11-xcb1 \
      libxcb1 \
      libxcomposite1 \
      libxdamage1 \
      libxext6 \
      libxfixes3 \
      libxkbcommon0 \
      libxrandr2 \
      libxshmfence1 \
      libxt6 \
      libxtst6 \
      xauth \
      xvfb \
 && cd /app/test \
 && npm ci --no-audit --no-fund \
 && ./node_modules/.bin/playwright-core install chromium firefox \
 && rm /usr/bin/update-mime-database \
 && dpkg-divert --local --rename --remove /usr/bin/update-mime-database \
 && rm -rf \
      /root/.npm \
      /tmp/* \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*

# curl-impersonate from lwthiker/curl-impersonate:0.6-chrome
# User pattern: copy curl-impersonate, curl-impersonate-chrome, curl_chrome99_android
# (+ chrome116 wrapper for desktop profile) and full /usr/local/lib (libcurl-impersonate
# + BoringSSL-linked shared objects). Binaries are musl/Alpine; ship musl loader + zlib.
RUN mkdir -p /app/curl-impersonate/bin /app/curl-impersonate/lib /lib
COPY --from=curl-impersonate \
    /usr/local/bin/curl-impersonate \
    /usr/local/bin/curl-impersonate-chrome \
    /usr/local/bin/curl_chrome99_android \
    /usr/local/bin/curl_chrome116 \
    /app/curl-impersonate/bin/
# Full dedicated libcurl/BoringSSL tree from the upstream image
COPY --from=curl-impersonate /usr/local/lib/ /app/curl-impersonate/lib/
COPY --from=curl-impersonate /lib/ld-musl-x86_64.so.1 /lib/ld-musl-x86_64.so.1
COPY --from=curl-impersonate /lib/libz.so.1.2.13 /lib/libz.so.1.2.13
RUN ln -sf libz.so.1.2.13 /lib/libz.so.1 \
 && ln -sf ld-musl-x86_64.so.1 /lib/libc.musl-x86_64.so.1 \
 && sed -i '1s|^#!.*|#!/bin/bash|' /app/curl-impersonate/bin/curl_chrome* \
 && chmod +x /app/curl-impersonate/bin/*

COPY test /app/test
COPY scripts/assert-*.mjs /app/scripts/
COPY deployments/bots/ /app/
# Seed lives outside the runtime mount point so an empty named volume cannot hide keys.
COPY --from=gobuild /src/deployments/patissuers /app/demo-keys-seed/patissuers
COPY --from=gobuild /src/deployments/webauthncreds /app/demo-keys-seed/webauthncreds
# CRLF fix for shell only — never sed the musl ELF binaries (corrupts them → segfault).
RUN sed -i 's/\r$//' /app/*.sh /app/curl-impersonate/bin/curl_chrome* \
 && chmod +x /app/*.sh /app/curl-impersonate/bin/*

ENV HM_REDTEAM_BIN=/app/bin/redteam \
    HM_TLSPARROT_BIN=/app/bin/tlsparrot \
    HM_CURL_IMPERSONATE_DIR=/app/curl-impersonate/bin \
    HM_CURL_IMPERSONATE_LIB=/app/curl-impersonate/lib \
    HM_BROWSER_CHANNEL=chromium \
    HM_LAUNCH_ARGS=--no-sandbox,--disable-dev-shm-usage \
    NODE_TLS_REJECT_UNAUTHORIZED=0 \
    LD_LIBRARY_PATH=/app/curl-impersonate/lib

# Default job: run the automation catalog against the detector engine. Overridable
# in compose (e.g. the gate-e2e service runs run-gate-e2e.sh instead).
ENTRYPOINT ["/app/run-attack.sh"]
