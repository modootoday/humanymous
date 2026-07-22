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
RUN CGO_ENABLED=0 GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH} go build -trimpath -ldflags="-s -w" -o /out/gate ./cmd/gate/

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /out/gate /app/gate
# Ship licence + third-party notices with the image (audit: BSD-3/MIT redistribution).
COPY --from=builder /src/LICENSE /src/NOTICE /src/THIRD_PARTY_LICENSES.md /app/
EXPOSE 8444 8445
USER 65532:65532
# Flags (edge/admin addr, upstream, tokens) are supplied by docker-compose.
ENTRYPOINT ["/app/gate"]
