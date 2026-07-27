FROM node:22-bookworm-slim@sha256:6c74791e557ce11fc957704f6d4fe134a7bc8d6f5ca4403205b2966bd488f6b3

RUN apt-get update \
 && apt-get install -y --no-install-recommends kmod util-linux ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY test/externalinput/vusb/ /app/test/externalinput/vusb/
COPY test/externalinput/usb-broker/protocol.mjs /app/test/externalinput/usb-broker/protocol.mjs
COPY test/externalinput/*.mjs /app/test/externalinput/
COPY deployments/external-input/trust/catalog-ed25519.pub /app/trust/catalog-ed25519.pub
COPY scripts/assert-external-input-vusb.mjs /app/scripts/assert-external-input-vusb.mjs
RUN chmod 0555 /app/test/externalinput/vusb/lifecycle.sh \
 && mkdir -p /receipts /run/lock \
 && chown -R node:node /app

ENTRYPOINT ["node"]
CMD ["--help"]
