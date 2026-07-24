# humanymous Gate — the reverse-proxy security layer (SoT-19..28).
# Self-contained: the admin console is embedded and the detection bundle is
# injected inline, so no web/ assets are needed. Build context = repo root;
# the adjacent gate.Dockerfile.dockerignore re-includes cmd/gate.
# Cross-compile to the target arch on the build platform (SoT-31 R6) so multi-arch
# release builds compile natively instead of emulating under QEMU. Static, CGO off.
FROM --platform=$BUILDPLATFORM golang:1.25 AS builder
ARG TARGETOS
ARG TARGETARCH
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
ARG VERSION=dev
RUN CGO_ENABLED=0 GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH} go build -trimpath -ldflags="-s -w -X main.version=${VERSION}" -o /out/gate ./cmd/gate/
# Pre-create the writable mount points owned by the non-root runtime uid. distroless
# has no shell/mkdir, so we make them here and COPY --chown below. A fresh named volume
# mounted at these paths inherits this ownership, so -keystore /data, -audit-wal /data,
# and -acme-cache /acme-cache are writable under `read_only` + USER 65532 (otherwise a
# root-owned volume denies the non-root process and Gate cannot boot).
RUN mkdir -p /out/data /out/acme-cache

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /out/gate /app/gate
# Ship licence + third-party notices with the image (audit: BSD-3/MIT redistribution).
COPY --from=builder /src/LICENSE /src/NOTICE /src/THIRD_PARTY_LICENSES.md /app/
# Writable data + ACME cache dirs, owned by the runtime uid (see builder note above).
COPY --from=builder --chown=65532:65532 /out/data /data
COPY --from=builder --chown=65532:65532 /out/acme-cache /acme-cache
EXPOSE 8444 8445
USER 65532:65532
# Flags (edge/admin addr, upstream, tokens) are supplied by docker-compose.
ENTRYPOINT ["/app/gate"]
