# Go adapter Compose home

`Dockerfile.fixture` builds an untouched upstream binary export. Gate composition
and the eight-case product profile live under `deployments/lab/` (see
`compose.rust.go.yaml` and scenario `go-product`).

```bash
# Fixture export only (profile fixture-export). Context = verified checkout.
docker compose -p humanymous-go-fixture -f adapters/go/compose.yaml --env-file adapters/go/.env --profile fixture-export config
docker compose -p humanymous-go-fixture -f adapters/go/compose.yaml --profile fixture-export build

# Product gate (owned lab runner; not this file alone):
# cargo run --locked -p humanymous-lab-runner -- run --evidence … --go-product …
```
