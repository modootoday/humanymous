# G2 collector product-binding Cap topology image.
# Cap=0 Cap-ready candidate. Cap=1 after Cap realloc + live TRANSFER only.
# Component Cap green != pass_product_clean_room. Detection freeze. CARGO_BUILD_JOBS=1.
# Cap pilot: emit honesty measure receipt scaffold; refuse product invent.
# network_mode=none; never claims hosted product / Cap free invent.
FROM alpine:3.20
COPY deployments/lab/collector/g2-collector-product-binding-contract.json /opt/lab/g2-collector-product-binding-contract.json
COPY deployments/lab/collector/g2-collector-product-binding-measure-receipt.scaffold.json /opt/lab/measure-receipt.json
COPY deployments/lab/collector/g2-collector-product-binding-cap-pilot.sh /opt/lab/cap-pilot.sh
RUN chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
