# humanymous — build & test targets (SoT/plan driven).
# Windows note: run under Git Bash; uses forward slashes.

GO       ?= go
WASM_OUT := web/detector.wasm
SRV_OUT  := bin/server.exe
RPT_OUT  := bin/report.exe
ADDR     ?= :8443

IMAGE   ?= humanymous/core:local
COMPOSE ?= docker compose -f deployments/compose.yaml

.PHONY: all wasm wasmexec server report build test race e2e-deps e2e report-html run clean fmt vet \
        docker up attack swarm gate-e2e down logs

all: build

## wasm: build the browser detection engine
wasm:
	GOOS=js GOARCH=wasm $(GO) build -o $(WASM_OUT) ./cmd/wasm/

## wasmexec: copy Go's wasm_exec.js into web/js
wasmexec:
	@cp "$$($(GO) env GOROOT)/lib/wasm/wasm_exec.js" web/js/wasm_exec.js 2>/dev/null \
	 || cp "$$($(GO) env GOROOT)/misc/wasm/wasm_exec.js" web/js/wasm_exec.js

## server: build the Blue backend binary
server:
	$(GO) build -o $(SRV_OUT) ./cmd/server/

## report: build the report generator
report:
	$(GO) build -o $(RPT_OUT) ./cmd/report/

build: wasm server report

## test: unit tests
test:
	$(GO) test ./internal/...

## race: unit tests with the race detector
race:
	$(GO) test -race ./internal/...

fmt:
	$(GO) fmt ./...
vet:
	$(GO) vet ./internal/... ./cmd/server/... ./cmd/report/...

## run: start the Blue server (self-signed TLS)
run: server wasm
	$(SRV_OUT) -addr $(ADDR) -web web

## e2e-deps: install the Red-team harness deps (playwright-core; uses installed Edge)
e2e-deps:
	cd test && npm install --no-audit --no-fund

## e2e: run the Red vs Blue harness (server must be running separately, or use scripts/e2e.sh)
e2e:
	node test/e2e/runner.mjs

## report-html: aggregate results.json into docs/report.html
report-html: report
	$(RPT_OUT) -in test/e2e/results.json -out docs/report.html

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

## gate-e2e: run the Gate proxy-layer conformance (34 checks)
gate-e2e:
	$(COMPOSE) run --rm gate-e2e

## logs: follow the detection-stack logs
logs:
	$(COMPOSE) logs -f core gate

## down: tear down the whole stack (containers + networks + volumes)
down:
	$(COMPOSE) down -v

clean:
	rm -f $(SRV_OUT) $(RPT_OUT) $(WASM_OUT) test/e2e/results.json
