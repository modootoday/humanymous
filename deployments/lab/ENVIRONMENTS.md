# Humanymous usage-environment Compose matrix

Thin index only. Each environment keeps its own directory + `compose.yaml`
(or includes an existing canonical file). Writing Compose is **not** product e2e
success (`test/AGENTS.md`, rule `60-e2e-docker-only`, SoT
`container-test-environments`).

## Conventions followed

| Rule | Where |
| --- | --- |
| Per-adapter Compose home | `adapters/nginx/compose.yaml` (+ README pilot commands); S6 packet `nginx-compose-pilot` |
| Lab layout | `.spec/sot/container-test-environments.sot.md` → `deployments/lab/` + `topology/`, `authority/`, `clients/`, `adapters/`, `scenarios/`, `providers/` |
| Oracle e2e modular include | `deployments/compose.yaml` → `deployments/compose/*.yaml` |
| Docker-only product gate | `test/AGENTS.md`, `.agents/rules/60-e2e-docker-only.md` |
| Engine affinity | `.agents/rules/96-docker-engine-affinity.md` — Windows Desktop `desktop-linux` |
| Defense-only / internal nets | Lab `internal: true`; no third-party probing |
| A11y | No motor-gated challenge environments invented here |

**Project prefix:** always `humanymous-<family>-*` (e.g. `humanymous-lab-go-product`,
`humanymous-nginx-native`). Never reuse `humanymous-pages-preview`.

## Adapter component homes (`adapters/<name>/`)

| Environment | Compose | Project prefix | Profile | Ports | Healthcheck | Product gate fed |
| --- | --- | --- | --- | --- | --- | --- |
| nginx native module | `adapters/nginx/compose.yaml` | `humanymous-nginx-native` | (default) | none (`network_mode: none`) | process exit / pilot report | adapter-conformance; **not** rust-product-lab |
| apache HTTPD module | `adapters/apache/compose.yaml` | `humanymous-apache-native` | (default) | none | process exit / pilot report | adapter-conformance |
| node Express/Hono host | `adapters/node/compose.yaml` | `humanymous-node-pilot` | (default) | none | process exit / host_process | rust-authority-node-ipc component |
| go fixture export | `adapters/go/compose.yaml` | `humanymous-go-fixture` | `fixture-export` | none | build/export only | input to go-product lab |
| python host scaffold | `adapters/python/compose.yaml` | `humanymous-python-pilot` | `host-pilot` | none | image-required (`not_run`) | future adapter-conformance |
| ruby host scaffold | `adapters/ruby/compose.yaml` | `humanymous-ruby-pilot` | `host-pilot` | none | image-required (`not_run`) | future adapter-conformance |
| dotnet host scaffold | `adapters/dotnet/compose.yaml` | `humanymous-dotnet-pilot` | `host-pilot` | none | image-required (`not_run`); Windows tuples separate | future adapter-conformance |
| native ABI scaffold | `adapters/native/compose.yaml` | `humanymous-native-abi` | `abi-pilot` | none | image-required (`not_run`) | future ABI component |

Lab mirrors (include only): `deployments/lab/adapters/<name>/`.

## Lab scenario compositions (`deployments/lab/scenarios/`)

| Environment | Compose | Project prefix | Profile(s) | Ports | Healthcheck | Product gate fed |
| --- | --- | --- | --- | --- | --- | --- |
| topology fixture | `scenarios/topology-fixture/` → `compose.yaml` | `humanymous-lab-topology-fixture` | `controls` | none published | fixture health CMD | topology-fixture only |
| rust controls / pilots | `scenarios/rust-controls/` | `humanymous-lab-rust-controls` | `controls` | none | lab-runner fixture health | rust-product-lab prep |
| authority + Node IPC | `scenarios/authority-node-ipc/` | `humanymous-lab-authority-node` | `authority` | none (`network_mode: none`) | container exit | rust-authority-node-ipc |
| go Gate 8-case | `scenarios/go-product/` | `humanymous-lab-go-product` | `go-product` | internal bridges only | lab-runner / case ledger | go bounded product (**not** full e2e alone) |
| browser B product | `scenarios/browser-product/` | `humanymous-lab-browser-product` | `browser-product` | internal | lab-runner browser drivers | browser product (when allocated) |
| collector WASM component | `scenarios/collector-component/` | `humanymous-lab-collector` | `collector` | none | component run | collector component |
| custody | `scenarios/custody/` | `humanymous-lab-custody` | (file-defined) | none | file-defined | audit/custody component |
| runtime | `scenarios/runtime/` | `humanymous-lab-runtime` | (file-defined) | none | file-defined | runtime component |
| witness overlay | `scenarios/witness/` | `humanymous-lab-witness` | (file-defined) | none | file-defined | witness composition |
| oracle detector-vs-bots | `scenarios/oracle-detector-bots/` → `deployments/compose.yaml` | `humanymous-oracle-*` | `swarm` etc. | host `:8443`/`:8444`/`:8445` | make e2e / scripts | **Go oracle** e2e (not Rust full-product) |
| pages/docs preview | `scenarios/pages-docs-preview/README.md` only | **do not** use `humanymous-pages-preview` | — | — | — | docs/Pages; not product Gate |

Existing single-file overlays retained (also listed above via scenarios):
`compose.yaml`, `compose.rust*.yaml`, `compose.authority.yaml`,
`compose.collector.yaml`, `compose.custody.yaml`, `compose.runtime.yaml`.

## OS / arch notes

| Surface | Notes |
| --- | --- |
| Local Windows lab | Docker Desktop Linux engine only (`desktop-linux` / npipe); rule 96 |
| Adapter native modules | Linux amd64 component images (nginx/apache Dockerfiles) |
| .NET | Windows host pilots primary in contract; Linux Compose is scaffold |
| Cloudflare Containers / Tunnel | SoT profiles exist; **not** native Compose import targets here |
| Pages preview | Separate tooling process/container; avoid name collision |

## How to validate (config only)

From repo root, after Desktop affinity preflight:

```text
docker.exe --context desktop-linux compose -p humanymous-lab-<id> -f <compose> --env-file <.env> config
```

Full `up` / red-blue product e2e requires separate Root allocation and is out of
scope for Compose authorship alone.
