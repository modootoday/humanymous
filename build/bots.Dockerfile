# Bots image — the automation catalog (SoT-04) + the Go protocol attack binaries
# (cmd/redteam uTLS/RIT, cmd/tlsparrot). Also carries the Sentinel binary so the
# self-contained Sentinel conformance e2e can run on loopback.
#
# LOCAL TARGET ONLY. In docker-compose this container is attached ONLY to the
# internal `lab` network (no internet route), so it can physically reach nothing
# but the detector — the defensive mandate enforced at the network layer, not just
# by convention.
#
# Build context = repo root; the adjacent bots.Dockerfile.dockerignore re-includes
# cmd/redteam, cmd/tlsparrot, cmd/sentinel and test/.

# ---- Go attack + sentinel binaries ----------------------------------------
FROM golang:1.25 AS gobuild
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -trimpath -o /out/redteam   ./cmd/redteam/ \
 && CGO_ENABLED=0 GOOS=linux go build -trimpath -o /out/tlsparrot ./cmd/tlsparrot/ \
 && CGO_ENABLED=0 GOOS=linux go build -trimpath -o /out/sentinel  ./cmd/sentinel/

# ---- Bots runtime: Node + Playwright browsers (Chromium + Firefox) ---------
FROM mcr.microsoft.com/playwright:v1.61.1-noble
WORKDIR /app

COPY --from=gobuild /out/redteam   /app/bin/redteam
COPY --from=gobuild /out/tlsparrot /app/bin/tlsparrot
COPY --from=gobuild /out/sentinel  /app/bin/sentinel

COPY test /app/test
COPY deployments/bots/ /app/
RUN sed -i 's/\r$//' /app/*.sh && chmod +x /app/*.sh \
 && cd /app/test && (npm ci --no-audit --no-fund || npm install --no-audit --no-fund)

ENV HM_REDTEAM_BIN=/app/bin/redteam \
    HM_TLSPARROT_BIN=/app/bin/tlsparrot \
    HM_BROWSER_CHANNEL=chromium \
    HM_LAUNCH_ARGS=--no-sandbox,--disable-dev-shm-usage \
    NODE_TLS_REJECT_UNAUTHORIZED=0

# Default job: run the automation catalog against the detector engine. Overridable
# in compose (e.g. the sentinel-e2e service runs run-sentinel-e2e.sh instead).
ENTRYPOINT ["/app/run-attack.sh"]
