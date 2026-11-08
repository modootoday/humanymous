# Node adapter Compose home

Mirrors `adapters/nginx/compose.yaml` for the existing `Dockerfile.pilot`.
Host-process / authority lifecycle only — not isolated product e2e.

```bash
docker compose -p humanymous-node-pilot -f adapters/node/compose.yaml --env-file adapters/node/.env config
docker compose -p humanymous-node-pilot -f adapters/node/compose.yaml build
docker compose -p humanymous-node-pilot -f adapters/node/compose.yaml run --rm host-pilot
docker compose -p humanymous-node-pilot -f adapters/node/compose.yaml down --remove-orphans
```

Project prefix `humanymous-node-*` must not collide with `humanymous-pages-preview`.
