# Public demo image — the detection engine ONLY (SoT-30 §11 safety gate).
# The build context is trimmed by .dockerignore to exclude the dev playground,
# red-team tooling, docs, and design resources. The engine terminates its own
# TLS (to read JA3/JA4 + the HTTP/2 fingerprint), so it must bind :443 directly
# on a plain VPS — never behind a TLS-terminating proxy/load balancer.

# ---- builder ---------------------------------------------------------------
# Pin the builder to the BUILD platform and cross-compile to the TARGET arch, so
# multi-arch (amd64+arm64) release builds compile natively instead of emulating
# under QEMU (SoT-31 R6). The binary is static (CGO off), so this is free.
FROM --platform=$BUILDPLATFORM golang:1.25 AS builder
ARG TARGETOS
ARG TARGETARCH
WORKDIR /src

# Cache modules first.
COPY go.mod go.sum ./
RUN go mod download

# Source (the .dockerignore keeps dev-process + red-team paths out of context).
COPY . .

# Build the browser detector (WASM, arch-independent) + copy Go's loader shim,
# then cross-compile the server for the target arch.
RUN GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/ \
 && cp "$(go env GOROOT)/lib/wasm/wasm_exec.js" web/js/wasm_exec.js
ARG VERSION=dev
RUN CGO_ENABLED=0 GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH} go build -trimpath -ldflags="-s -w -X main.version=${VERSION}" -o /out/server ./cmd/server/

# Pre-create the ACME cache dir owned by the non-root runtime user so autocert
# can persist issued certs + the account key across restarts (mount a volume here).
RUN mkdir -p /acme-cache && chown 65532:65532 /acme-cache

# ---- runtime ---------------------------------------------------------------
# distroless/static: no shell, no package manager, minimal attack surface. It
# ships CA roots (needed for the Let's Encrypt ACME calls) and runs as nonroot.
FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app

COPY --from=builder /out/server /app/server
COPY --from=builder /src/web /app/web
# Ship licence + third-party notices with the image (audit: BSD-3/MIT redistribution).
COPY --from=builder /src/LICENSE /src/NOTICE /src/THIRD_PARTY_LICENSES.md /app/
COPY --from=builder --chown=65532:65532 /acme-cache /acme-cache

VOLUME ["/acme-cache"]
EXPOSE 443
USER 65532:65532

# HMN_PLAYGROUND is intentionally unset → the dev observatory + local Red launcher
# are never registered in this image. Provide the public domain at run time, e.g.:
#   docker run -p 443:443 -v hmn-acme:/acme-cache humanymous -acme-domain=demo.example.com
ENTRYPOINT ["/app/server", "-addr", ":443", "-web", "/app/web", "-acme-cache", "/acme-cache"]
