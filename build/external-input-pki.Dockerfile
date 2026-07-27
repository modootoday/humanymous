FROM alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce

RUN apk add --no-cache openssl

COPY deployments/external-input/pki-entrypoint.sh /usr/local/bin/pki-entrypoint
RUN chmod 0555 /usr/local/bin/pki-entrypoint

ENTRYPOINT ["/usr/local/bin/pki-entrypoint"]
