# Docker-only external-input lab Core. The detector/server sources and the four
# fixture assets are built into one image so the signed runtime digest freezes
# the page under measurement; no source-tree bind mount can replace it at run time.
FROM --platform=$BUILDPLATFORM golang:1.25-alpine@sha256:56961d79ea8129efddcc0b8643fd8a5416b4e6228cfd477e3fd61deb2672c587 AS builder
ARG TARGETOS
ARG TARGETARCH
WORKDIR /src

COPY go.mod go.sum ./
RUN go mod download
COPY . .

RUN GOOS=js GOARCH=wasm go build -o web/detector.wasm ./cmd/wasm/ \
 && cp "$(go env GOROOT)/lib/wasm/wasm_exec.js" web/js/wasm_exec.js
ARG VERSION=dev
RUN CGO_ENABLED=0 GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH} \
    go build -trimpath -ldflags="-s -w -X main.version=${VERSION}" \
    -o /out/server ./cmd/server/

FROM gcr.io/distroless/static-debian12:nonroot@sha256:f5b485ea962d9bd1186b2f6b3a061191539b905b82ec395de78cbfae51f20e35
WORKDIR /app

COPY --from=builder /out/server /app/server
COPY --from=builder /src/web /app/web
COPY --from=builder /src/test/externalinput/fixture.html /app/web/external-input.html
COPY --from=builder /src/test/externalinput/fixture.css /app/web/external-input.css
COPY --from=builder /src/test/externalinput/fixture.mjs /app/web/external-input.mjs
COPY --from=builder /src/test/externalinput/ime-fixture.mjs /app/web/external-input-ime.mjs
COPY --from=builder /src/LICENSE /src/NOTICE /src/THIRD_PARTY_LICENSES.md /app/

EXPOSE 8443
USER 65532:65532
ENTRYPOINT ["/app/server"]
CMD ["-addr", ":8443", "-web", "/app/web"]
