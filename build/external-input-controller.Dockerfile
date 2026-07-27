FROM node:22-bookworm-slim@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3

WORKDIR /app
COPY test/externalinput /app/test/externalinput
COPY test/e2e/external-input-runner.mjs /app/test/e2e/external-input-runner.mjs
COPY test/redteam/external_input_*.mjs /app/test/redteam/
COPY deployments/bots/external-input-run.sh /app/external-input-run.sh
COPY scripts/assert-external-input.mjs /app/scripts/assert-external-input.mjs
COPY scripts/external-input-readiness.mjs /app/scripts/external-input-readiness.mjs

RUN groupadd --gid 12001 externalinput \
 && usermod -a -G externalinput node \
 && chmod 0555 /app/external-input-run.sh \
 && mkdir -p /artifacts/external-input \
 && chown -R node:node /artifacts

USER node
ENTRYPOINT ["/app/external-input-run.sh"]
