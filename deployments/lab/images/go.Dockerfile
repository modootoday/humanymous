# The only external bytes are the independently verified untouched fixture export.
FROM product_image
COPY --from=go_fixture /go-httpbin /usr/local/bin/go-httpbin
ENTRYPOINT ["/usr/local/bin/humanymous-runtime-pilot", "--go-backend"]
