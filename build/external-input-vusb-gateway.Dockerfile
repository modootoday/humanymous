FROM node:22-bookworm-slim@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3

WORKDIR /app
COPY test/externalinput/vusb/ /app/test/externalinput/vusb/
COPY test/externalinput/usb-broker/ /app/test/externalinput/usb-broker/
COPY test/externalinput/input.mjs test/externalinput/errors.mjs /app/test/externalinput/

RUN mkdir -p /run/external-input \
 && chown -R node:node /run/external-input /app

USER node
ENTRYPOINT ["node", "/app/test/externalinput/vusb/gateway.mjs"]
