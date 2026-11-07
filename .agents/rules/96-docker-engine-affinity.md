# Docker engine affinity and cleanup scope

## Local Windows engine

Local Windows Docker work uses the Windows Docker Desktop Linux engine. Invoke
the resolved Windows `docker.exe` with explicit `--context desktop-linux` for
each direct Docker CLI operation. If it is absent from PATH, resolve
`C:/Program Files/Docker/Docker/resources/bin/docker.exe`. If that executable or
the required context is unavailable, stop and report the missing prerequisite.
Never automatically substitute `wsl docker`, a Unix socket or another context.
Do not change the user's default context to make a command work.

Before an allocated operation, record the executable path, client/server
versions, context name and endpoint, server engine ID and OSType. Require the
`desktop-linux` endpoint `npipe:////./pipe/dockerDesktopLinuxEngine`, an OSType
of `linux`, and the same engine ID throughout the operation. Context names and
version strings alone do not prove engine identity. An unexpected endpoint,
missing identity or changed ID stops the operation before workload mutation.
Resolve conflicting Docker environment overrides rather than silently switching
engines; keep overrides scoped to the allocated child process.

Root observed two distinct engines during the incident: Windows Docker Desktop
29.9.2 on `desktop-linux`, and a WSL-native 29.1.3 engine on a Unix endpoint with
a different engine ID. These are historical observations, not required future
versions or evidence that either engine is currently healthy. Root owns the
source-bound incident and measurement evidence.

## WSL, act and hosted runners

A WSL CLI may be used only when separately allocated and its actual server ID
is verified to match the Windows Docker Desktop engine. A standalone WSL daemon
is a distinct engine and is never the default local fallback. Record the CLI
venue and endpoint as well as the matched server ID.

An SDK client such as `act` must not be assumed to honor the Docker CLI context.
For Windows `act`, explicitly supply the Docker Desktop named-pipe endpoint in
the child process's `DOCKER_HOST` and verify the engine ID used by that client
against the Windows Docker Desktop preflight. A successful Docker CLI context
check alone does not establish where `act` created its containers. If actual
client affinity cannot be established, do not start the workload or claim parity.

Genuine hosted Linux runners use their own allocated Docker engine. Record their
endpoint, engine ID, versions, image digest and actual runner provenance
separately. Local Windows, local WSL, local `act` and GitHub-hosted results remain
distinct. Never spoof GitHub environment variables to bypass hosted-only guards
or label a local execution as hosted evidence.

## Image cleanup authorization

Run `image prune -a` only after an explicit user request for that image cleanup,
against the verified intended engine. Image-cleanup authorization does not cover
container removal, volume removal, build-cache pruning or `system prune`.
Preserve containers, volumes and build cache unless separately authorized. Do
not remove containers to make images eligible for pruning or add broader cleanup
flags. Record the exact engine and cleanup scope before and after the operation.

Engine affinity is a prerequisite, not an execution allocation. Existing quiet
windows, lane ownership, resource budgets and product-e2e gates still apply.
