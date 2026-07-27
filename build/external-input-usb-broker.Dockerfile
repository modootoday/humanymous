FROM node:22-bookworm-slim@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3

WORKDIR /app
COPY test/externalinput/usb-broker/ /app/usb-broker/
RUN mkdir -p /run/external-input \
 && chown -R node:node /run/external-input

USER node
ENTRYPOINT ["node", "/app/usb-broker/main.mjs"]
