# humanymous — build & test targets (SoT/plan driven).
# Windows note: run under Git Bash; uses forward slashes.

GO       ?= go
WASM_OUT := web/detector.wasm
SRV_OUT  := bin/server.exe
GATE_OUT := bin/gate.exe
RPT_OUT  := bin/report.exe
ADDR     ?= :8443

IMAGE   ?= humanymous/core:local
COMPOSE ?= docker compose -f deployments/compose.yaml

.PHONY: all wasm wasmexec server gate report build test race e2e-deps e2e e2e-docker e2e-quick report-html run clean fmt vet \
        docker up attack swarm gate-e2e down logs changelog-unreleased release-notes docs-assets docs-frames docs-anim

all: build

## wasm: build the browser detection engine
wasm:
	GOOS=js GOARCH=wasm $(GO) build -o $(WASM_OUT) ./cmd/wasm/

## wasmexec: copy Go's wasm_exec.js into web/js
wasmexec:
	@cp "$$($(GO) env GOROOT)/lib/wasm/wasm_exec.js" web/js/wasm_exec.js 2>/dev/null \
	 || cp "$$($(GO) env GOROOT)/misc/wasm/wasm_exec.js" web/js/wasm_exec.js

## server: build the Blue backend binary (detection engine / Core)
server:
	$(GO) build -o $(SRV_OUT) ./cmd/server/

## gate: build the reverse-proxy enforcement layer binary
gate:
	$(GO) build -o $(GATE_OUT) ./cmd/gate/

## report: build the report generator
report:
	$(GO) build -o $(RPT_OUT) ./cmd/report/

build: wasm server gate report

## test: unit tests (all packages, matching README `go test ./...`)
test:
	$(GO) test ./...

## race: unit tests with the race detector (all packages)
race:
	$(GO) test -race ./...

fmt:
	$(GO) fmt ./...
vet:
	$(GO) vet ./internal/... ./cmd/server/... ./cmd/report/...

## docs-assets: (re)generate the docs OG images (brand WebP) + per-page llms.txt.
## Run after editing a page title/description (needs Node; sharp installs on first run).
docs-assets:
	cd scripts/docsgen && npm install --silent && node gen.mjs

## docs-frames: (re)render the OSX-framed surface screenshots (Ledger, /demo, Pass) as
## brand-framed WebP for the README + docs. Renders the REAL console/demo/pass HTML
## headlessly with stubbed data — needs Node, sharp (installed here), and the Playwright
## browser from the e2e harness (test/). Run after a surface's UI changes.
docs-frames:
	cd test && npm install --silent
	cd scripts/docsgen && npm install --silent && node shoot.mjs

## docs-anim: (re)render the animated docs sources (Ledger hero, quickstart cast) to
## animated WebP (github.com) + WebM/MP4/poster (docs <video>). Needs Node, sharp, the
## e2e Playwright browser, and ffmpeg on PATH. Run after editing scripts/docsgen/mocks/.
docs-anim:
	cd test && npm install --silent
	cd scripts/docsgen && npm install --silent && node anim.mjs

## changelog-unreleased: preview the notes for unreleased commits (since the last tag) to stdout.
changelog-unreleased:
	npx --yes git-cliff@latest --config cliff.toml --unreleased

## release-notes: print the release notes for the latest tag (what the GitHub Release body will be).
release-notes:
	npx --yes git-cliff@latest --config cliff.toml --latest --strip header

## run: start the Blue server (self-signed TLS)
run: server wasm
	$(SRV_OUT) -addr $(ADDR) -web web

## e2e-deps: install host-side Node deps for docs screenshots / local profile authoring
## (the authoritative attack catalog runs inside the bots Docker image — not via this target)
e2e-deps:
	cd test && npm install --no-audit --no-fund

## e2e / e2e-docker: authoritative end-to-end suite (Docker only).
## Runs attack catalog + assert + gate-e2e + swarm (+ overlays unless skipped).
## Host/loopback `node test/e2e/runner.mjs` is NOT completion authority (misses L5 topology).
e2e e2e-docker:
	bash scripts/e2e-docker.sh

## e2e-quick: Docker e2e without swarm/overlays (faster local iteration; still not unit-only)
e2e-quick:
	E2E_SKIP_SWARM=1 E2E_SKIP_OVERLAYS=1 bash scripts/e2e-docker.sh

## report-html: aggregate Docker attack artifacts (or local results) into docs/report.html
report-html: report
	@if [ -f deployments/artifacts/core-results.json ]; then \
	  $(RPT_OUT) -in deployments/artifacts/core-results.json -out docs/report.html; \
	else \
	  $(RPT_OUT) -in test/e2e/results.json -out docs/report.html; \
	fi

## docker: build the public demo image (detection engine only; build/core.Dockerfile)
docker:
	docker build -f build/core.Dockerfile -t $(IMAGE) .

# --- Local Docker detector-vs-bots stack (modular compose in deployments/) -----

## up: build + start the detection stack (engine + origin + Gate), host-visible
up:
	$(COMPOSE) up -d --build core origin gate

## attack: run the automation catalog (bots) against the engine (writes deployments/artifacts/)
attack:
	$(COMPOSE) run --rm bots

## swarm: run the multi-subnet correlation swarm (one fingerprint, three real subnets)
swarm:
	$(COMPOSE) --profile swarm up --build --abort-on-container-exit bot-swarm-a bot-swarm-b bot-swarm-c

## gate-e2e: run the Gate proxy-layer conformance (34 checks) inside Docker
gate-e2e:
	$(COMPOSE) run --rm gate-e2e

## logs: follow the detection-stack logs
logs:
	$(COMPOSE) logs -f core gate

## down: tear down the whole stack (containers + networks + volumes)
down:
	$(COMPOSE) down -v

## e2e-assert: re-check last Docker attack artifact inside the bots image (authoritative)
e2e-assert:
	$(COMPOSE) run --rm attack-assert

clean:
	rm -f $(SRV_OUT) $(RPT_OUT) $(WASM_OUT) test/e2e/results.json
