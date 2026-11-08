# Clean-room product Cap topology image.
# Cap=0 Cap-ready candidate. Cap=1 after Cap realloc + live TRANSFER only.
# G2+G5 Cap=1 holds bound ≠ pass_product_clean_room. Detection freeze. CARGO_BUILD_JOBS=1.
# Cap pilot: emit honesty measure receipt scaffold; refuse product invent.
# network_mode=none; never claims BrowserReady / N06 join / Cap free invent.
FROM alpine:3.20
COPY deployments/lab/product/clean-room-product-contract.json /opt/lab/clean-room-product-contract.json
COPY deployments/lab/product/clean-room-product-measure-receipt.scaffold.json /opt/lab/measure-receipt.json
COPY deployments/lab/product/clean-room-product-cap-pilot.sh /opt/lab/cap-pilot.sh
RUN chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
