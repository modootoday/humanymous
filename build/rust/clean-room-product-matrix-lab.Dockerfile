# Clean-room product-matrix Cap topology image.
# Cap=0 Cap-ready candidate. Cap=1 after Cap realloc + live TRANSFER only.
# Product-matrix require-full feeder. Never invents pass_product_clean_room.
# Distinct from spent clean-room-product Cap topology twin.
# Detection freeze. CARGO_BUILD_JOBS=1.
FROM alpine:3.20
COPY deployments/lab/product/clean-room-product-matrix-contract.json /opt/lab/clean-room-product-matrix-contract.json
COPY deployments/lab/product/clean-room-product-matrix-measure-receipt.scaffold.json /opt/lab/measure-receipt.json
COPY deployments/lab/product/clean-room-product-matrix-cap-pilot.sh /opt/lab/cap-pilot.sh
RUN chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
