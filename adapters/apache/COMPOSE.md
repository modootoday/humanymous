# Apache adapter Compose home

Convention match: `adapters/nginx/compose.yaml` + S6 packet `nginx-compose-pilot`
(lane `adapter-apache`). Component pilot only — `fullProductSuccess` stays false.

```bash
# Windows Docker Desktop affinity (rule 96):
#   "C:/Program Files/Docker/Docker/resources/bin/docker.exe" --context desktop-linux …
docker compose -p humanymous-apache-native -f adapters/apache/compose.yaml --env-file adapters/apache/.env config
docker compose -p humanymous-apache-native -f adapters/apache/compose.yaml build
docker compose -p humanymous-apache-native -f adapters/apache/compose.yaml run --rm native-pilot
docker compose -p humanymous-apache-native -f adapters/apache/compose.yaml down --remove-orphans
```

Lab matrix: `deployments/lab/ENVIRONMENTS.md`.
