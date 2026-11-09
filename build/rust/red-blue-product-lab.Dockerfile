# Red-blue product Cap topology image.
# Cap=0 Cap-ready candidate. Cap=1 after Cap realloc + live TRANSFER only.
# Product/red-blue feeder (require-full). Never invents pass_product_clean_room.
# Detection freeze. CARGO_BUILD_JOBS=1.
FROM alpine:3.20
COPY deployments/lab/product/red-blue-product-contract.json /opt/lab/red-blue-product-contract.json
COPY deployments/lab/product/red-blue-product-measure-receipt.scaffold.json /opt/lab/measure-receipt.json
COPY deployments/lab/product/red-blue-product-cap-pilot.sh /opt/lab/cap-pilot.sh
RUN chmod 0555 /opt/lab/cap-pilot.sh
USER 65532:65532
ENTRYPOINT ["/bin/sh", "/opt/lab/cap-pilot.sh"]
