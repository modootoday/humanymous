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
FROM lwthiker/curl-impersonate:0.6-chrome AS curl-impersonate

# ---- Go attack + gate binaries ----------------------------------------
FROM golang:1.25 AS gobuild
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
 && go run ./scripts/gen-demo-keys

# ---- Bots runtime: Node + Playwright browsers (Chromium + Firefox) ---------
FROM mcr.microsoft.com/playwright:v1.61.1-noble
WORKDIR /app

COPY --from=gobuild /out/redteam   /app/bin/redteam
COPY --from=gobuild /out/tlsparrot /app/bin/tlsparrot
COPY --from=gobuild /out/gate  /app/bin/gate

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

COPY test/package.json test/package-lock.json /app/test/
RUN cd /app/test && npm ci --no-audit --no-fund
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
