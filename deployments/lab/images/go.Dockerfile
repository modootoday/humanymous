# The only external bytes are the independently verified untouched fixture export.
ARG HM_PRODUCT_IMAGE
FROM ${HM_PRODUCT_IMAGE}
COPY --from=go_fixture /go-httpbin /usr/local/bin/go-httpbin
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot", "--go-backend"]
