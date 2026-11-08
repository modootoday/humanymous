# Python adapter Compose home

Scaffold aligned with nginx/apache network-none hardening. Execution stays
`not_run` until an owned Dockerfile/image and Root allocation exist. Prefer
`cargo test -p humanymous-python-adapter --features host-pilot` for current
component pilots (S6 `python-ruby-pilots` packet).

```bash
docker compose -p humanymous-python-pilot -f adapters/python/compose.yaml --env-file adapters/python/.env --profile host-pilot config
```
