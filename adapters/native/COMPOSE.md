# Native ABI Compose home

Scaffold for a future owned Linux ABI pilot image. Header provenance and C/C++
layout remain hosted pending cases per `pilot-contract.json`.

```bash
docker compose -p humanymous-native-abi -f adapters/native/compose.yaml --env-file adapters/native/.env --profile abi-pilot config
```
