# Bots image — the automation catalog (SoT-04) + the Go protocol attack binaries
# (cmd/redteam uTLS/RIT, cmd/tlsparrot). Also carries the Gate binary so the
# self-contained Gate conformance e2e can run on loopback.
#
# LOCAL TARGET ONLY. In docker-compose this container is attached ONLY to the
# internal `lab` network (no internet route), so it can physically reach nothing
# but the detector — the defensive mandate enforced at the network layer, not just
# by convention.
#
# Build context = repo root; the adjacent bots.Dockerfile.dockerignore re-includes
# cmd/redteam, cmd/tlsparrot, cmd/gate and test/.

# ---- Go attack + gate binaries ----------------------------------------
FROM golang:1.25 AS gobuild
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
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

COPY test /app/test
COPY scripts/assert-*.mjs /app/scripts/
COPY deployments/bots/ /app/
# Seed lives outside the runtime mount point so an empty named volume cannot hide keys.
COPY --from=gobuild /src/deployments/patissuers /app/demo-keys-seed/patissuers
COPY --from=gobuild /src/deployments/webauthncreds /app/demo-keys-seed/webauthncreds
RUN sed -i 's/\r$//' /app/*.sh && chmod +x /app/*.sh \
 && cd /app/test && (npm ci --no-audit --no-fund || npm install --no-audit --no-fund)

ENV HM_REDTEAM_BIN=/app/bin/redteam \
    HM_TLSPARROT_BIN=/app/bin/tlsparrot \
    HM_BROWSER_CHANNEL=chromium \
    HM_LAUNCH_ARGS=--no-sandbox,--disable-dev-shm-usage \
    NODE_TLS_REJECT_UNAUTHORIZED=0

# Default job: run the automation catalog against the detector engine. Overridable
# in compose (e.g. the gate-e2e service runs run-gate-e2e.sh instead).
ENTRYPOINT ["/app/run-attack.sh"]
