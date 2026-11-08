# .NET adapter Compose home

Linux container scaffold for a future source-bound image. Current pilot contract
emphasizes Windows managed host cases; do not claim those via this file.

```bash
docker compose -p humanymous-dotnet-pilot -f adapters/dotnet/compose.yaml --env-file adapters/dotnet/.env --profile host-pilot config
```
