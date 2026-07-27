# Canonical SoT-41 display/browser images. These targets deliberately use the
# distribution browser packages rather than WebDriver/CDP-oriented images.
FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS display

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      nodejs \
      tigervnc-scraping-server \
      tigervnc-tools \
      x11-utils \
      xauth \
      xserver-xorg-core \
      xserver-xorg-input-libinput \
      xserver-xorg-video-dummy \
 && rm -rf \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*

RUN groupadd --gid 12001 externalinput \
 && usermod -a -G externalinput root \
 && install -d -o root -g externalinput -m 2770 /run/external-evidence
COPY deployments/external-input/display-entrypoint.sh /usr/local/bin/external-display
COPY test/externalinput/vusb/seat-observer.mjs /opt/external-input/seat-observer.mjs
COPY test/externalinput/contracts.mjs /opt/external-input/contracts.mjs
COPY test/externalinput/runtime-purity.mjs /opt/external-input/runtime-purity.mjs
COPY test/externalinput/runtime-display-probe.mjs /opt/external-input/runtime-display-probe.mjs
RUN chmod 0555 /usr/local/bin/external-display
ENTRYPOINT ["/usr/local/bin/external-display"]

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS browser-common

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates \
      fonts-dejavu-core \
      libnss3-tools \
      nodejs \
      openssl \
      x11-xkb-utils \
 && rm -rf \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*

RUN groupadd --gid 12001 externalinput \
 && useradd --create-home --uid 10001 --shell /usr/sbin/nologin extbrowser \
 && usermod -a -G externalinput extbrowser \
 && install -d -o extbrowser -g extbrowser /run/browser-profile \
 && install -d -o extbrowser -g externalinput -m 2770 /run/dom-observer \
 && install -d -o extbrowser -g externalinput -m 2770 /run/external-evidence
COPY deployments/external-input/browser-entrypoint.sh /usr/local/bin/external-browser
COPY test/externalinput/contracts.mjs /opt/external-input/contracts.mjs
COPY test/externalinput/runtime-purity.mjs /opt/external-input/runtime-purity.mjs
COPY test/externalinput/runtime-browser-probe.mjs /opt/external-input/runtime-browser-probe.mjs
RUN chmod 0555 /usr/local/bin/external-browser \
 && chown -R extbrowser:extbrowser /run/browser-profile \
 && chown extbrowser:externalinput /run/dom-observer
ENTRYPOINT ["/usr/local/bin/external-browser"]

FROM browser-common AS browser-chromium-base
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends chromium chromium-sandbox \
 && rm -rf \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*

FROM browser-chromium-base AS browser-chromium
USER extbrowser

FROM browser-chromium-base AS browser-chromium-ime
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      fonts-noto-cjk \
      ibus \
      ibus-anthy \
      ibus-gtk3 \
      ibus-hangul \
      ibus-libpinyin \
      libglib2.0-bin \
      procps \
 && rm -rf \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*
USER extbrowser

FROM browser-chromium-base AS browser-chromium-dom
COPY test/externalinput/dom-observer/ /opt/external-input/dom-observer/
RUN install -d -m 0755 /etc/chromium/native-messaging-hosts \
 && install -o root -g root -m 0444 \
      /opt/external-input/dom-observer/native-host-manifest.json \
      /etc/chromium/native-messaging-hosts/org.humanymous.external_input_dom_observer.json \
 && chmod 0555 /opt/external-input/dom-observer/native-host.mjs \
 && chown -R root:root /opt/external-input/dom-observer
USER extbrowser

FROM browser-common AS browser-firefox-base
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends firefox-esr zip \
 && rm -rf \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*

FROM browser-firefox-base AS browser-firefox
USER extbrowser

FROM browser-firefox-base AS browser-firefox-ime
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      fonts-noto-cjk \
      ibus \
      ibus-anthy \
      ibus-gtk3 \
      ibus-hangul \
      ibus-libpinyin \
      libglib2.0-bin \
      procps \
 && rm -rf \
      /var/cache/apt/* \
      /var/lib/apt/lists/* \
      /usr/share/doc/* \
      /usr/share/man/*
USER extbrowser

# The disposable Firefox ESR DOM-observer target uses the distribution
# extension directory and a lab-only enterprise policy. It is intentionally
# distinct from the extension-free Firefox baseline image.
FROM browser-firefox-base AS browser-firefox-dom
COPY test/externalinput/dom-observer/ /opt/external-input/dom-observer/
COPY test/externalinput/dom-observer/firefox-native-host-manifest.json \
  /usr/lib/mozilla/native-messaging-hosts/org.humanymous.external_input_dom_observer.json
COPY deployments/external-input/firefox-policies.json \
  /usr/lib/firefox-esr/distribution/policies.json
RUN install -d -m 0755 \
      /tmp/external-input-firefox-extension \
      /usr/lib/firefox-esr/distribution/extensions \
 && install -m 0444 \
      /opt/external-input/dom-observer/firefox-manifest.json \
      /tmp/external-input-firefox-extension/manifest.json \
 && install -m 0444 \
      /opt/external-input/dom-observer/extension/content-script.js \
      /opt/external-input/dom-observer/extension/protocol.mjs \
      /opt/external-input/dom-observer/extension/service-worker.mjs \
      /tmp/external-input-firefox-extension/ \
 && cd /tmp/external-input-firefox-extension \
 && zip -X -q \
      /usr/lib/firefox-esr/distribution/extensions/external-input-dom@humanymous.invalid.xpi \
      manifest.json content-script.js protocol.mjs service-worker.mjs \
 && cd / \
 && rm -rf /tmp/external-input-firefox-extension \
 && find /opt/external-input/dom-observer \
         /usr/lib/firefox-esr/distribution \
         /usr/lib/mozilla/native-messaging-hosts \
      -type f -exec chmod 0444 {} + \
 && chmod 0555 /opt/external-input/dom-observer/native-host.mjs \
 && chown -R root:root \
      /opt/external-input/dom-observer \
      /usr/lib/firefox-esr/distribution \
      /usr/lib/mozilla/native-messaging-hosts
USER extbrowser
