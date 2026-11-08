# Ruby adapter Compose home

Scaffold only. Component host pilots remain cargo `host-pilot` until a Docker
image recipe is owned and allocated.

```bash
docker compose -p humanymous-ruby-pilot -f adapters/ruby/compose.yaml --env-file adapters/ruby/.env --profile host-pilot config
```
